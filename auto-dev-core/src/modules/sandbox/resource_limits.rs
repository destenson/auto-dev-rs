//! Resource monitoring and limiting for sandboxed modules

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Tracks resource usage for a sandboxed module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub memory_bytes: u64,
    pub cpu_time_ms: u64,
    pub thread_count: usize,
    pub file_handle_count: usize,
    pub network_bytes_sent: u64,
    pub network_bytes_received: u64,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub last_update: chrono::DateTime<chrono::Utc>,
}

impl ResourceUsage {
    /// Create new resource usage tracker
    pub fn new() -> Self {
        let now = chrono::Utc::now();
        Self {
            memory_bytes: 0,
            cpu_time_ms: 0,
            thread_count: 0,
            file_handle_count: 0,
            network_bytes_sent: 0,
            network_bytes_received: 0,
            start_time: now,
            last_update: now,
        }
    }

    /// Check if usage exceeds configured limits
    pub fn exceeds_limits(&self) -> bool {
        // Check against default limits (can be made configurable)
        if self.memory_bytes > 100 * 1024 * 1024 {  // 100MB
            return true;
        }
        if self.cpu_time_ms > 60 * 1000 {  // 60 seconds
            return true;
        }
        if self.thread_count > 10 {
            return true;
        }
        if self.file_handle_count > 50 {
            return true;
        }
        false
    }

    /// Get a summary of the resource usage
    pub fn summary(&self) -> String {
        format!(
            "Memory: {} MB, CPU: {} ms, Threads: {}, Files: {}",
            self.memory_bytes / (1024 * 1024),
            self.cpu_time_ms,
            self.thread_count,
            self.file_handle_count
        )
    }
}

/// Configured resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory_bytes: Option<u64>,
    pub max_cpu_time_ms: Option<u64>,
    pub max_threads: Option<usize>,
    pub max_file_handles: Option<usize>,
    pub max_network_bandwidth_bps: Option<u64>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: Some(100 * 1024 * 1024),  // 100MB
            max_cpu_time_ms: Some(60 * 1000),           // 60 seconds
            max_threads: Some(10),
            max_file_handles: Some(50),
            max_network_bandwidth_bps: Some(10 * 1024 * 1024),  // 10MB/s
        }
    }
}

/// Monitors resource usage for sandboxed modules
pub struct ResourceMonitor {
    usage: Arc<RwLock<ResourceUsage>>,
    limits: ResourceLimits,
    monitoring_active: Arc<RwLock<bool>>,
    start_instant: Arc<RwLock<Option<Instant>>>,
}

impl ResourceMonitor {
    /// Create a new resource monitor
    pub fn new() -> Self {
        Self::with_limits(ResourceLimits::default())
    }

    /// Create a new resource monitor with specific limits
    pub fn with_limits(limits: ResourceLimits) -> Self {
        Self {
            usage: Arc::new(RwLock::new(ResourceUsage::new())),
            limits,
            monitoring_active: Arc::new(RwLock::new(false)),
            start_instant: Arc::new(RwLock::new(None)),
        }
    }

    /// Start monitoring resources
    pub fn start_monitoring(&self) {
        let mut active = self.monitoring_active.blocking_write();
        *active = true;
        
        let mut instant = self.start_instant.blocking_write();
        *instant = Some(Instant::now());
    }

    /// Stop monitoring resources
    pub fn stop_monitoring(&self) {
        let mut active = self.monitoring_active.blocking_write();
        *active = false;
    }

    /// Get current resource usage
    pub async fn get_usage(&self) -> Result<ResourceUsage> {
        let usage = self.usage.read().await;
        Ok(usage.clone())
    }

    /// Update memory usage
    pub async fn update_memory(&self, bytes: u64) -> Result<()> {
        let mut usage = self.usage.write().await;
        usage.memory_bytes = bytes;
        usage.last_update = chrono::Utc::now();
        
        if let Some(limit) = self.limits.max_memory_bytes {
            if bytes > limit {
                return Err(anyhow::anyhow!(
                    "Memory limit exceeded: {} > {} bytes",
                    bytes,
                    limit
                ));
            }
        }
        
        Ok(())
    }

