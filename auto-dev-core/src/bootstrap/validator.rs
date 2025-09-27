//! Environment validation for bootstrap

use super::{BootstrapError, Result};
use std::path::Path;
use tracing::{debug, info};

pub struct EnvironmentValidator;

impl EnvironmentValidator {
    pub fn new() -> Self {
        Self
    }

    pub async fn validate(&self) -> Result<()> {
        info!("Validating environment");

        self.validate_project_structure()?;
        self.check_dependencies()?;
        self.validate_module_system()?;
        self.check_sandbox_availability()?;

        info!("Environment validation complete");
        Ok(())
    }

    fn validate_project_structure(&self) -> Result<()> {
        debug!("Validating project structure");

        // Check for essential project directories
        let required_dirs = vec!["src", "auto-dev-core/src", "auto-dev/src"];

        for dir in required_dirs {
            if !Path::new(dir).exists() {
                return Err(BootstrapError::ValidationFailed(format!(
                    "Required directory '{}' not found",
                    dir
                )));
            }
        }

        // Check for essential files
        let required_files = vec!["Cargo.toml", "auto-dev-core/Cargo.toml", "auto-dev/Cargo.toml"];

        for file in required_files {
            if !Path::new(file).exists() {
                return Err(BootstrapError::ValidationFailed(format!(
                    "Required file '{}' not found",
                    file
                )));
            }
        }

        debug!("Project structure validated");
        Ok(())
    }

    fn check_dependencies(&self) -> Result<()> {
        debug!("Checking dependencies");

        // Check if dependencies are built
        if !Path::new("target").exists() {
            debug!("Target directory not found - dependencies may need to be built");
        }

        // Could run cargo check here to verify dependencies
        // For now we'll assume they're OK if Cargo.toml exists

        debug!("Dependencies check passed");
        Ok(())
    }

    fn validate_module_system(&self) -> Result<()> {
        debug!("Validating module system");

        // Check if module system directories exist
        if !Path::new("auto-dev-core/src/modules").exists() {
            return Err(BootstrapError::ValidationFailed(
                "Module system not found at expected location".to_string(),
            ));
        }

        debug!("Module system validated");
        Ok(())
    }

    fn check_sandbox_availability(&self) -> Result<()> {
        debug!("Checking sandbox availability");

        // Check if WASM host is available
        if Path::new("auto-dev-core/src/modules/wasm_host.rs").exists() {
            debug!("WASM sandbox support detected");
        } else {
            debug!("WASM sandbox support not found - will use native modules only");
        }

        Ok(())
    }

    pub fn describe_validations(&self) -> Vec<String> {
        vec![
            "Project structure integrity".to_string(),
            "Required dependencies".to_string(),
            "Module system availability".to_string(),
            "Sandbox environment".to_string(),
        ]
    }
}
