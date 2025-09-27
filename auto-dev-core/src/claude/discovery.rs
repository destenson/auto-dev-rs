//! Claude configuration discovery system
//!
//! Discovers .claude directories in both project-local and user home locations,
//! establishing the foundation for Claude configuration awareness.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Location types for Claude configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaudeConfigLocation {
    /// Project-specific configuration (./.claude)
    Project,
    /// Global user configuration (~/.claude)
    Global,
    /// Both project and global configurations exist
    Both,
    /// No configuration found
    None,
}

/// Discovered Claude configuration paths
#[derive(Debug, Clone)]
pub struct ClaudeConfigPaths {
    /// Project .claude directory path (if exists)
    pub project_dir: Option<PathBuf>,
    /// Global ~/.claude directory path (if exists)
    pub global_dir: Option<PathBuf>,
    /// CLAUDE.md file path in project (if exists)
    pub project_claude_md: Option<PathBuf>,
    /// CLAUDE.md file path in global (if exists)
    pub global_claude_md: Option<PathBuf>,
    /// Commands directory in project (if exists)
    pub project_commands_dir: Option<PathBuf>,
    /// Commands directory in global (if exists)
    pub global_commands_dir: Option<PathBuf>,
}

impl ClaudeConfigPaths {
    /// Get the location type based on discovered paths
    pub fn location(&self) -> ClaudeConfigLocation {
        match (self.project_dir.is_some(), self.global_dir.is_some()) {
            (true, true) => ClaudeConfigLocation::Both,
            (true, false) => ClaudeConfigLocation::Project,
            (false, true) => ClaudeConfigLocation::Global,
            (false, false) => ClaudeConfigLocation::None,
        }
    }

    /// Get all CLAUDE.md paths in priority order (project first)
    pub fn claude_md_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(path) = &self.project_claude_md {
            paths.push(path.clone());
        }
        if let Some(path) = &self.global_claude_md {
            paths.push(path.clone());
        }
        paths
    }

    /// Get all commands directories in priority order (project first)
    pub fn commands_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Some(dir) = &self.project_commands_dir {
            dirs.push(dir.clone());
        }
        if let Some(dir) = &self.global_commands_dir {
            dirs.push(dir.clone());
        }
        dirs
    }
}

/// Cache entry for discovered paths
struct CacheEntry {
    paths: ClaudeConfigPaths,
    timestamp: Instant,
}

/// Claude configuration discovery system
pub struct ClaudeConfigDiscovery {
    /// Cache for discovered paths
    cache: Arc<RwLock<Option<CacheEntry>>>,
    /// Cache TTL duration (default: 5 minutes)
    cache_ttl: Duration,
    /// Current working directory (for testing)
    working_dir: Option<PathBuf>,
}

