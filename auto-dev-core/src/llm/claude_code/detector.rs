//! Claude Code CLI binary detection and validation
//!
//! Detects and validates the Claude Code CLI binary across different platforms
//! and installation methods.

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::process::Command;
use std::env;
use tracing::{debug, info, warn};

/// Cached detection result
static DETECTION_CACHE: Lazy<Mutex<Option<ClaudeLocation>>> = Lazy::new(|| Mutex::new(None));

/// Represents a detected Claude binary location
#[derive(Debug, Clone)]
pub struct ClaudeLocation {
    /// Path to the Claude binary
    pub path: PathBuf,
    /// Version string (e.g., "1.2.3")
    pub version: String,
    /// Full version output for debugging
    pub version_output: String,
}

/// Detection errors with helpful messages
#[derive(Debug, thiserror::Error)]
pub enum DetectionError {
    #[error("Claude Code CLI not found. Install from: https://docs.anthropic.com/en/docs/claude-code/quickstart")]
    NotFound,
    
    #[error("Claude binary found at {path} but failed to execute: {error}")]
    ExecutionFailed { path: String, error: String },
    
    #[error("Claude version {version} is too old. Minimum required: {minimum}")]
    VersionTooOld { version: String, minimum: String },
    
    #[error("Failed to parse Claude version from output: {output}")]
    VersionParseError { output: String },
}

/// Main detector for Claude Code CLI
pub struct ClaudeDetector;

impl ClaudeDetector {
    /// Detect Claude binary with caching
    pub fn detect() -> Result<ClaudeLocation> {
        // Check cache first
        if let Some(cached) = Self::get_cached() {
            debug!("Using cached Claude location: {:?}", cached.path);
            return Ok(cached);
        }
        
        // Perform detection
        let location = Self::detect_uncached()?;
        
        // Cache the result
        Self::cache_result(location.clone());
        
        Ok(location)
    }
    
    /// Clear the detection cache (useful for tests or when Claude is installed)
    pub fn clear_cache() {
        let mut cache = DETECTION_CACHE.lock().unwrap();
        *cache = None;
    }
    
    /// Get cached detection result
    fn get_cached() -> Option<ClaudeLocation> {
        DETECTION_CACHE.lock().unwrap().clone()
    }
    
    /// Cache a detection result
    fn cache_result(location: ClaudeLocation) {
        let mut cache = DETECTION_CACHE.lock().unwrap();
        *cache = Some(location);
    }
    
    /// Perform detection without caching
    fn detect_uncached() -> Result<ClaudeLocation> {
        // Search order as specified in PRP
        
        // 1. Check PATH environment variable
        if let Some(location) = Self::detect_in_path()? {
            info!("Found Claude in PATH: {:?}", location.path);
            return Ok(location);
        }
        
        // 2. Check NPM global installation
        if let Some(location) = Self::detect_npm_global()? {
            info!("Found Claude via NPM: {:?}", location.path);
            return Ok(location);
        }
        
        // 3. Check native installation
        if let Some(location) = Self::detect_native_install()? {
            info!("Found Claude native installation: {:?}", location.path);
            return Ok(location);
        }
        
        // 4. Check user-specified custom path (via environment variable)
        if let Some(location) = Self::detect_custom_path()? {
            info!("Found Claude at custom path: {:?}", location.path);
            return Ok(location);
        }
        
        Err(DetectionError::NotFound.into())
    }
    
