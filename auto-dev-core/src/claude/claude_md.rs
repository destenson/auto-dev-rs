//! CLAUDE.md file loader for user instructions and context
//!
//! Loads and merges CLAUDE.md files from discovered locations,
//! providing user-specific instructions to guide auto-dev's behavior.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Maximum allowed file size for CLAUDE.md (1MB)
const MAX_FILE_SIZE: u64 = 1024 * 1024;

/// Separator used when merging multiple CLAUDE.md files
const MERGE_SEPARATOR: &str = "\n\n---\n\n";

/// Loaded CLAUDE.md content with metadata
#[derive(Debug, Clone)]
pub struct ClaudeMdContent {
    /// The merged content from all CLAUDE.md files
    pub content: String,
    /// Source files that were loaded (in priority order)
    pub sources: Vec<PathBuf>,
    /// Total size of merged content
    pub total_size: usize,
}

impl ClaudeMdContent {
    /// Get the content as a string reference
    pub fn as_str(&self) -> &str {
        &self.content
    }

    /// Check if content is empty
    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty()
    }

    /// Get the number of source files
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }
}

/// CLAUDE.md file loader
pub struct ClaudeMdLoader {
    /// Maximum file size to load
    max_file_size: u64,
}

impl ClaudeMdLoader {
    /// Create a new CLAUDE.md loader
    pub fn new() -> Self {
        Self {
            max_file_size: MAX_FILE_SIZE,
        }
    }

    /// Create a loader with custom max file size
    pub fn with_max_file_size(max_size: u64) -> Self {
        Self {
            max_file_size: max_size,
        }
    }

    /// Load and merge CLAUDE.md files from multiple paths
    pub async fn load_and_merge(&self, paths: &[PathBuf]) -> Result<Option<ClaudeMdContent>> {
        if paths.is_empty() {
            debug!("No CLAUDE.md paths provided");
            return Ok(None);
        }

        let mut contents = Vec::new();
        let mut sources = Vec::new();
        let mut total_size = 0usize;

        for path in paths {
            match self.load_file(path).await {
                Ok(Some(content)) => {
                    info!("Loaded CLAUDE.md from {:?} ({} bytes)", path, content.len());
                    total_size += content.len();
                    contents.push(content);
                    sources.push(path.clone());
                }
                Ok(None) => {
                    debug!("CLAUDE.md not found at {:?}", path);
                }
                Err(e) => {
                    warn!("Failed to load CLAUDE.md from {:?}: {}", path, e);
                }
            }
        }

        if contents.is_empty() {
            debug!("No CLAUDE.md files were successfully loaded");
            return Ok(None);
        }

        // Merge contents with separators
        let merged = if contents.len() == 1 {
            contents.into_iter().next().unwrap()
        } else {
            contents.join(MERGE_SEPARATOR)
        };

        Ok(Some(ClaudeMdContent {
            content: merged,
            sources,
            total_size,
        }))
    }

    /// Load a single CLAUDE.md file
    pub async fn load_file(&self, path: &Path) -> Result<Option<String>> {
        // Check if file exists
        if !path.exists() {
            return Ok(None);
        }

        // Check if it's actually a file
        let metadata = tokio::fs::metadata(path)
            .await
            .with_context(|| format!("Failed to get metadata for {:?}", path))?;

        if !metadata.is_file() {
            debug!("{:?} is not a file", path);
            return Ok(None);
        }

        // Check file size
        if metadata.len() > self.max_file_size {
            return Err(anyhow::anyhow!(
                "File {:?} exceeds maximum size limit ({} > {} bytes)",
                path,
                metadata.len(),
                self.max_file_size
            ));
        }

        // Read file content
        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read {:?}", path))?;

        // Remove UTF-8 BOM if present
        let content = if content.starts_with('\u{FEFF}') {
            content.chars().skip(1).collect()
        } else {
            content
        };

        // Validate content
        self.validate_content(&content)?;

        Ok(Some(content))
    }

    /// Validate CLAUDE.md content
    fn validate_content(&self, content: &str) -> Result<()> {
        // Check for suspicious patterns that might indicate code execution attempts
        let suspicious_patterns = [
            "<script",
            "javascript:",
            "eval(",
            "exec(",
            "__import__",
            "subprocess",
            "os.system",
        ];

        for pattern in &suspicious_patterns {
            if content.to_lowercase().contains(pattern) {
                return Err(anyhow::anyhow!(
                    "CLAUDE.md contains suspicious pattern: {}",
                    pattern
                ));
            }
        }

        // Check for reasonable line count (prevent abuse)
        let line_count = content.lines().count();
        if line_count > 10000 {
            return Err(anyhow::anyhow!(
                "CLAUDE.md has too many lines ({} > 10000)",
                line_count
            ));
        }

        Ok(())
    }

    /// Load from discovered paths (convenience method)
    pub async fn load_from_discovery(
        &self,
        discovery: &super::discovery::ClaudeConfigPaths,
    ) -> Result<Option<ClaudeMdContent>> {
        let paths = discovery.claude_md_paths();
        self.load_and_merge(&paths).await
    }
}

impl Default for ClaudeMdLoader {
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
    async fn test_load_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let claude_md = temp_dir.path().join("CLAUDE.md");
        
