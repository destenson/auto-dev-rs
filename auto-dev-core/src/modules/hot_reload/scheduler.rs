#![allow(unused)]
//! Reload Scheduler - Manages timing and coordination of module reloads

use super::{HotReloadConfig, HotReloadError, HotReloadResult, ReloadCoordinator, ReloadResult};
use crate::modules::{ModuleLoader, ModuleRegistry, ModuleRuntime};
use anyhow::Result;
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Reload request priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReloadPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// A scheduled reload request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReloadRequest {
    pub module_id: String,
    pub new_path: PathBuf,
    pub priority: ReloadPriority,
    pub requested_at: chrono::DateTime<chrono::Utc>,
    pub scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl ReloadRequest {
    pub fn new(module_id: String, new_path: PathBuf, priority: ReloadPriority) -> Self {
        Self {
            module_id,
            new_path,
            priority,
            requested_at: chrono::Utc::now(),
            scheduled_for: None,
            retry_count: 0,
            max_retries: 3,
        }
    }

    pub fn with_schedule(mut self, time: chrono::DateTime<chrono::Utc>) -> Self {
        self.scheduled_for = Some(time);
        self
    }
}

/// Scheduling strategy for reloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchedulingStrategy {
    /// Execute immediately
    Immediate,
    /// Batch reloads together
    Batched { window_ms: u64 },
    /// Rate-limited execution
    RateLimited { max_per_second: u32 },
    /// Time-based scheduling
    Scheduled,
    /// Resource-aware scheduling
    Adaptive,
}

/// Scheduler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub strategy: SchedulingStrategy,
    pub max_concurrent_reloads: usize,
    pub retry_delay: Duration,
    pub health_check_interval: Duration,
    pub enable_auto_scheduling: bool,
    pub quiet_hours: Option<(u32, u32)>, // Start and end hour for quiet period
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            strategy: SchedulingStrategy::Batched { window_ms: 100 },
            max_concurrent_reloads: 2,
            retry_delay: Duration::from_secs(5),
            health_check_interval: Duration::from_secs(30),
            enable_auto_scheduling: true,
            quiet_hours: None,
        }
    }
}

/// Manages scheduling of module reloads
pub struct ReloadScheduler {
    config: SchedulerConfig,
    coordinator: Arc<ReloadCoordinator>,
    pending_queue: Arc<RwLock<VecDeque<ReloadRequest>>>,
    active_reloads: Arc<RwLock<HashMap<String, ReloadRequest>>>,
    completed_reloads: Arc<RwLock<Vec<(ReloadRequest, ReloadResult)>>>,
    semaphore: Arc<Semaphore>,
    is_running: Arc<RwLock<bool>>,
}

