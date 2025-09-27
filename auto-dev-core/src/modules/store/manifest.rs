//! Module Manifest Format and Parser
//!
//! Defines the TOML manifest format for modules in the local store.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Module manifest that describes a module in the store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleManifest {
    /// Module metadata
    pub module: ModuleMetadata,
    /// Module capabilities
    pub capabilities: ModuleCapabilities,
    /// Compatibility requirements
    pub compatibility: CompatibilityRequirements,
    /// Verification information
    pub verification: Option<VerificationInfo>,
    /// Module dependencies
    pub dependencies: Option<Vec<ModuleDependency>>,
}

/// Basic module metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetadata {
    /// Module name
    pub name: String,
    /// Module version
    pub version: String,
    /// Short description
    pub description: String,
    /// Module authors
    pub authors: Vec<String>,
    /// License identifier
    pub license: String,
    /// Module category
    pub category: String,
    /// Repository URL (optional)
    pub repository: Option<String>,
    /// Keywords for search
    pub keywords: Option<Vec<String>>,
    /// Creation date
    pub created: Option<DateTime<Utc>>,
    /// Last updated
    pub updated: Option<DateTime<Utc>>,
}

/// Module capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleCapabilities {
    /// What the module provides
    pub provides: Vec<String>,
    /// What capabilities the module requires
    pub requires: Vec<String>,
    /// Optional capabilities
    pub optional: Option<Vec<String>>,
}

/// Compatibility requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityRequirements {
    /// Required auto-dev version
    pub auto_dev_version: String,
    /// Supported platforms
    pub platform: Vec<String>,
    /// Required Rust version (if applicable)
    pub rust_version: Option<String>,
    /// Required features
    pub features: Option<Vec<String>>,
}

/// Verification information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationInfo {
    /// SHA256 checksum of module contents
    pub checksum: String,
    /// Digital signature (optional)
    pub signature: Option<String>,
    /// Signing key ID (optional)
    pub key_id: Option<String>,
    /// Trust level
    pub trust_level: Option<TrustLevel>,
}

/// Module dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDependency {
    /// Dependency name
    pub name: String,
    /// Version requirement
    pub version: String,
    /// Optional dependency
    pub optional: bool,
}

/// Trust levels for modules
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrustLevel {
    /// Built-in core module
    Core,
    /// Verified by team
    Verified,
    /// Community trusted
    Trusted,
    /// Known module
    Known,
    /// Unknown/unverified
    Unknown,
}

impl Default for TrustLevel {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Module manifest parser
pub struct ManifestParser;

impl ManifestParser {
    /// Parse manifest from TOML string
    pub fn parse(content: &str) -> Result<ModuleManifest> {
        let mut manifest: ModuleManifest = toml::from_str(content)
            .context("Failed to parse module manifest")?;
        
        // Set timestamps if not provided
        let now = Utc::now();
        if manifest.module.created.is_none() {
            manifest.module.created = Some(now);
        }
        if manifest.module.updated.is_none() {
            manifest.module.updated = Some(now);
        }
        
        // Validate manifest
        Self::validate(&manifest)?;
        
        Ok(manifest)
    }

    /// Parse manifest from file path
    pub fn parse_from_path(path: &Path) -> Result<ModuleManifest> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read manifest from {:?}", path))?;
        Self::parse(&content)
    }

    /// Validate manifest
    fn validate(manifest: &ModuleManifest) -> Result<()> {
        // Validate module name
        if manifest.module.name.is_empty() {
            anyhow::bail!("Module name cannot be empty");
        }
        
        // Validate version format
        if !Self::is_valid_version(&manifest.module.version) {
            anyhow::bail!("Invalid version format: {}", manifest.module.version);
        }
        
        // Validate category
        if !Self::is_valid_category(&manifest.module.category) {
            anyhow::bail!("Invalid module category: {}", manifest.module.category);
        }
        
        // Validate capabilities
        if manifest.capabilities.provides.is_empty() {
            anyhow::bail!("Module must provide at least one capability");
        }
        
        Ok(())
    }