        let content = "# User Instructions\n\nAlways be helpful.";
        fs::write(&claude_md, content).await.unwrap();
        
        let loader = ClaudeMdLoader::new();
        let result = loader.load_file(&claude_md).await.unwrap();
        
        assert!(result.is_some());
        assert_eq!(result.unwrap(), content);
    }

    #[tokio::test]
    async fn test_load_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let claude_md = temp_dir.path().join("CLAUDE.md");
        
        let loader = ClaudeMdLoader::new();
        let result = loader.load_file(&claude_md).await.unwrap();
        
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_load_with_utf8_bom() {
        let temp_dir = TempDir::new().unwrap();
        let claude_md = temp_dir.path().join("CLAUDE.md");
        
        // Write with UTF-8 BOM
        let content_with_bom = "\u{FEFF}# Instructions\nTest content";
        fs::write(&claude_md, content_with_bom).await.unwrap();
        
        let loader = ClaudeMdLoader::new();
        let result = loader.load_file(&claude_md).await.unwrap();
        
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "# Instructions\nTest content");
    }

    #[tokio::test]
    async fn test_merge_multiple_files() {
        let temp_dir = TempDir::new().unwrap();
        
        let file1 = temp_dir.path().join("global.md");
        let file2 = temp_dir.path().join("project.md");
        
        fs::write(&file1, "Global instructions").await.unwrap();
        fs::write(&file2, "Project instructions").await.unwrap();
        
        let loader = ClaudeMdLoader::new();
        let result = loader.load_and_merge(&[file1.clone(), file2.clone()]).await.unwrap();
        
        assert!(result.is_some());
        let content = result.unwrap();
        assert!(content.content.contains("Global instructions"));
        assert!(content.content.contains("Project instructions"));
        assert!(content.content.contains("---"));
        assert_eq!(content.sources.len(), 2);
    }

    #[tokio::test]
    async fn test_file_size_limit() {
        let temp_dir = TempDir::new().unwrap();
        let claude_md = temp_dir.path().join("CLAUDE.md");
        
        // Create content larger than 10 bytes
        let large_content = "a".repeat(20);
        fs::write(&claude_md, &large_content).await.unwrap();
        
        let loader = ClaudeMdLoader::with_max_file_size(10);
        let result = loader.load_file(&claude_md).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum size"));
    }

    #[tokio::test]
    async fn test_suspicious_content_validation() {
        let temp_dir = TempDir::new().unwrap();
        let claude_md = temp_dir.path().join("CLAUDE.md");
        
        let suspicious_content = "Instructions\n<script>alert('xss')</script>";
        fs::write(&claude_md, suspicious_content).await.unwrap();
        
        let loader = ClaudeMdLoader::new();
        let result = loader.load_file(&claude_md).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("suspicious pattern"));
    }

    #[tokio::test]
    async fn test_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let claude_md = temp_dir.path().join("CLAUDE.md");
        
        fs::write(&claude_md, "").await.unwrap();
        
        let loader = ClaudeMdLoader::new();
        let result = loader.load_file(&claude_md).await.unwrap();
        
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "");
    }

    #[tokio::test]
    async fn test_directory_instead_of_file() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("CLAUDE.md");
        fs::create_dir(&dir_path).await.unwrap();
        
        let loader = ClaudeMdLoader::new();
        let result = loader.load_file(&dir_path).await.unwrap();
        
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_windows_line_endings() {
        let temp_dir = TempDir::new().unwrap();
        let claude_md = temp_dir.path().join("CLAUDE.md");
        
        let content = "Line 1\r\nLine 2\r\nLine 3";
        fs::write(&claude_md, content).await.unwrap();
        
        let loader = ClaudeMdLoader::new();
        let result = loader.load_file(&claude_md).await.unwrap();
        
        assert!(result.is_some());
        assert_eq!(result.unwrap(), content);
    }

    #[tokio::test]
    async fn test_load_from_discovery() {
        let temp_dir = TempDir::new().unwrap();
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir(&claude_dir).await.unwrap();
        
        let claude_md = claude_dir.join("CLAUDE.md");
        fs::write(&claude_md, "Test instructions").await.unwrap();
        
        use crate::claude::discovery::ClaudeConfigDiscovery;
        let discovery = ClaudeConfigDiscovery::with_working_dir(temp_dir.path().to_path_buf());
        let paths = discovery.discover().await.unwrap();
        
        let loader = ClaudeMdLoader::new();
        let result = loader.load_from_discovery(&paths).await.unwrap();
        
        assert!(result.is_some());
        let content = result.unwrap();
        assert_eq!(content.content, "Test instructions");
        assert_eq!(content.source_count(), 1);
    }

    #[tokio::test]
    async fn test_content_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let claude_md = temp_dir.path().join("CLAUDE.md");
        
        let test_content = "Test content";
        fs::write(&claude_md, test_content).await.unwrap();
        
        let loader = ClaudeMdLoader::new();
        let result = loader.load_and_merge(&[claude_md.clone()]).await.unwrap().unwrap();
        
        assert_eq!(result.as_str(), test_content);
        assert!(!result.is_empty());
        assert_eq!(result.source_count(), 1);
        assert_eq!(result.total_size, test_content.len());
        assert_eq!(result.sources[0], claude_md);
    }
}