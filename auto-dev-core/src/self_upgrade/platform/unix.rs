//! Unix/Linux/macOS specific binary replacement

use anyhow::{Context, Result};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use crate::info;

/// Swap binary on Unix systems
pub fn swap_binary(current: &Path, new: &Path) -> Result<()> {
    // Get current permissions
    let metadata = std::fs::metadata(current)?;
    let permissions = metadata.permissions();

    // Create temp backup
    let backup = format!("{}.old", current.display());
    std::fs::rename(current, &backup).context("Failed to move current binary")?;

    // Move new binary into place
    std::fs::copy(new, current).context("Failed to copy new binary")?;

    // Restore permissions
    std::fs::set_permissions(current, permissions)?;

    // Remove backup
    std::fs::remove_file(&backup).ok();

    info!("Binary swapped successfully");
    Ok(())
}

/// Restart with exec system call
pub fn restart_with_args(binary: &Path, args: &[&str]) -> Result<()> {
    use std::os::unix::process::CommandExt;

    let mut cmd = Command::new(binary);
    cmd.args(args);

    // This replaces the current process
    let err = cmd.exec();

    // If we get here, exec failed
    Err(anyhow::anyhow!("Failed to exec: {}", err))
}
