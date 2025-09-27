use std::collections::HashMap;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::{SelfTestError, sandbox_env::TestSandbox};

/// Checks API compatibility of modifications
pub struct CompatibilityChecker {
    known_interfaces: HashMap<String, InterfaceSignature>,
}

impl CompatibilityChecker {
    pub fn new() -> Self {
        Self {
            known_interfaces: HashMap::new(),
        }
    }
    
    /// Check interfaces for breaking changes
    pub async fn check_interfaces(&self, sandbox: &TestSandbox) -> Result<Vec<InterfaceChange>, SelfTestError> {
        info!("Checking interface compatibility");
        
        let mut changes = Vec::new();
        
        // Check public API surface
        let api_check = sandbox.run_command("cargo", &["public-api", "diff"]).await;
        
        if let Ok(result) = api_check {
            if !result.success {
                // Parse API differences
                changes.extend(self.parse_api_changes(&result.output));
            }
        }
        
        // Check module interfaces
        changes.extend(self.check_module_interfaces(sandbox).await?);
        
        // Check configuration compatibility
        changes.extend(self.check_config_compatibility(sandbox).await?);
        
        Ok(changes)
    }
    
    /// Check module interface compatibility
    async fn check_module_interfaces(&self, sandbox: &TestSandbox) -> Result<Vec<InterfaceChange>, SelfTestError> {
        let mut changes = Vec::new();
        
        // Check if module trait definitions have changed
        let module_check = sandbox.run_command(
            "cargo",
            &["test", "--", "module_interface_compatibility"]
        ).await;
        
        if let Ok(result) = module_check {
            if !result.success {
                changes.push(InterfaceChange {
                    interface_name: "ModuleInterface".to_string(),
                    change_type: ChangeType::Modified,
                    severity: Severity::Breaking,
                    details: "Module interface trait has been modified".to_string(),
                });
            }
        }
        
        Ok(changes)
    }
    
    /// Check configuration format compatibility
    async fn check_config_compatibility(&self, sandbox: &TestSandbox) -> Result<Vec<InterfaceChange>, SelfTestError> {
        let mut changes = Vec::new();
        
        // Check if config structures can still deserialize old formats
        let config_test = sandbox.run_command(
            "cargo",
            &["test", "--", "config_backward_compat"]
        ).await;
        
        if let Ok(result) = config_test {
            if !result.success {
                changes.push(InterfaceChange {
                    interface_name: "Configuration".to_string(),
                    change_type: ChangeType::Breaking,
                    severity: Severity::Critical,
                    details: "Configuration format is not backward compatible".to_string(),
                });
            }
        }
        
        Ok(changes)
    }
    
    /// Parse API changes from output
    fn parse_api_changes(&self, output: &str) -> Vec<InterfaceChange> {
        let mut changes = Vec::new();
        
        for line in output.lines() {
            if line.contains("BREAKING") {
                changes.push(InterfaceChange {
                    interface_name: self.extract_interface_name(line),
                    change_type: ChangeType::Breaking,
                    severity: Severity::Breaking,
                    details: line.to_string(),
                });
            } else if line.contains("MODIFIED") {
                changes.push(InterfaceChange {
                    interface_name: self.extract_interface_name(line),
                    change_type: ChangeType::Modified,
                    severity: Severity::Minor,
                    details: line.to_string(),
                });
            } else if line.contains("ADDED") {
                changes.push(InterfaceChange {
                    interface_name: self.extract_interface_name(line),
                    change_type: ChangeType::Added,
                    severity: Severity::Safe,
                    details: line.to_string(),
                });
            }
        }
        
        changes
    }
    
    fn extract_interface_name(&self, line: &str) -> String {
        // Extract interface name from change description
        line.split_whitespace()
            .nth(1)
            .unwrap_or("Unknown")
            .to_string()
    }
    
    /// Store current interfaces as baseline
    pub async fn capture_baseline(&mut self, sandbox: &TestSandbox) -> Result<(), SelfTestError> {
        info!("Capturing interface baseline");
        
        // Capture public API signatures
        let api_dump = sandbox.run_command("cargo", &["public-api", "dump"]).await;
        
        if let Ok(result) = api_dump {
            self.parse_interface_signatures(&result.output);
        }
        
        Ok(())
    }
    
    fn parse_interface_signatures(&mut self, output: &str) {
        for line in output.lines() {
            if let Some((name, signature)) = self.parse_signature_line(line) {
                self.known_interfaces.insert(name, signature);
            }
        }
    }
    
    fn parse_signature_line(&self, line: &str) -> Option<(String, InterfaceSignature)> {
        // Parse interface signature from output
        // This is a simplified version - real implementation would be more sophisticated
        if line.starts_with("pub ") {
            let name = line.split_whitespace().nth(2)?.to_string();
            let signature = InterfaceSignature {
                name: name.clone(),
                visibility: "public".to_string(),
                kind: self.detect_interface_kind(line),
                signature_hash: self.hash_signature(line),
            };
            Some((name, signature))
        } else {
            None
        }
    }
    
    fn detect_interface_kind(&self, line: &str) -> InterfaceKind {
        if line.contains("trait") {
            InterfaceKind::Trait
        } else if line.contains("struct") {
            InterfaceKind::Struct
        } else if line.contains("enum") {
            InterfaceKind::Enum
        } else if line.contains("fn") {
            InterfaceKind::Function
        } else {
            InterfaceKind::Other
        }
    }
    
    fn hash_signature(&self, signature: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        signature.hash(&mut hasher);
        hasher.finish()
    }
}

/// Represents a change to an interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceChange {
    pub interface_name: String,
    pub change_type: ChangeType,
    pub severity: Severity,
    pub details: String,
}

impl InterfaceChange {
    pub fn is_breaking(&self) -> bool {
        matches!(self.severity, Severity::Breaking | Severity::Critical)
    }
    
    pub fn description(&self) -> String {
        format!("{}: {} ({})", self.interface_name, self.details, self.severity)
    }
}

/// Type of interface change
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ChangeType {
    Added,
    Modified,
    Removed,
    Breaking,
}

/// Severity of the change
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Severity {
    Safe,
    Minor,
    Breaking,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Safe => write!(f, "Safe"),
            Severity::Minor => write!(f, "Minor"),
            Severity::Breaking => write!(f, "Breaking"),
            Severity::Critical => write!(f, "Critical"),
        }
    }
}

/// Stored interface signature
#[derive(Debug, Clone, Serialize, Deserialize)]
struct InterfaceSignature {
    name: String,
    visibility: String,
    kind: InterfaceKind,
    signature_hash: u64,
}

/// Kind of interface
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum InterfaceKind {
    Trait,
    Struct,
    Enum,
    Function,
    Other,
}