    /// Update CPU time
    pub async fn update_cpu_time(&self) -> Result<()> {
        let instant = self.start_instant.read().await;
        if let Some(start) = *instant {
            let elapsed = start.elapsed();
            let mut usage = self.usage.write().await;
            usage.cpu_time_ms = elapsed.as_millis() as u64;
            usage.last_update = chrono::Utc::now();
            
            if let Some(limit) = self.limits.max_cpu_time_ms {
                if usage.cpu_time_ms > limit {
                    return Err(anyhow::anyhow!(
                        "CPU time limit exceeded: {} > {} ms",
                        usage.cpu_time_ms,
                        limit
                    ));
                }
            }
        }
        
        Ok(())
    }

    /// Update thread count
    pub async fn update_threads(&self, count: usize) -> Result<()> {
        let mut usage = self.usage.write().await;
        usage.thread_count = count;
        usage.last_update = chrono::Utc::now();
        
        if let Some(limit) = self.limits.max_threads {
            if count > limit {
                return Err(anyhow::anyhow!(
                    "Thread limit exceeded: {} > {}",
                    count,
                    limit
                ));
            }
        }
        
        Ok(())
    }

    /// Update file handle count
    pub async fn update_file_handles(&self, count: usize) -> Result<()> {
        let mut usage = self.usage.write().await;
        usage.file_handle_count = count;
        usage.last_update = chrono::Utc::now();
        
        if let Some(limit) = self.limits.max_file_handles {
            if count > limit {
                return Err(anyhow::anyhow!(
                    "File handle limit exceeded: {} > {}",
                    count,
                    limit
                ));
            }
        }
        
        Ok(())
    }

    /// Check if monitoring is active
    pub async fn is_monitoring(&self) -> bool {
        let active = self.monitoring_active.read().await;
        *active
    }

    /// Enforce resource limits
    pub async fn enforce_limits(&self) -> Result<()> {
        let usage = self.usage.read().await;
        
        if let Some(limit) = self.limits.max_memory_bytes {
            if usage.memory_bytes > limit {
                return Err(anyhow::anyhow!(
                    "Memory limit exceeded: {} > {} bytes",
                    usage.memory_bytes,
                    limit
                ));
            }
        }
        
        if let Some(limit) = self.limits.max_cpu_time_ms {
            if usage.cpu_time_ms > limit {
                return Err(anyhow::anyhow!(
                    "CPU time limit exceeded: {} > {} ms",
                    usage.cpu_time_ms,
                    limit
                ));
            }
        }
        
        if let Some(limit) = self.limits.max_threads {
            if usage.thread_count > limit {
                return Err(anyhow::anyhow!(
                    "Thread limit exceeded: {} > {}",
                    usage.thread_count,
                    limit
                ));
            }
        }
        
        if let Some(limit) = self.limits.max_file_handles {
            if usage.file_handle_count > limit {
                return Err(anyhow::anyhow!(
                    "File handle limit exceeded: {} > {}",
                    usage.file_handle_count,
                    limit
                ));
            }
        }
        
        Ok(())
    }

    /// Get configured limits
    pub fn limits(&self) -> &ResourceLimits {
        &self.limits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resource_monitor() {
        let monitor = ResourceMonitor::new();
        monitor.start_monitoring();
        
        // Update memory usage
        assert!(monitor.update_memory(50 * 1024 * 1024).await.is_ok());
        
        // Exceed memory limit
        assert!(monitor.update_memory(200 * 1024 * 1024).await.is_err());
        
        let usage = monitor.get_usage().await.unwrap();
        assert!(usage.memory_bytes > 0);
    }

    #[tokio::test]
    async fn test_resource_limits_enforcement() {
        let limits = ResourceLimits {
            max_memory_bytes: Some(10 * 1024 * 1024),
            max_threads: Some(5),
            ..Default::default()
        };
        
        let monitor = ResourceMonitor::with_limits(limits);
        
        // Within limits
        assert!(monitor.update_memory(5 * 1024 * 1024).await.is_ok());
        assert!(monitor.update_threads(3).await.is_ok());
        
        // Exceed limits
        assert!(monitor.update_memory(20 * 1024 * 1024).await.is_err());
        assert!(monitor.update_threads(10).await.is_err());
    }
}