    /// Detect Claude in PATH
    fn detect_in_path() -> Result<Option<ClaudeLocation>> {
        let claude_cmd = if cfg!(windows) { "claude.cmd" } else { "claude" };
        
        // Use which crate logic or manual PATH search
        if let Ok(path_var) = env::var("PATH") {
            let separator = if cfg!(windows) { ';' } else { ':' };
            
            for path_dir in path_var.split(separator) {
                let claude_path = Path::new(path_dir).join(claude_cmd);
                if claude_path.exists() {
                    if let Ok(location) = Self::validate_binary(&claude_path) {
                        return Ok(Some(location));
                    }
                }
                
                // On Windows, also check for .exe
                if cfg!(windows) {
                    let claude_exe = Path::new(path_dir).join("claude.exe");
                    if claude_exe.exists() {
                        if let Ok(location) = Self::validate_binary(&claude_exe) {
                            return Ok(Some(location));
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// Detect NPM global installation
    fn detect_npm_global() -> Result<Option<ClaudeLocation>> {
        let npm_paths = if cfg!(windows) {
            vec![
                // Windows NPM paths
                env::var("APPDATA").ok()
                    .map(|p| Path::new(&p).join("npm").join("claude.cmd")),
                env::var("APPDATA").ok()
                    .map(|p| Path::new(&p).join("npm").join("claude.exe")),
            ]
        } else {
            vec![
                // Unix NPM paths
                Some(PathBuf::from("/usr/local/bin/claude")),
                Some(PathBuf::from("/usr/bin/claude")),
                env::var("HOME").ok()
                    .map(|h| Path::new(&h).join(".npm-global").join("bin").join("claude")),
                env::var("HOME").ok()
                    .map(|h| Path::new(&h).join(".local").join("bin").join("claude")),
            ]
        };
        
        for path_opt in npm_paths.into_iter().flatten() {
            if path_opt.exists() {
                if let Ok(location) = Self::validate_binary(&path_opt) {
                    return Ok(Some(location));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Detect native installation
    fn detect_native_install() -> Result<Option<ClaudeLocation>> {
        let native_paths = if cfg!(windows) {
            vec![
                // Windows native paths
                env::var("LOCALAPPDATA").ok()
                    .map(|p| Path::new(&p).join("Programs").join("Claude").join("claude.exe")),
                env::var("ProgramFiles").ok()
                    .map(|p| Path::new(&p).join("Claude").join("claude.exe")),
            ]
        } else if cfg!(target_os = "macos") {
            vec![
                // macOS native paths
                Some(PathBuf::from("/Applications/Claude.app/Contents/MacOS/claude")),
                env::var("HOME").ok()
                    .map(|h| Path::new(&h).join("Applications").join("Claude.app")
                        .join("Contents").join("MacOS").join("claude")),
            ]
        } else {
            vec![
                // Linux native paths
                Some(PathBuf::from("/opt/claude/claude")),
                Some(PathBuf::from("/usr/local/claude/claude")),
            ]
        };
        
        for path_opt in native_paths.into_iter().flatten() {
            if path_opt.exists() {
                if let Ok(location) = Self::validate_binary(&path_opt) {
                    return Ok(Some(location));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Check for user-specified custom path
    fn detect_custom_path() -> Result<Option<ClaudeLocation>> {
        if let Ok(custom_path) = env::var("CLAUDE_CODE_PATH") {
            let path = PathBuf::from(custom_path);
            if path.exists() {
                if let Ok(location) = Self::validate_binary(&path) {
                    return Ok(Some(location));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Validate that a binary is Claude and get its version
    fn validate_binary(path: &Path) -> Result<ClaudeLocation> {
        debug!("Validating potential Claude binary at: {:?}", path);
        
        // Run --version command
        let output = Command::new(path)
            .arg("--version")
            .output()
            .with_context(|| format!("Failed to execute {:?}", path))?;
        
        // Check if command succeeded
        if !output.status.success() && output.stdout.is_empty() && output.stderr.is_empty() {
            return Err(DetectionError::ExecutionFailed {
                path: path.display().to_string(),
                error: "Command failed with no output".to_string(),
            }.into());
        }
        
        // Parse version from output (could be in stdout or stderr)
        let version_output = if !output.stdout.is_empty() {
            String::from_utf8_lossy(&output.stdout).to_string()
        } else {
            String::from_utf8_lossy(&output.stderr).to_string()
        };
        
        // Check if this is actually Claude
        if !version_output.to_lowercase().contains("claude") {
            return Err(DetectionError::ExecutionFailed {
                path: path.display().to_string(),
                error: "Not a Claude binary".to_string(),
            }.into());
        }
        
        // Parse version number (looking for patterns like "1.2.3" or "v1.2.3")
        let version = Self::parse_version(&version_output)?;
        
        // Check minimum version
        if !Self::is_version_supported(&version) {
            return Err(DetectionError::VersionTooOld {
                version: version.clone(),
                minimum: "1.0.0".to_string(),
            }.into());
        }
        
        Ok(ClaudeLocation {
            path: path.to_path_buf(),
            version,
            version_output,
        })
    }
    
    /// Parse version string from --version output
    fn parse_version(output: &str) -> Result<String> {
        // Look for semantic version pattern
        let version_regex = regex::Regex::new(r"(\d+\.\d+\.\d+)").unwrap();
        
        if let Some(captures) = version_regex.captures(output) {
            if let Some(version) = captures.get(1) {
                return Ok(version.as_str().to_string());
            }
        }
        
        // Fallback: if output contains "Claude" and looks like a version, accept it
        if output.contains("Claude") && output.contains(".") {
            // Try to extract anything that looks like a version
            for word in output.split_whitespace() {
                if word.chars().any(|c| c.is_ascii_digit()) && word.contains('.') {
                    return Ok(word.trim_start_matches('v').to_string());
                }
            }
        }
        
        Err(DetectionError::VersionParseError {
            output: output.to_string(),
        }.into())
    }
    
    /// Check if version meets minimum requirements
    fn is_version_supported(version: &str) -> bool {
        // Parse major.minor.patch
        let parts: Vec<u32> = version
            .split('.')
            .filter_map(|p| p.parse().ok())
            .collect();
        
        if parts.is_empty() {
            return false;
        }
        
        // Minimum version is 1.0.0
        if parts[0] >= 1 {
            return true;
        }
        
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use tempfile::TempDir;
    
    #[test]
    fn test_version_parsing() {
        let cases = vec![
            ("Claude Code 1.2.3", "1.2.3"),
            ("claude version 2.0.0", "2.0.0"),
            ("v1.5.0", "1.5.0"),
            ("Claude CLI v1.0.0-beta", "1.0.0"),
        ];
        
        for (input, expected) in cases {
            let result = ClaudeDetector::parse_version(input).unwrap();
            assert_eq!(result, expected, "Failed to parse: {}", input);
        }
    }
    
    #[test]
    fn test_version_support() {
        assert!(ClaudeDetector::is_version_supported("1.0.0"));
        assert!(ClaudeDetector::is_version_supported("1.2.3"));
        assert!(ClaudeDetector::is_version_supported("2.0.0"));
        assert!(!ClaudeDetector::is_version_supported("0.9.0"));
        assert!(!ClaudeDetector::is_version_supported("0.1.0"));
    }
    
    #[test]
    fn test_cache_operations() {
        // Clear cache
        ClaudeDetector::clear_cache();
        assert!(ClaudeDetector::get_cached().is_none());
        
        // Cache a result
        let location = ClaudeLocation {
            path: PathBuf::from("/test/claude"),
            version: "1.2.3".to_string(),
            version_output: "Claude Code 1.2.3".to_string(),
        };
        
        ClaudeDetector::cache_result(location.clone());
        
        // Verify cached
        let cached = ClaudeDetector::get_cached().unwrap();
        assert_eq!(cached.version, "1.2.3");
        
        // Clear again
        ClaudeDetector::clear_cache();
        assert!(ClaudeDetector::get_cached().is_none());
    }
    
    #[test]
    fn test_custom_path_env() {
        // Create a temporary directory and file
        let temp_dir = TempDir::new().unwrap();
        let claude_path = temp_dir.path().join("claude");
        
        // Create a mock executable
        fs::write(&claude_path, "#!/bin/sh\necho 'Claude Code 1.5.0'").unwrap();
        
        // Set custom path env var
        unsafe {
            env::set_var("CLAUDE_CODE_PATH", claude_path.to_str().unwrap());
        }
        
        // Note: This would need actual binary for full test
        // This test demonstrates the structure
        
        unsafe {
            env::remove_var("CLAUDE_CODE_PATH");
        }
    }
}