    /// Check if version string is valid semver
    fn is_valid_version(version: &str) -> bool {
        let parts: Vec<&str> = version.split('.').collect();
        parts.len() == 3 && parts.iter().all(|p| p.parse::<u32>().is_ok())
    }

    /// Check if category is valid
    fn is_valid_category(category: &str) -> bool {
        const VALID_CATEGORIES: &[&str] = &[
            "parser",
            "generator", 
            "analyzer",
            "formatter",
            "validator",
            "integration",
            "optimizer",
            "utility",
            "testing",
            "documentation",
        ];
        VALID_CATEGORIES.contains(&category)
    }
}

/// Create a sample manifest for testing
impl ModuleManifest {
    pub fn example() -> Self {
        Self {
            module: ModuleMetadata {
                name: "python-parser".to_string(),
                version: "1.0.0".to_string(),
                description: "Python code parser module".to_string(),
                authors: vec!["auto-dev-rs".to_string()],
                license: "MIT".to_string(),
                category: "parser".to_string(),
                repository: Some("local://module_store/python-parser".to_string()),
                keywords: Some(vec!["python".to_string(), "parser".to_string(), "ast".to_string()]),
                created: Some(Utc::now()),
                updated: Some(Utc::now()),
            },
            capabilities: ModuleCapabilities {
                provides: vec!["parser:python".to_string()],
                requires: vec!["filesystem:read".to_string()],
                optional: Some(vec!["cache:write".to_string()]),
            },
            compatibility: CompatibilityRequirements {
                auto_dev_version: ">=0.5.0".to_string(),
                platform: vec!["wasm".to_string(), "native".to_string()],
                rust_version: Some("1.75.0".to_string()),
                features: None,
            },
            verification: Some(VerificationInfo {
                checksum: "sha256:abcdef1234567890".to_string(),
                signature: None,
                key_id: None,
                trust_level: Some(TrustLevel::Unknown),
            }),
            dependencies: None,
        }
    }

    /// Convert to TOML string
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self)
            .context("Failed to serialize manifest to TOML")
    }

    /// Save to file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = self.to_toml()?;
        fs::write(path, content)
            .with_context(|| format!("Failed to write manifest to {:?}", path))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_manifest() {
        let manifest = ModuleManifest::example();
        assert_eq!(manifest.module.name, "python-parser");
        assert_eq!(manifest.module.version, "1.0.0");
        assert_eq!(manifest.module.category, "parser");
    }

    #[test]
    fn test_manifest_serialization() {
        let manifest = ModuleManifest::example();
        let toml = manifest.to_toml().unwrap();
        assert!(toml.contains("[module]"));
        assert!(toml.contains("name = \"python-parser\""));
    }

    #[test]
    fn test_parse_manifest() {
        let toml = r#"
[module]
name = "test-module"
version = "0.1.0"
description = "Test module"
authors = ["test"]
license = "MIT"
category = "utility"

[capabilities]
provides = ["test:capability"]
requires = ["base:capability"]

[compatibility]
auto_dev_version = ">=0.1.0"
platform = ["native"]
        "#;

        let manifest = ManifestParser::parse(toml).unwrap();
        assert_eq!(manifest.module.name, "test-module");
    }

    #[test]
    fn test_version_validation() {
        assert!(ManifestParser::is_valid_version("1.0.0"));
        assert!(ManifestParser::is_valid_version("0.1.0"));
        assert!(!ManifestParser::is_valid_version("1.0"));
        assert!(!ManifestParser::is_valid_version("invalid"));
    }

    #[test]
    fn test_category_validation() {
        assert!(ManifestParser::is_valid_category("parser"));
        assert!(ManifestParser::is_valid_category("generator"));
        assert!(!ManifestParser::is_valid_category("invalid"));
    }
}