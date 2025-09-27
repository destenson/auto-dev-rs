#![allow(unused)]
//! Baseline snapshot creation for bootstrap

use super::{BaselineConfig, BootstrapError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    pub id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub binary_hash: Option<String>,
    pub performance_metrics: Option<PerformanceMetrics>,
    pub capabilities: Option<SystemCapabilities>,
    pub configuration: ConfigurationSnapshot,
    pub rollback_point: RollbackPoint,
    // Context preservation - addresses idempotency concerns
    pub context: SnapshotContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotContext {
    pub reason: String,
    pub previous_snapshot_id: Option<String>,
    pub bootstrap_attempt: u32,
    pub environment_changes: Vec<String>,
    pub intent_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub compilation_time_ms: Option<u64>,
    pub test_execution_time_ms: Option<u64>,
    pub binary_size_bytes: Option<u64>,
    pub memory_usage_mb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemCapabilities {
    pub modules_loaded: Vec<String>,
    pub llm_providers: Vec<String>,
    pub enabled_features: Vec<String>,
    pub safety_gates_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationSnapshot {
    pub cargo_toml_hash: String,
    pub dependencies: Vec<String>,
    pub feature_flags: std::collections::HashMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPoint {
    pub binary_backup_path: Option<PathBuf>,
    pub config_backup_path: PathBuf,
    pub git_commit: Option<String>,
}

// Append-only audit log for context preservation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapAuditEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub action: String,
    pub success: bool,
    pub details: String,
    pub snapshot_id: Option<String>,
    pub context: Option<String>,
}

pub struct BaselineCreator {
    config: BaselineConfig,
    audit_log_path: PathBuf,
}

impl BaselineCreator {
    pub fn new(config: BaselineConfig) -> Self {
        let audit_log_path = config.snapshot_dir.join("audit.jsonl");
        Self {
            config,
            audit_log_path,
        }
    }
    
    pub async fn create_baseline(&self, context: SnapshotContext) -> Result<Baseline> {
        info!("Creating baseline snapshot");
        
        // Log the attempt (append-only for context preservation)
        self.log_audit_entry(BootstrapAuditEntry {
            timestamp: chrono::Utc::now(),
            action: "baseline_creation_started".to_string(),
            success: false, // Will update on success
            details: format!("Attempt #{}, Reason: {}", context.bootstrap_attempt, context.reason),
            snapshot_id: None,
            context: context.intent_description.clone(),
        })?;
        
        let snapshot_id = self.generate_snapshot_id();
        
        let mut baseline = Baseline {
            id: snapshot_id.clone(),
            created_at: chrono::Utc::now(),
            binary_hash: None,
            performance_metrics: None,
            capabilities: None,
            configuration: self.capture_configuration()?,
            rollback_point: self.create_rollback_point(&snapshot_id)?,
            context,
        };
        
        if self.config.include_performance {
            baseline.performance_metrics = Some(self.capture_performance_metrics().await?);
        }
        
        if self.config.include_capabilities {
            baseline.capabilities = Some(self.capture_capabilities()?);
        }
        
        baseline.binary_hash = Some(self.calculate_binary_hash()?);
        
        // Save the baseline
        self.save_baseline(&baseline)?;
        
        // Log successful creation
        self.log_audit_entry(BootstrapAuditEntry {
            timestamp: chrono::Utc::now(),
            action: "baseline_creation_completed".to_string(),
            success: true,
            details: format!("Snapshot {} created successfully", snapshot_id),
            snapshot_id: Some(snapshot_id),
            context: baseline.context.intent_description.clone(),
        })?;
        
        info!("Baseline snapshot created: {}", baseline.id);
        Ok(baseline)
    }
    
    fn generate_snapshot_id(&self) -> String {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        format!("snapshot_{}", timestamp)
    }
    
    fn capture_configuration(&self) -> Result<ConfigurationSnapshot> {
        debug!("Capturing configuration snapshot");
        
        let cargo_toml = fs::read_to_string("Cargo.toml")
            .map_err(|e| BootstrapError::BaselineFailed(
                format!("Failed to read Cargo.toml: {}", e)
            ))?;
        
        let cargo_toml_hash = format!("{:x}", md5::compute(&cargo_toml));
        
        // Parse dependencies (simplified)
        let dependencies = self.extract_dependencies(&cargo_toml);
        
        Ok(ConfigurationSnapshot {
            cargo_toml_hash,
            dependencies,
            feature_flags: std::collections::HashMap::new(),
        })
    }
    
    fn extract_dependencies(&self, cargo_toml: &str) -> Vec<String> {
        let mut deps = Vec::new();
        let mut in_deps = false;
        
        for line in cargo_toml.lines() {
            if line.starts_with("[dependencies]") {
                in_deps = true;
                continue;
            }
            if in_deps && line.starts_with('[') {
                break;
            }
            if in_deps && line.contains('=') {
                if let Some(dep_name) = line.split('=').next() {
                    deps.push(dep_name.trim().to_string());
                }
            }
        }
        
        deps
    }
    
    async fn capture_performance_metrics(&self) -> Result<PerformanceMetrics> {
        debug!("Capturing performance metrics");
        
        let mut metrics = PerformanceMetrics {
            compilation_time_ms: None,
            test_execution_time_ms: None,
            binary_size_bytes: None,
            memory_usage_mb: None,
        };
        
        // Measure compilation time
        let start = std::time::Instant::now();
        let output = Command::new("cargo")
            .args(&["build", "--release"])
            .output()
            .map_err(|e| BootstrapError::BaselineFailed(
                format!("Failed to build for metrics: {}", e)
            ))?;
        
        if output.status.success() {
            metrics.compilation_time_ms = Some(start.elapsed().as_millis() as u64);
            
            // Get binary size
            let binary_path = Path::new("target/release/auto-dev");
            if binary_path.exists() {
                if let Ok(metadata) = fs::metadata(binary_path) {
                    metrics.binary_size_bytes = Some(metadata.len());
                }
            }
        }
        
        Ok(metrics)
    }
    
    fn capture_capabilities(&self) -> Result<SystemCapabilities> {
        debug!("Capturing system capabilities");
        
        Ok(SystemCapabilities {
            modules_loaded: vec!["bootstrap".to_string()],
            llm_providers: vec!["claude".to_string(), "openai".to_string()],
            enabled_features: vec!["self-monitoring".to_string(), "hot-reload".to_string()],
            safety_gates_active: true,
        })
    }
    
    fn calculate_binary_hash(&self) -> Result<String> {
        debug!("Calculating binary hash");
        
        let binary_path = Path::new("target/release/auto-dev");
        if !binary_path.exists() {
            // Try debug build
            let debug_path = Path::new("target/debug/auto-dev");
            if debug_path.exists() {
                let content = fs::read(debug_path)
                    .map_err(|e| BootstrapError::BaselineFailed(
                        format!("Failed to read binary: {}", e)
                    ))?;
                return Ok(format!("{:x}", md5::compute(&content)));
            }
            return Ok("no_binary".to_string());
        }
        
        let content = fs::read(binary_path)
            .map_err(|e| BootstrapError::BaselineFailed(
                format!("Failed to read binary: {}", e)
            ))?;
        
        Ok(format!("{:x}", md5::compute(&content)))
    }
    
    fn create_rollback_point(&self, snapshot_id: &str) -> Result<RollbackPoint> {
        debug!("Creating rollback point");
        
        let backup_dir = self.config.snapshot_dir.join(snapshot_id);
        fs::create_dir_all(&backup_dir)
            .map_err(|e| BootstrapError::BaselineFailed(
                format!("Failed to create backup directory: {}", e)
            ))?;
        
        // Backup configuration
        let config_backup = backup_dir.join("config_backup.toml");
        if Path::new("Cargo.toml").exists() {
            fs::copy("Cargo.toml", &config_backup)
                .map_err(|e| BootstrapError::BaselineFailed(
                    format!("Failed to backup Cargo.toml: {}", e)
                ))?;
        }
        
        // Get current git commit
        let git_commit = Command::new("git")
            .args(&["rev-parse", "HEAD"])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout).ok()
                } else {
                    None
                }
            })
            .map(|s| s.trim().to_string());
        
        Ok(RollbackPoint {
            binary_backup_path: None, // Would copy binary in production
            config_backup_path: config_backup,
            git_commit,
        })
    }
    
    fn save_baseline(&self, baseline: &Baseline) -> Result<()> {
        let baseline_path = self.config.snapshot_dir.join(format!("{}.json", baseline.id));
        
        let json = serde_json::to_string_pretty(baseline)
            .map_err(|e| BootstrapError::BaselineFailed(
                format!("Failed to serialize baseline: {}", e)
            ))?;
        
        fs::write(&baseline_path, json)
            .map_err(|e| BootstrapError::BaselineFailed(
                format!("Failed to save baseline: {}", e)
            ))?;
        
        // Also update the "latest" symlink/reference
        let latest_path = self.config.snapshot_dir.join("latest.json");
        fs::write(&latest_path, serde_json::to_string_pretty(baseline).unwrap())
            .map_err(|e| BootstrapError::BaselineFailed(
                format!("Failed to update latest baseline: {}", e)
            ))?;
        
        debug!("Baseline saved to {:?}", baseline_path);
        Ok(())
    }
    
    // Append-only audit logging for context preservation
    fn log_audit_entry(&self, entry: BootstrapAuditEntry) -> Result<()> {
        let json_line = serde_json::to_string(&entry)
            .map_err(|e| BootstrapError::BaselineFailed(
                format!("Failed to serialize audit entry: {}", e)
            ))?;
        
        // Append to audit log (preserves all context)
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.audit_log_path)
            .map_err(|e| BootstrapError::BaselineFailed(
                format!("Failed to open audit log: {}", e)
            ))?;
        
        writeln!(file, "{}", json_line)
            .map_err(|e| BootstrapError::BaselineFailed(
                format!("Failed to write audit entry: {}", e)
            ))?;
        
        Ok(())
    }
    
    pub fn get_previous_context(&self) -> Option<SnapshotContext> {
        // Read the latest baseline to get context for idempotent operations
        let latest_path = self.config.snapshot_dir.join("latest.json");
        if latest_path.exists() {
            if let Ok(content) = fs::read_to_string(&latest_path) {
                if let Ok(baseline) = serde_json::from_str::<Baseline>(&content) {
                    return Some(baseline.context);
                }
            }
        }
        None
    }
    
    pub fn describe_baseline_components(&self) -> Vec<String> {
        let mut components = vec![
            "Configuration snapshot".to_string(),
            "Rollback point creation".to_string(),
            "Binary hash calculation".to_string(),
        ];
        
        if self.config.include_performance {
            components.push("Performance metrics".to_string());
        }
        
        if self.config.include_capabilities {
            components.push("System capabilities".to_string());
        }
        
        components.push("Audit trail (preserves context)".to_string());
        
        components
    }
}