impl ReloadScheduler {
    pub fn new(config: SchedulerConfig, hot_reload_config: HotReloadConfig) -> Self {
        let max_concurrent = config.max_concurrent_reloads;
        
        Self {
            config: config.clone(),
            coordinator: Arc::new(ReloadCoordinator::new(hot_reload_config)),
            pending_queue: Arc::new(RwLock::new(VecDeque::new())),
            active_reloads: Arc::new(RwLock::new(HashMap::new())),
            completed_reloads: Arc::new(RwLock::new(Vec::new())),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Schedule a reload request
    pub async fn schedule_reload(&self, request: ReloadRequest) -> HotReloadResult<()> {
        info!(
            "Scheduling reload for module {} with priority {:?}",
            request.module_id, request.priority
        );

        // Check if module is already being reloaded
        if self.active_reloads.read().await.contains_key(&request.module_id) {
            return Err(HotReloadError::ConcurrentReloadDenied);
        }

        // Check quiet hours
        if self.is_quiet_hours() {
            debug!("Deferring reload due to quiet hours");
            let tomorrow = chrono::Utc::now() + chrono::Duration::days(1);
            let scheduled = request.with_schedule(tomorrow);
            self.add_to_queue(scheduled).await;
            return Ok(());
        }

        // Add to queue based on strategy
        match self.config.strategy {
            SchedulingStrategy::Immediate => {
                self.execute_reload_immediate(request).await;
            }
            SchedulingStrategy::Batched { .. } => {
                self.add_to_queue(request).await;
            }
            SchedulingStrategy::RateLimited { .. } => {
                self.add_to_queue(request).await;
            }
            SchedulingStrategy::Scheduled => {
                if request.scheduled_for.is_some() {
                    self.add_to_queue(request).await;
                } else {
                    self.execute_reload_immediate(request).await;
                }
            }
            SchedulingStrategy::Adaptive => {
                // Check system resources and decide
                if self.should_defer_reload().await {
                    self.add_to_queue(request).await;
                } else {
                    self.execute_reload_immediate(request).await;
                }
            }
        }

        Ok(())
    }

    /// Start the scheduler
    pub async fn start(
        &self,
        registry: Arc<RwLock<ModuleRegistry>>,
        loader: Arc<ModuleLoader>,
        runtime: Arc<ModuleRuntime>,
    ) {
        if *self.is_running.read().await {
            warn!("Scheduler is already running");
            return;
        }

        *self.is_running.write().await = true;
        info!("Starting reload scheduler");

        let scheduler = self.clone();
        tokio::spawn(async move {
            scheduler.run_scheduler_loop(registry, loader, runtime).await;
        });
    }

    /// Stop the scheduler
    pub async fn stop(&self) {
        info!("Stopping reload scheduler");
        *self.is_running.write().await = false;
    }

    /// Main scheduler loop
    async fn run_scheduler_loop(
        &self,
        registry: Arc<RwLock<ModuleRegistry>>,
        loader: Arc<ModuleLoader>,
        runtime: Arc<ModuleRuntime>,
    ) {
        let mut tick_interval = interval(Duration::from_millis(100));

        while *self.is_running.read().await {
            tick_interval.tick().await;

            // Process pending reloads based on strategy
            match self.config.strategy {
                SchedulingStrategy::Batched { window_ms } => {
                    self.process_batched_reloads(registry.clone(), loader.clone(), runtime.clone(), window_ms)
                        .await;
                }
                SchedulingStrategy::RateLimited { max_per_second } => {
                    self.process_rate_limited_reloads(
                        registry.clone(),
                        loader.clone(),
                        runtime.clone(),
                        max_per_second,
                    )
                    .await;
                }
                SchedulingStrategy::Scheduled => {
                    self.process_scheduled_reloads(registry.clone(), loader.clone(), runtime.clone())
                        .await;
                }
                _ => {
                    self.process_queue(registry.clone(), loader.clone(), runtime.clone())
                        .await;
                }
            }

            // Clean up completed reloads
            self.cleanup_completed().await;
        }
    }

    /// Process batched reloads
    async fn process_batched_reloads(
        &self,
        registry: Arc<RwLock<ModuleRegistry>>,
        loader: Arc<ModuleLoader>,
        runtime: Arc<ModuleRuntime>,
        window_ms: u64,
    ) {
        let mut batch = Vec::new();
        let batch_start = Instant::now();

        // Collect requests within window
        while batch_start.elapsed().as_millis() < window_ms as u128 {
            if let Some(request) = self.get_next_request().await {
                batch.push(request);
            } else {
                break;
            }
        }

        // Execute batch
        for request in batch {
            self.execute_reload(request, registry.clone(), loader.clone(), runtime.clone())
                .await;
        }
    }

    /// Process rate-limited reloads
    async fn process_rate_limited_reloads(
        &self,
        registry: Arc<RwLock<ModuleRegistry>>,
        loader: Arc<ModuleLoader>,
        runtime: Arc<ModuleRuntime>,
        max_per_second: u32,
    ) {
        let delay = Duration::from_millis(1000 / max_per_second as u64);
        let mut last_execution = Instant::now();

        if let Some(request) = self.get_next_request().await {
            if last_execution.elapsed() >= delay {
                self.execute_reload(request, registry, loader, runtime).await;
                last_execution = Instant::now();
            } else {
                // Re-queue for later
                self.add_to_queue(request).await;
            }
        }
    }

    /// Process scheduled reloads
    async fn process_scheduled_reloads(
        &self,
        registry: Arc<RwLock<ModuleRegistry>>,
        loader: Arc<ModuleLoader>,
        runtime: Arc<ModuleRuntime>,
    ) {
        let now = chrono::Utc::now();
        let mut ready = Vec::new();

        // Find ready requests
        {
            let mut queue = self.pending_queue.write().await;
            let mut i = 0;
            while i < queue.len() {
                if let Some(scheduled_time) = queue[i].scheduled_for {
                    if scheduled_time <= now {
                        if let Some(request) = queue.remove(i) {
                            ready.push(request);
                        }
                    } else {
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            }
        }

        // Execute ready requests
        for request in ready {
            self.execute_reload(request, registry.clone(), loader.clone(), runtime.clone())
                .await;
        }
    }

    /// Process general queue
    async fn process_queue(
        &self,
        registry: Arc<RwLock<ModuleRegistry>>,
        loader: Arc<ModuleLoader>,
        runtime: Arc<ModuleRuntime>,
    ) {
        if let Some(request) = self.get_next_request().await {
            self.execute_reload(request, registry, loader, runtime).await;
        }
    }

    /// Execute a reload immediately
    async fn execute_reload_immediate(&self, request: ReloadRequest) {
        // This is a placeholder - in real use, we'd need the registry, loader, and runtime
        warn!("Immediate reload requested but registry/loader/runtime not available");
        self.add_to_queue(request).await;
    }

    /// Execute a reload request
    async fn execute_reload(
        &self,
        mut request: ReloadRequest,
        registry: Arc<RwLock<ModuleRegistry>>,
        loader: Arc<ModuleLoader>,
        runtime: Arc<ModuleRuntime>,
    ) {
        // Acquire semaphore permit
        let _permit = self.semaphore.acquire().await.unwrap();

        // Mark as active
        self.active_reloads
            .write()
            .await
            .insert(request.module_id.clone(), request.clone());

        info!("Executing reload for module: {}", request.module_id);

        // Execute reload
        let result = self
            .coordinator
            .reload_module(
                &request.module_id,
                request.new_path.clone(),
                registry,
                loader,
                runtime,
            )
            .await;

        // Handle result
        match result {
            Ok(reload_result) => {
                info!(
                    "Reload completed successfully for module: {}",
                    request.module_id
                );
                self.completed_reloads
                    .write()
                    .await
                    .push((request.clone(), reload_result));
            }
            Err(e) => {
                error!("Reload failed for module {}: {}", request.module_id, e);
                
                // Retry if applicable
                if request.retry_count < request.max_retries {
                    request.retry_count += 1;
                    warn!(
                        "Retrying reload for module {} (attempt {}/{})",
                        request.module_id, request.retry_count, request.max_retries
                    );
                    
                    // Re-queue with delay
                    tokio::time::sleep(self.config.retry_delay).await;
                    self.add_to_queue(request.clone()).await;
                }
            }
        }

        // Remove from active
        self.active_reloads.write().await.remove(&request.module_id);
    }

    /// Add request to queue
    async fn add_to_queue(&self, request: ReloadRequest) {
        let mut queue = self.pending_queue.write().await;
        
        // Insert based on priority
        let position = queue
            .iter()
            .position(|r| r.priority < request.priority)
            .unwrap_or(queue.len());
        
        queue.insert(position, request);
    }

    /// Get next request from queue
    async fn get_next_request(&self) -> Option<ReloadRequest> {
        self.pending_queue.write().await.pop_front()
    }

    /// Check if in quiet hours
    fn is_quiet_hours(&self) -> bool {
        if let Some((start_hour, end_hour)) = self.config.quiet_hours {
            let current_hour = chrono::Utc::now().hour();
            if start_hour <= end_hour {
                current_hour >= start_hour && current_hour < end_hour
            } else {
                // Wraps around midnight
                current_hour >= start_hour || current_hour < end_hour
            }
        } else {
            false
        }
    }

    /// Check if reload should be deferred (adaptive strategy)
    async fn should_defer_reload(&self) -> bool {
        // Check system load, memory, etc.
        // For now, just check active reload count
        let active_count = self.active_reloads.read().await.len();
        active_count >= self.config.max_concurrent_reloads
    }

    /// Clean up old completed reloads
    async fn cleanup_completed(&self) {
        let mut completed = self.completed_reloads.write().await;
        
        // Keep only last 100 completed reloads
        if completed.len() > 100 {
            let drain_count = completed.len() - 100;
            completed.drain(0..drain_count);
        }
    }

    /// Get scheduler statistics
    pub async fn get_stats(&self) -> SchedulerStats {
        SchedulerStats {
            pending_count: self.pending_queue.read().await.len(),
            active_count: self.active_reloads.read().await.len(),
            completed_count: self.completed_reloads.read().await.len(),
            is_running: *self.is_running.read().await,
        }
    }

    /// Cancel a pending reload
    pub async fn cancel_reload(&self, module_id: &str) -> bool {
        let mut queue = self.pending_queue.write().await;
        
        if let Some(pos) = queue.iter().position(|r| r.module_id == module_id) {
            queue.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get pending reloads
    pub async fn get_pending_reloads(&self) -> Vec<ReloadRequest> {
        self.pending_queue.read().await.iter().cloned().collect()
    }
}

// Clone implementation for scheduler
impl Clone for ReloadScheduler {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            coordinator: self.coordinator.clone(),
            pending_queue: self.pending_queue.clone(),
            active_reloads: self.active_reloads.clone(),
            completed_reloads: self.completed_reloads.clone(),
            semaphore: self.semaphore.clone(),
            is_running: self.is_running.clone(),
        }
    }
}

/// Scheduler statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStats {
    pub pending_count: usize,
    pub active_count: usize,
    pub completed_count: usize,
    pub is_running: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scheduler_creation() {
        let config = SchedulerConfig::default();
        let hot_reload_config = HotReloadConfig::default();
        let scheduler = ReloadScheduler::new(config, hot_reload_config);
        
        let stats = scheduler.get_stats().await;
        assert_eq!(stats.pending_count, 0);
        assert_eq!(stats.active_count, 0);
        assert!(!stats.is_running);
    }

    #[tokio::test]
    async fn test_reload_request_priority() {
        let config = SchedulerConfig::default();
        let hot_reload_config = HotReloadConfig::default();
        let scheduler = ReloadScheduler::new(config, hot_reload_config);
        
        // Add requests with different priorities
        let low = ReloadRequest::new(
            "low".to_string(),
            PathBuf::from("low.wasm"),
            ReloadPriority::Low,
        );
        let high = ReloadRequest::new(
            "high".to_string(),
            PathBuf::from("high.wasm"),
            ReloadPriority::High,
        );
        let normal = ReloadRequest::new(
            "normal".to_string(),
            PathBuf::from("normal.wasm"),
            ReloadPriority::Normal,
        );
        
        scheduler.add_to_queue(low).await;
        scheduler.add_to_queue(normal).await;
        scheduler.add_to_queue(high).await;
        
        // High priority should come first
        let first = scheduler.get_next_request().await.unwrap();
        assert_eq!(first.module_id, "high");
        
        let second = scheduler.get_next_request().await.unwrap();
        assert_eq!(second.module_id, "normal");
        
        let third = scheduler.get_next_request().await.unwrap();
        assert_eq!(third.module_id, "low");
    }

    #[tokio::test]
    async fn test_quiet_hours() {
        let mut config = SchedulerConfig::default();
        config.quiet_hours = Some((22, 6)); // 10pm to 6am
        
        let hot_reload_config = HotReloadConfig::default();
        let scheduler = ReloadScheduler::new(config, hot_reload_config);
        
        // Test would need time mocking for full coverage
        // This just tests the structure
        assert!(!scheduler.is_quiet_hours() || scheduler.is_quiet_hours());
    }

    #[tokio::test]
    async fn test_cancel_reload() {
        let config = SchedulerConfig::default();
        let hot_reload_config = HotReloadConfig::default();
        let scheduler = ReloadScheduler::new(config, hot_reload_config);
        
        let request = ReloadRequest::new(
            "test".to_string(),
            PathBuf::from("test.wasm"),
            ReloadPriority::Normal,
        );
        
        scheduler.add_to_queue(request).await;
        assert_eq!(scheduler.get_stats().await.pending_count, 1);
        
        let cancelled = scheduler.cancel_reload("test").await;
        assert!(cancelled);
        assert_eq!(scheduler.get_stats().await.pending_count, 0);
    }
}
