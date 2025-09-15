//! Self-targeting configuration for auto-dev-rs
//!
//! Enables auto-dev-rs to analyze and improve its own codebase
//! by treating itself as any other Rust project.

use anyhow::{Result, Context};
use cargo_metadata::{MetadataCommand, Package};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;

/// Configuration for self-targeting mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfTargetConfig {
    /// Project information from cargo metadata
    pub project: ProjectInfo,
    
    /// Monitor configuration for watching files
    pub monitor: MonitorConfig,
    
    /// Analyzer configuration
    pub analyzer: AnalyzerConfig,
    
    /// Synthesis configuration
    pub synthesis: SynthesisConfig,
    
    /// Safety validations
    pub safety: SafetyConfig,
}

/// Project information derived from cargo metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// Project root path
    pub path: PathBuf,
    
    /// Project name
    pub name: String,
    
    /// Workspace root if in a workspace
    pub workspace_root: Option<PathBuf>,
    
    /// Package version
    pub version: String,
    
    /// Source directories
    pub src_dirs: Vec<PathBuf>,
}

/// Monitor configuration for file watching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    /// Paths to watch
    pub watch: Vec<String>,
    
    /// Paths to exclude from watching
    pub exclude: Vec<String>,
    
    /// Debounce time in milliseconds
    pub debounce_ms: u64,
}

/// Analyzer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerConfig {
    /// Programming language
    pub language: String,
    
    /// Whether this is a workspace project
    pub workspace: bool,
    
    /// File extensions to analyze
    pub extensions: Vec<String>,
}

/// Synthesis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisConfig {
    /// Target directory for synthesis
    pub target: PathBuf,
    
    /// Safety mode (strict, normal, permissive)
    pub safety_mode: SafetyMode,
    
    /// Whether to allow destructive operations
    pub allow_destructive: bool,
}

/// Safety mode for self-targeting operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SafetyMode {
    /// Strict safety - no destructive operations allowed
    Strict,
    /// Normal safety - some modifications allowed with validation
    Normal,
    /// Permissive - allow most operations (dangerous!)
    Permissive,
}

/// Safety validations configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    /// Require confirmation for destructive operations
    pub require_confirmation: bool,
    
    /// Create backups before modifications
    pub create_backups: bool,
    
    /// Maximum file size to modify (in bytes)
    pub max_file_size: usize,
    
    /// Forbidden paths that should never be modified
    pub forbidden_paths: Vec<String>,
}

impl SelfTargetConfig {
    /// Create configuration from cargo metadata
    pub fn from_cargo_metadata() -> Result<Self> {
        let metadata = MetadataCommand::new()
            .exec()
            .context("Failed to execute cargo metadata")?;
        
        let root_package = metadata.root_package()
            .context("No root package found")?;
        
        let workspace_root = metadata.workspace_root.as_std_path().to_path_buf();
        let project_path = root_package.manifest_path.parent()
            .map(|p| p.as_std_path().to_path_buf())
            .unwrap_or_else(|| workspace_root.clone());
        
        let mut src_dirs = vec![];
        
        // Find all src directories in the workspace
        for package in &metadata.packages {
            if let Some(parent) = package.manifest_path.parent() {
                let src_dir = parent.as_std_path().join("src");
                if src_dir.exists() {
                    src_dirs.push(src_dir);
                }
            }
        }
        
        let project_info = ProjectInfo {
            path: project_path.clone(),
            name: root_package.name.to_string(),
            workspace_root: Some(workspace_root.clone()),
            version: root_package.version.to_string(),
            src_dirs,
        };
        
        let monitor = MonitorConfig {
            watch: vec![
                "src/**/*.rs".to_string(),
                "PRPs/*.md".to_string(),
                "*.toml".to_string(),
                "auto-dev-core/src/**/*.rs".to_string(),
                "auto-dev/src/**/*.rs".to_string(),
            ],
            exclude: vec![
                "target/".to_string(),
                ".git/".to_string(),
                "*.bak".to_string(),
            ],
            debounce_ms: 1000,
        };
        
        let analyzer = AnalyzerConfig {
            language: "rust".to_string(),
            workspace: true,
            extensions: vec!["rs".to_string(), "toml".to_string(), "md".to_string()],
        };
        
        let synthesis = SynthesisConfig {
            target: project_path,
            safety_mode: SafetyMode::Strict,
            allow_destructive: false,
        };
        
        let safety = SafetyConfig {
            require_confirmation: true,
            create_backups: true,
            max_file_size: 1024 * 1024, // 1MB
            forbidden_paths: vec![
                ".git".to_string(),
                "target".to_string(),
                "Cargo.lock".to_string(),
            ],
        };
        
        Ok(Self {
            project: project_info,
            monitor,
            analyzer,
            synthesis,
            safety,
        })
    }
    
