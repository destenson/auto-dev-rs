#![allow(unused)]
//! Windows specific binary replacement

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;
use tracing::{info, warn};

/// Swap binary on Windows systems
pub fn swap_binary(current: &Path, new: &Path) -> Result<()> {
    // Windows doesn't allow replacing a running executable directly
    // Use a batch script to perform the replacement after a delay

    let batch_content = format!(
        r#"@echo off
timeout /t 2 /nobreak > nul
move /y "{}" "{}.old" > nul 2>&1
copy /y "{}" "{}" > nul
del "{}.old" > nul 2>&1
start "" "{}"
del "%~f0"
"#,
        current.display(),
        current.display(),
        new.display(),
        current.display(),
        current.display(),
        current.display()
    );

    let batch_file = current.with_extension("upgrade.bat");
    std::fs::write(&batch_file, batch_content)?;

    // Execute the batch file
    Command::new("cmd")
        .args(&["/c", "start", "/min", batch_file.to_str().unwrap()])
        .spawn()
        .context("Failed to start upgrade batch file")?;

    info!("Upgrade batch file started, application will restart shortly");

    // Exit current process
    std::process::exit(0);
}

/// Restart the application
pub fn restart_with_args(binary: &Path, args: &[&str]) -> Result<()> {
    // Start new process
    Command::new(binary).args(args).spawn().context("Failed to start new process")?;

    // Exit current process
    std::process::exit(0);
}
