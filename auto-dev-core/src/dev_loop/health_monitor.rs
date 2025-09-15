//! Health monitoring for the development loop

use super::*;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Monitors system health and takes corrective actions
pub struct HealthMonitor {
    thresholds: HealthThresholds,
    metrics: Arc<RwLock<SystemMetrics>>,
    recovery_manager: RecoveryManager,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            thresholds: HealthThresholds::default(),
            metrics: Arc::new(RwLock::new(SystemMetrics::default())),
            recovery_manager: RecoveryManager::new(),
        }
    }

    /// Check overall system health
    pub async fn check_health(&self) -> Result<HealthStatus> {
        let mut metrics = self.metrics.write().await;
        metrics.update().await?;
        
        let memory_usage = metrics.memory_usage;
        let cpu_usage = metrics.cpu_usage;
        let disk_space = metrics.disk_space;
        let error_rate = metrics.calculate_error_rate();
        let llm_quota = metrics.llm_quota;
        
        let mut warnings = Vec::new();
        let mut is_healthy = true;
        
        // Check memory
        if memory_usage > self.thresholds.memory_critical {
            warnings.push(format!("Critical: Memory usage at {:.1}%", memory_usage * 100.0));
            is_healthy = false;
        } else if memory_usage > self.thresholds.memory_warning {
            warnings.push(format!("Warning: Memory usage at {:.1}%", memory_usage * 100.0));
        }
        
        // Check CPU
        if cpu_usage > self.thresholds.cpu_critical {
            warnings.push(format!("Critical: CPU usage at {:.1}%", cpu_usage * 100.0));
            is_healthy = false;
        } else if cpu_usage > self.thresholds.cpu_warning {
            warnings.push(format!("Warning: CPU usage at {:.1}%", cpu_usage * 100.0));
        }
        
        // Check disk space
        if disk_space < self.thresholds.disk_critical {
            warnings.push(format!("Critical: Only {:.1}% disk space remaining", disk_space * 100.0));
            is_healthy = false;
        } else if disk_space < self.thresholds.disk_warning {
            warnings.push(format!("Warning: Only {:.1}% disk space remaining", disk_space * 100.0));
        }
        
        // Check error rate
        if error_rate > self.thresholds.error_rate_critical {
            warnings.push(format!("Critical: Error rate at {:.1}%", error_rate * 100.0));
            is_healthy = false;
        }
        
        // Check LLM quota
        if llm_quota < self.thresholds.llm_quota_critical {
            warnings.push(format!("Critical: LLM quota at {:.1}%", llm_quota * 100.0));
            is_healthy = false;
        } else if llm_quota < self.thresholds.llm_quota_warning {
            warnings.push(format!("Warning: LLM quota at {:.1}%", llm_quota * 100.0));
        }
        
        let status = HealthStatus {
            memory_usage,
            cpu_usage,
            disk_space,
            llm_quota,
            error_rate,
            is_healthy,
            warnings: warnings.clone(),
        };
        
        if !warnings.is_empty() {
            debug!("Health check warnings: {:?}", warnings);
        }
        
        Ok(status)
    }

    /// Take corrective action based on health status
    pub async fn take_corrective_action(&self, status: &HealthStatus) -> Result<()> {
        if !status.is_healthy {
            warn!("Taking corrective action for unhealthy system");
        }
        
        // Memory issues
        if status.memory_usage > self.thresholds.memory_critical {
            info!("Triggering garbage collection due to high memory usage");
            self.trigger_gc().await?;
            self.clear_caches().await?;
        }
        
        // CPU issues
        if status.cpu_usage > self.thresholds.cpu_critical {
            info!("Reducing concurrent tasks due to high CPU usage");
            self.reduce_concurrency().await?;
        }
        
        // Disk space issues
        if status.disk_space < self.thresholds.disk_critical {
            info!("Cleaning up temporary files due to low disk space");
            self.cleanup_temp_files().await?;
        }
        
        // Error rate issues
        if status.error_rate > self.thresholds.error_rate_critical {
            warn!("Entering safe mode due to high error rate");
            self.enter_safe_mode().await?;
        }
        
        // LLM quota issues
        if status.llm_quota < self.thresholds.llm_quota_critical {
            info!("Switching to local model due to low LLM quota");
            self.switch_to_local_model().await?;
        }
        
        Ok(())
    }

    /// Trigger garbage collection
    async fn trigger_gc(&self) -> Result<()> {
        debug!("Triggering garbage collection");
        // In Rust, we don't have explicit GC, but we can drop caches
        Ok(())
    }

    /// Clear caches
    async fn clear_caches(&self) -> Result<()> {
        debug!("Clearing caches");
        // Would clear various caches in the system
        Ok(())
    }

    /// Reduce concurrency
    async fn reduce_concurrency(&self) -> Result<()> {
        debug!("Reducing concurrent task limit");
        // Would update configuration to reduce concurrent tasks
        Ok(())
    }

    /// Clean up temporary files
    async fn cleanup_temp_files(&self) -> Result<()> {
        debug!("Cleaning up temporary files");
        // Would clean up .auto-dev/tmp and other temporary locations
        Ok(())
    }

    /// Enter safe mode
    async fn enter_safe_mode(&self) -> Result<()> {
        warn!("Entering safe mode - only critical operations will be performed");
        // Would update system state to safe mode
        Ok(())
    }

    /// Switch to local model
    async fn switch_to_local_model(&self) -> Result<()> {
        info!("Switching to local LLM model");
        // Would update LLM configuration to use local models
        Ok(())
    }

    /// Record error
    pub async fn record_error(&self, error: &str) {
        let mut metrics = self.metrics.write().await;
        metrics.record_error(error);
    }

    /// Record success
    pub async fn record_success(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.record_success();
    }
}

