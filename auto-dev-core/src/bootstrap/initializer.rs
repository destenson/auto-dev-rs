//! System initialization for bootstrap

use super::{BootstrapError, ModulesConfig, MonitoringConfig, Result, SafetyConfig};
use std::path::{Path, PathBuf};
use std::fs;
use tracing::{debug, info};

pub struct SystemInitializer {
    safety_config: SafetyConfig,
    modules_config: ModulesConfig,
    monitoring_config: MonitoringConfig,
}

impl SystemInitializer {
    pub fn new(
        safety_config: SafetyConfig,
        modules_config: ModulesConfig,
        monitoring_config: MonitoringConfig,
    ) -> Self {
        Self {
            safety_config,
            modules_config,
            monitoring_config,
        }
    }
    
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing system for self-development");
        
        self.create_working_directories()?;
        self.initialize_module_system().await?;
        self.setup_sandbox_environment()?;
        self.configure_monitoring_paths()?;
        self.establish_safety_boundaries()?;
        
        info!("System initialization complete");
        Ok(())
    }
    
    fn create_working_directories(&self) -> Result<()> {
        debug!("Creating working directories");
        
        let dirs = vec![
            ".auto-dev",
            ".auto-dev/snapshots",
            ".auto-dev/backups",
            ".auto-dev/logs",
            ".auto-dev/modules",
            ".auto-dev/sandbox",
            ".auto-dev/metrics",
        ];
        
        for dir in dirs {
            if !Path::new(dir).exists() {
                fs::create_dir_all(dir)
                    .map_err(|e| BootstrapError::InitializationFailed(
                        format!("Failed to create directory {}: {}", dir, e)
                    ))?;
                debug!("Created directory: {}", dir);
            }
        }
        
        Ok(())
    }
    
    async fn initialize_module_system(&self) -> Result<()> {
        debug!("Initializing module system");
        
        if self.modules_config.load_existing {
            // Check for existing modules
            let modules_dir = Path::new(".auto-dev/modules");
            if modules_dir.exists() {
                let entries = fs::read_dir(modules_dir)
                    .map_err(|e| BootstrapError::InitializationFailed(
                        format!("Failed to read modules directory: {}", e)
                    ))?;
                
                let module_count = entries.count();
                if module_count > 0 {
                    debug!("Found {} existing modules", module_count);
                }
            }
        }
        
        // Initialize module registry
        let registry_path = Path::new(".auto-dev/modules/registry.json");
        if !registry_path.exists() {
            let initial_registry = serde_json::json!({
                "modules": [],
                "version": "1.0.0",
                "created_at": chrono::Utc::now().to_rfc3339()
            });
            
            fs::write(registry_path, serde_json::to_string_pretty(&initial_registry).unwrap())
                .map_err(|e| BootstrapError::InitializationFailed(
                    format!("Failed to create module registry: {}", e)
                ))?;
            
            debug!("Created module registry");
        }
        
        Ok(())
    }
    
    fn setup_sandbox_environment(&self) -> Result<()> {
        debug!("Setting up sandbox environment");
        
        if self.modules_config.sandbox_enabled {
            let sandbox_dir = Path::new(".auto-dev/sandbox");
            
            // Create sandbox subdirectories
            let sandbox_dirs = vec![
                ".auto-dev/sandbox/wasm",
                ".auto-dev/sandbox/native",
                ".auto-dev/sandbox/temp",
            ];
            
            for dir in sandbox_dirs {
                if !Path::new(dir).exists() {
                    fs::create_dir_all(dir)
                        .map_err(|e| BootstrapError::InitializationFailed(
                            format!("Failed to create sandbox directory {}: {}", dir, e)
                        ))?;
                }
            }
            
            // Create sandbox configuration
            let sandbox_config_path = Path::new(".auto-dev/sandbox/config.toml");
            if !sandbox_config_path.exists() {
                let config_content = r#"# Sandbox Configuration
[wasm]
enabled = true
max_memory_mb = 256
max_execution_time_ms = 30000

[native]
enabled = true
restricted_paths = [".git", "target", ".auto-dev/backups"]
allowed_commands = ["cargo", "rustc", "git"]

[limits]
max_file_size_mb = 10
max_open_files = 100
"#;
                fs::write(sandbox_config_path, config_content)
                    .map_err(|e| BootstrapError::InitializationFailed(
                        format!("Failed to create sandbox config: {}", e)
                    ))?;
                
                debug!("Created sandbox configuration");
            }
        }
        
        Ok(())
    }
    
    fn configure_monitoring_paths(&self) -> Result<()> {
        debug!("Configuring monitoring paths");
        
        // Create monitoring configuration
        let monitor_config_path = Path::new(".auto-dev/monitor.toml");
        if !monitor_config_path.exists() {
            let watch_paths: Vec<String> = self.monitoring_config.watch_paths.clone();
            
            let config_content = format!(r#"# Monitoring Configuration
[monitor]
enabled = true
watch_paths = {:?}
exclude_paths = ["target", ".git", "node_modules", ".auto-dev/backups"]

[events]
debounce_ms = 500
batch_size = 10
"#, watch_paths);
            
            fs::write(monitor_config_path, config_content)
                .map_err(|e| BootstrapError::InitializationFailed(
                    format!("Failed to create monitor config: {}", e)
                ))?;
            
            debug!("Created monitoring configuration");
        }
        
        Ok(())
    }
    
    fn establish_safety_boundaries(&self) -> Result<()> {
        debug!("Establishing safety boundaries");
        
        // Create safety configuration
        let safety_config_path = Path::new(".auto-dev/safety.toml");
        if !safety_config_path.exists() {
            let config_content = format!(r#"# Safety Configuration
[boundaries]
forbidden_paths = [".git", "target/release", ".auto-dev/backups"]
max_file_size_mb = 50
require_clean_git = {}
create_backups = {}

[validation]
require_tests = true
require_compilation = true
max_changes_per_session = 100

[rollback]
enabled = true
keep_snapshots = 10
"#, self.safety_config.require_clean_git, self.safety_config.create_backup);
            
            fs::write(safety_config_path, config_content)
                .map_err(|e| BootstrapError::InitializationFailed(
                    format!("Failed to create safety config: {}", e)
                ))?;
            
            debug!("Created safety configuration");
        }
        
        Ok(())
    }
    
    pub fn describe_initialization_steps(&self) -> Vec<String> {
        let mut steps = vec![
            "Create working directories".to_string(),
            "Initialize module system".to_string(),
            "Configure monitoring paths".to_string(),
            "Establish safety boundaries".to_string(),
        ];
        
        if self.modules_config.sandbox_enabled {
            steps.insert(2, "Setup sandbox environment".to_string());
        }
        
        if self.modules_config.load_existing {
            steps.push("Load existing modules".to_string());
        }
        
        steps
    }
}