impl ClaudeConfigDiscovery {
    /// Create a new discovery instance
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(None)),
            cache_ttl: Duration::from_secs(300), // 5 minutes
            working_dir: None,
        }
    }

    /// Create a discovery instance with custom working directory (for testing)
    pub fn with_working_dir(dir: PathBuf) -> Self {
        Self {
            cache: Arc::new(RwLock::new(None)),
            cache_ttl: Duration::from_secs(300),
            working_dir: Some(dir),
        }
    }

    /// Set the cache TTL duration
    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// Discover Claude configuration paths
    pub async fn discover(&self) -> Result<ClaudeConfigPaths> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.as_ref() {
                if entry.timestamp.elapsed() < self.cache_ttl {
                    debug!("Using cached Claude configuration paths");
                    return Ok(entry.paths.clone());
                }
            }
        }

        // Perform discovery
        info!("Discovering Claude configuration paths");
        let paths = self.discover_paths().await?;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            *cache = Some(CacheEntry {
                paths: paths.clone(),
                timestamp: Instant::now(),
            });
        }

        Ok(paths)
    }

    /// Clear the cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        *cache = None;
    }

    /// Perform actual path discovery
    async fn discover_paths(&self) -> Result<ClaudeConfigPaths> {
        let mut paths = ClaudeConfigPaths {
            project_dir: None,
            global_dir: None,
            project_claude_md: None,
            global_claude_md: None,
            project_commands_dir: None,
            global_commands_dir: None,
        };

        // Discover project .claude directory
        let project_dir = self.find_project_claude_dir().await?;
        if let Some(dir) = project_dir {
            debug!("Found project .claude directory: {:?}", dir);
            paths.project_dir = Some(dir.clone());
            
            // Check for CLAUDE.md
            let claude_md = dir.join("CLAUDE.md");
            if self.is_readable_file(&claude_md).await {
                paths.project_claude_md = Some(claude_md);
            }
            
            // Check for commands directory
            let commands_dir = dir.join("commands");
            if self.is_readable_dir(&commands_dir).await {
                paths.project_commands_dir = Some(commands_dir);
            }
        }

        // Discover global ~/.claude directory
        let global_dir = self.find_global_claude_dir().await?;
        if let Some(dir) = global_dir {
            debug!("Found global .claude directory: {:?}", dir);
            paths.global_dir = Some(dir.clone());
            
            // Check for CLAUDE.md
            let claude_md = dir.join("CLAUDE.md");
            if self.is_readable_file(&claude_md).await {
                paths.global_claude_md = Some(claude_md);
            }
            
            // Check for commands directory
            let commands_dir = dir.join("commands");
            if self.is_readable_dir(&commands_dir).await {
                paths.global_commands_dir = Some(commands_dir);
            }
        }

        info!("Discovered Claude configuration: {:?}", paths.location());
        Ok(paths)
    }

    /// Find project .claude directory (searches up from current directory)
    async fn find_project_claude_dir(&self) -> Result<Option<PathBuf>> {
        let start_dir = if let Some(dir) = &self.working_dir {
            dir.clone()
        } else {
            std::env::current_dir().context("Failed to get current directory")?
        };

        let mut current = start_dir.as_path();
        
        loop {
            let claude_dir = current.join(".claude");
            if self.is_readable_dir(&claude_dir).await {
                return Ok(Some(claude_dir));
            }

            // For tests with working_dir set, stop at temp directory root
            // to avoid finding .claude directories outside the test environment
            if self.working_dir.is_some() {
                // Check if we're at a temp directory root (contains "tmp" or "temp" in path)
                let path_str = current.to_string_lossy().to_lowercase();
                if path_str.contains("\\tmp") || path_str.contains("/tmp") || 
                   path_str.contains("\\temp") || path_str.contains("/temp") {
                    // If next parent would exit temp dir, stop searching
                    if let Some(parent) = current.parent() {
                        let parent_str = parent.to_string_lossy().to_lowercase();
                        if !parent_str.contains("\\tmp") && !parent_str.contains("/tmp") &&
                           !parent_str.contains("\\temp") && !parent_str.contains("/temp") {
                            break;
                        }
                    }
                }
            }

            // Check parent directory
            match current.parent() {
                Some(parent) => current = parent,
                None => break,
            }
        }

        Ok(None)
    }

    /// Find global ~/.claude directory
    async fn find_global_claude_dir(&self) -> Result<Option<PathBuf>> {
        // If we have a working directory set (for testing), don't check global
        if self.working_dir.is_some() {
            return Ok(None);
        }
        
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        
        let claude_dir = home_dir.join(".claude");
        if self.is_readable_dir(&claude_dir).await {
            Ok(Some(claude_dir))
        } else {
            Ok(None)
        }
    }

    /// Check if a path is a readable directory
    async fn is_readable_dir(&self, path: &Path) -> bool {
        match tokio::fs::metadata(path).await {
            Ok(metadata) => {
                if !metadata.is_dir() {
                    return false;
                }
                // Try to read directory to check permissions
                match tokio::fs::read_dir(path).await {
                    Ok(_) => true,
                    Err(e) => {
                        debug!("Cannot read directory {:?}: {}", path, e);
                        false
                    }
                }
            }
            Err(_) => false,
        }
    }

    /// Check if a path is a readable file
    async fn is_readable_file(&self, path: &Path) -> bool {
        match tokio::fs::metadata(path).await {
            Ok(metadata) => {
                if !metadata.is_file() {
                    return false;
                }
                // Try to open file to check permissions
                match tokio::fs::File::open(path).await {
                    Ok(_) => true,
                    Err(e) => {
                        debug!("Cannot read file {:?}: {}", path, e);
                        false
                    }
                }
            }
            Err(_) => false,
        }
    }
}

impl Default for ClaudeConfigDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_no_claude_directories() {
        let temp_dir = TempDir::new().unwrap();
        // Create a nested directory to ensure we don't find parent .claude dirs
        let test_dir = temp_dir.path().join("test").join("nested");
        tokio::fs::create_dir_all(&test_dir).await.unwrap();
        
        let discovery = ClaudeConfigDiscovery::with_working_dir(test_dir)
            .with_cache_ttl(Duration::from_secs(0));
        
