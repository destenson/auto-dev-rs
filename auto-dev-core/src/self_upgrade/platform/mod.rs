//! Platform-specific binary replacement

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Platform-specific binary swapper
pub struct BinarySwapper {
    current_binary: PathBuf,
}

impl BinarySwapper {
    pub fn new(current_binary: PathBuf) -> Self {
        Self { current_binary }
    }
    
    /// Swap the current binary with a new one
    pub async fn swap(&self, new_binary: &Path) -> Result<()> {
        #[cfg(unix)]
        return unix::swap_binary(&self.current_binary, new_binary);
        
        #[cfg(windows)]
        return windows::swap_binary(&self.current_binary, new_binary);
        
        #[cfg(not(any(unix, windows)))]
        return Err(anyhow::anyhow!("Unsupported platform"));
    }
    
    /// Restart the application with arguments
    pub fn restart_with_args(&self, args: &[&str]) -> Result<()> {
        #[cfg(unix)]
        return unix::restart_with_args(&self.current_binary, args);
        
        #[cfg(windows)]
        return windows::restart_with_args(&self.current_binary, args);
        
        #[cfg(not(any(unix, windows)))]
        return Err(anyhow::anyhow!("Unsupported platform"));
    }
}