    /// Load configuration from file or create from metadata
    pub fn load_or_create() -> Result<Self> {
        let config_path = PathBuf::from(".auto-dev/self.toml");
        
        if config_path.exists() {
            Self::from_file(&config_path)
        } else {
            Self::from_cargo_metadata()
        }
    }
    
    /// Load configuration from a TOML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .context("Failed to read self-target config file")?;
        
        let config: Self = toml::from_str(&content)
            .context("Failed to parse self-target config")?;
        
        Ok(config)
    }
    
    /// Save configuration to a TOML file
    pub fn save(&self, path: &Path) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }
        
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize self-target config")?;
        
        fs::write(path, content)
            .context("Failed to write self-target config file")?;
        
        Ok(())
    }
    
    /// Validate that a path is safe to modify
    pub fn is_safe_to_modify(&self, path: &Path) -> bool {
        // Check forbidden paths
        for forbidden in &self.safety.forbidden_paths {
            if path.starts_with(forbidden) {
                return false;
            }
        }
        
        // Check file size if it exists
        if let Ok(metadata) = fs::metadata(path) {
            if metadata.len() as usize > self.safety.max_file_size {
                return false;
            }
        }
        
        true
    }
    
    /// Get the default configuration path
    pub fn default_config_path() -> PathBuf {
        PathBuf::from(".auto-dev/self.toml")
    }
}

/// Initialize self-targeting mode
pub async fn init_self_targeting() -> Result<()> {
    let config = SelfTargetConfig::from_cargo_metadata()?;
    let config_path = SelfTargetConfig::default_config_path();
    
    // Save the configuration
    config.save(&config_path)?;
    
    println!("Self-targeting configuration created at: {}", config_path.display());
    println!("Project: {} v{}", config.project.name, config.project.version);
    println!("Workspace root: {:?}", config.project.workspace_root);
    println!("Watching: {:?}", config.monitor.watch);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_safety_mode_serialization() {
        let mode = SafetyMode::Strict;
        let serialized = serde_json::to_string(&mode).unwrap();
        let deserialized: SafetyMode = serde_json::from_str(&serialized).unwrap();
        assert!(matches!(deserialized, SafetyMode::Strict));
    }
    
    #[test]
    fn test_is_safe_to_modify() {
        let config = SelfTargetConfig {
            safety: SafetyConfig {
                require_confirmation: true,
                create_backups: true,
                max_file_size: 1000,
                forbidden_paths: vec![".git".to_string(), "target".to_string()],
            },
            ..Default::default()
        };
        
        assert!(!config.is_safe_to_modify(Path::new(".git/config")));
        assert!(!config.is_safe_to_modify(Path::new("target/debug/auto-dev")));
        assert!(config.is_safe_to_modify(Path::new("src/main.rs")));
    }
}

impl Default for SelfTargetConfig {
    fn default() -> Self {
        Self {
            project: ProjectInfo {
                path: PathBuf::from("."),
                name: "auto-dev-rs".to_string(),
                workspace_root: None,
                version: "0.1.0".to_string(),
                src_dirs: vec![],
            },
            monitor: MonitorConfig {
                watch: vec!["src/**/*.rs".to_string()],
                exclude: vec!["target/".to_string()],
                debounce_ms: 1000,
            },
            analyzer: AnalyzerConfig {
                language: "rust".to_string(),
                workspace: false,
                extensions: vec!["rs".to_string()],
            },
            synthesis: SynthesisConfig {
                target: PathBuf::from("."),
                safety_mode: SafetyMode::Strict,
                allow_destructive: false,
            },
            safety: SafetyConfig {
                require_confirmation: true,
                create_backups: true,
                max_file_size: 1024 * 1024,
                forbidden_paths: vec![],
            },
        }
    }
}