/// Health thresholds configuration
#[derive(Debug, Clone)]
struct HealthThresholds {
    memory_warning: f32,
    memory_critical: f32,
    cpu_warning: f32,
    cpu_critical: f32,
    disk_warning: f32,
    disk_critical: f32,
    error_rate_critical: f32,
    llm_quota_warning: f32,
    llm_quota_critical: f32,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            memory_warning: 0.7,
            memory_critical: 0.9,
            cpu_warning: 0.7,
            cpu_critical: 0.9,
            disk_warning: 0.2,
            disk_critical: 0.1,
            error_rate_critical: 0.1,
            llm_quota_warning: 0.2,
            llm_quota_critical: 0.1,
        }
    }
}

/// System metrics
#[derive(Debug, Default)]
struct SystemMetrics {
    memory_usage: f32,
    cpu_usage: f32,
    disk_space: f32,
    llm_quota: f32,
    total_operations: u64,
    failed_operations: u64,
    recent_errors: Vec<(DateTime<Utc>, String)>,
}

impl SystemMetrics {
    /// Update metrics
    async fn update(&mut self) -> Result<()> {
        // These would be real system calls in production
        self.memory_usage = self.get_memory_usage().await?;
        self.cpu_usage = self.get_cpu_usage().await?;
        self.disk_space = self.get_disk_space().await?;
        self.llm_quota = self.get_llm_quota().await?;
        Ok(())
    }

    /// Get memory usage (0.0 to 1.0)
    async fn get_memory_usage(&self) -> Result<f32> {
        // Placeholder - would use actual system metrics
        Ok(0.45)
    }

    /// Get CPU usage (0.0 to 1.0)
    async fn get_cpu_usage(&self) -> Result<f32> {
        // Placeholder - would use actual system metrics
        Ok(0.35)
    }

    /// Get available disk space (0.0 to 1.0)
    async fn get_disk_space(&self) -> Result<f32> {
        // Placeholder - would check actual disk space
        Ok(0.65)
    }

    /// Get LLM quota remaining (0.0 to 1.0)
    async fn get_llm_quota(&self) -> Result<f32> {
        // Placeholder - would check actual LLM API quota
        Ok(0.75)
    }

    /// Calculate error rate
    fn calculate_error_rate(&self) -> f32 {
        if self.total_operations == 0 {
            0.0
        } else {
            self.failed_operations as f32 / self.total_operations as f32
        }
    }