        let paths = discovery.discover().await.unwrap();
        assert_eq!(paths.location(), ClaudeConfigLocation::None);
        assert!(paths.project_dir.is_none());
        assert!(paths.global_dir.is_none());
    }

    #[tokio::test]
    async fn test_project_claude_directory() {
        let temp_dir = TempDir::new().unwrap();
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir(&claude_dir).await.unwrap();
        
        // Create CLAUDE.md
        let claude_md = claude_dir.join("CLAUDE.md");
        fs::write(&claude_md, "# Test instructions").await.unwrap();
        
        // Create commands directory
        let commands_dir = claude_dir.join("commands");
        fs::create_dir(&commands_dir).await.unwrap();
        
        let discovery = ClaudeConfigDiscovery::with_working_dir(temp_dir.path().to_path_buf())
            .with_cache_ttl(Duration::from_secs(0));
        
        let paths = discovery.discover().await.unwrap();
        assert_eq!(paths.location(), ClaudeConfigLocation::Project);
        assert!(paths.project_dir.is_some());
        assert!(paths.project_claude_md.is_some());
        assert!(paths.project_commands_dir.is_some());
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        let temp_dir = TempDir::new().unwrap();
        // Use nested directory to avoid finding parent .claude
        let test_dir = temp_dir.path().join("test");
        fs::create_dir(&test_dir).await.unwrap();
        
        let claude_dir = test_dir.join(".claude");
        fs::create_dir(&claude_dir).await.unwrap();
        
        let discovery = ClaudeConfigDiscovery::with_working_dir(test_dir.clone())
            .with_cache_ttl(Duration::from_secs(60));
        
        // First discovery
        let paths1 = discovery.discover().await.unwrap();
        assert_eq!(paths1.location(), ClaudeConfigLocation::Project);
        
        // Delete the directory
        fs::remove_dir_all(&claude_dir).await.unwrap();
        
        // Second discovery should use cache
        let paths2 = discovery.discover().await.unwrap();
        assert_eq!(paths2.location(), ClaudeConfigLocation::Project);
        
        // Clear cache and discover again
        discovery.clear_cache().await;
        let paths3 = discovery.discover().await.unwrap();
        assert_eq!(paths3.location(), ClaudeConfigLocation::None);
    }

    #[tokio::test]
    async fn test_priority_order() {
        let temp_dir = TempDir::new().unwrap();
        let project_claude = temp_dir.path().join(".claude");
        fs::create_dir(&project_claude).await.unwrap();
        
        let project_claude_md = project_claude.join("CLAUDE.md");
        fs::write(&project_claude_md, "# Project").await.unwrap();
        
        let project_commands = project_claude.join("commands");
        fs::create_dir(&project_commands).await.unwrap();
        
        let discovery = ClaudeConfigDiscovery::with_working_dir(temp_dir.path().to_path_buf())
            .with_cache_ttl(Duration::from_secs(0));
        
        let paths = discovery.discover().await.unwrap();
        
        // Check priority order (project should come first)
        let claude_md_paths = paths.claude_md_paths();
        assert!(!claude_md_paths.is_empty());
        assert!(claude_md_paths[0].to_string_lossy().contains(".claude"));
        
        let commands_dirs = paths.commands_dirs();
        assert!(!commands_dirs.is_empty());
        assert!(commands_dirs[0].to_string_lossy().contains("commands"));
    }

    #[tokio::test]
    async fn test_search_up_directories() {
        let temp_dir = TempDir::new().unwrap();
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir(&claude_dir).await.unwrap();
        
        // Create nested directory
        let nested = temp_dir.path().join("src").join("nested");
        fs::create_dir_all(&nested).await.unwrap();
        
        let discovery = ClaudeConfigDiscovery::with_working_dir(nested.clone())
            .with_cache_ttl(Duration::from_secs(0));
        
        let paths = discovery.discover().await.unwrap();
        assert_eq!(paths.location(), ClaudeConfigLocation::Project);
        assert!(paths.project_dir.is_some());
        assert_eq!(paths.project_dir.unwrap(), claude_dir);
    }

    #[tokio::test]
    async fn test_permission_handling() {
        let temp_dir = TempDir::new().unwrap();
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir(&claude_dir).await.unwrap();
        
        // Create file with restricted permissions (platform-specific)
        let claude_md = claude_dir.join("CLAUDE.md");
        fs::write(&claude_md, "test").await.unwrap();
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&claude_md).await.unwrap().permissions();
            perms.set_mode(0o000);
            fs::set_permissions(&claude_md, perms).await.unwrap();
        }
        
        let discovery = ClaudeConfigDiscovery::with_working_dir(temp_dir.path().to_path_buf())
            .with_cache_ttl(Duration::from_secs(0));
        
        let paths = discovery.discover().await.unwrap();
        // Should find directory but may not be able to read file
        assert_eq!(paths.location(), ClaudeConfigLocation::Project);
        
        // Restore permissions for cleanup
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&claude_md).await.unwrap().permissions();
            perms.set_mode(0o644);
            fs::set_permissions(&claude_md, perms).await.unwrap();
        }
    }
}