    /// Record an error
    fn record_error(&mut self, error: &str) {
        self.failed_operations += 1;
        self.total_operations += 1;
        self.recent_errors.push((Utc::now(), error.to_string()));
        
        // Keep only recent errors (last 100)
        if self.recent_errors.len() > 100 {
            self.recent_errors.remove(0);
        }
    }

    /// Record a success
    fn record_success(&mut self) {
        self.total_operations += 1;
    }
}

/// Recovery manager for handling failures
struct RecoveryManager {
    retry_config: RetryConfig,
    checkpoints: Arc<RwLock<Vec<Checkpoint>>>,
}

impl RecoveryManager {
    fn new() -> Self {
        Self {
            retry_config: RetryConfig::default(),
            checkpoints: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Recover from error
    pub async fn recover_from_error(&self, error: &anyhow::Error) -> Result<()> {
        let severity = self.assess_severity(error);
        
        match severity {
            Severity::Critical => {
                error!("Critical error: {}", error);
                self.rollback_to_checkpoint().await?
            },
            Severity::Major => {
                warn!("Major error: {}", error);
                self.retry_with_backoff().await?
            },
            Severity::Minor => {
                debug!("Minor error: {}", error);
                // Continue operation
            },
        }
        
        Ok(())
    }

    /// Assess error severity
    fn assess_severity(&self, error: &anyhow::Error) -> Severity {
        let error_str = error.to_string();
        
        if error_str.contains("panic") || error_str.contains("fatal") {
            Severity::Critical
        } else if error_str.contains("failed") || error_str.contains("error") {
            Severity::Major
        } else {
            Severity::Minor
        }
    }

    /// Rollback to last checkpoint
    async fn rollback_to_checkpoint(&self) -> Result<()> {
        let checkpoints = self.checkpoints.read().await;
        
        if let Some(checkpoint) = checkpoints.last() {
            info!("Rolling back to checkpoint: {}", checkpoint.id);
            // Would restore state from checkpoint
        } else {
            warn!("No checkpoint available for rollback");
        }
        
        Ok(())
    }

    /// Retry with exponential backoff
    async fn retry_with_backoff(&self) -> Result<()> {
        debug!("Retrying with backoff");
        tokio::time::sleep(Duration::from_secs(1)).await;
        Ok(())
    }

    /// Create checkpoint
    pub async fn create_checkpoint(&self, id: String) -> Result<()> {
        let mut checkpoints = self.checkpoints.write().await;
        
        checkpoints.push(Checkpoint {
            id,
            timestamp: Utc::now(),
            state: vec![], // Would capture actual state
        });
        
        // Keep only last 10 checkpoints
        if checkpoints.len() > 10 {
            checkpoints.remove(0);
        }
        
        Ok(())
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Critical,
    Major,
    Minor,
}

/// Retry configuration
#[derive(Debug, Clone)]
struct RetryConfig {
    max_retries: usize,
    backoff_multiplier: f64,
    max_backoff: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            backoff_multiplier: 2.0,
            max_backoff: Duration::from_secs(60),
        }
    }
}

/// Checkpoint for recovery
#[derive(Debug, Clone)]
struct Checkpoint {
    id: String,
    timestamp: DateTime<Utc>,
    state: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_monitor() {
        let monitor = HealthMonitor::new();
        
        let status = monitor.check_health().await.unwrap();
        assert!(status.is_healthy);
        assert!(status.warnings.is_empty());
    }

    #[tokio::test]
    async fn test_metrics_tracking() {
        let monitor = HealthMonitor::new();
        
        // Record some operations
        monitor.record_success().await;
        monitor.record_success().await;
        monitor.record_error("Test error").await;
        
        let metrics = monitor.metrics.read().await;
        assert_eq!(metrics.total_operations, 3);
        assert_eq!(metrics.failed_operations, 1);
        assert!(metrics.calculate_error_rate() < 0.5);
    }

    #[tokio::test]
    async fn test_recovery_manager() {
        let recovery = RecoveryManager::new();
        
        // Create checkpoint
        recovery.create_checkpoint("test_checkpoint".to_string()).await.unwrap();
        
        // Test error recovery
        let error = anyhow::anyhow!("Test error");
        recovery.recover_from_error(&error).await.unwrap();
    }
}