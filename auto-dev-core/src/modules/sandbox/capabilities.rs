//! Capability-based permission system for sandboxed modules

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// Represents a capability that can be granted to a module
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Filesystem access capabilities
    FileSystem(FileSystemCapability),
    /// Network access capabilities
    Network(NetworkCapability),
    /// Resource limit capabilities
    Resource(ResourceCapability),
    /// Inter-module communication capabilities
    Module(ModuleCapability),
    /// System call capabilities
    System(SystemCapability),
}

/// Filesystem access permissions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct FileSystemCapability {
    pub operation: FileOperation,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FileOperation {
    Read,
    Write,
    Delete,
    List,
}

/// Network access permissions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct NetworkCapability {
    pub protocol: NetworkProtocol,
    pub host: String,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NetworkProtocol {
    Http,
    Https,
    Tcp,
    Udp,
}

/// Resource limit specifications
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ResourceCapability {
    pub resource_type: ResourceType,
    pub limit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Memory,      // in bytes
    CpuPercent,  // percentage (0-100)
    Threads,     // number of threads
    FileHandles, // number of open files
}

/// Inter-module communication permissions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ModuleCapability {
    pub operation: ModuleOperation,
    pub target_module: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModuleOperation {
    Call,
    Subscribe,
    Publish,
}

/// System call permissions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SystemCapability {
    pub operation: SystemOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SystemOperation {
    GetTime,
    GetEnvironment,
    SpawnProcess,
}

/// A set of capabilities granted to a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySet {
    capabilities: HashSet<Capability>,
    inherit_from: Option<String>, // Parent capability set
}

impl CapabilitySet {
    /// Create an empty capability set
    pub fn new() -> Self {
        Self {
            capabilities: HashSet::new(),
            inherit_from: None,
        }
    }

    /// Create a capability set with default minimal permissions
    pub fn minimal() -> Self {
        let mut set = Self::new();
        // Only allow getting time and basic memory allocation
        set.add(Capability::System(SystemCapability {
            operation: SystemOperation::GetTime,
        }));
        set.add(Capability::Resource(ResourceCapability {
            resource_type: ResourceType::Memory,
            limit: 10 * 1024 * 1024, // 10MB
        }));
        set
    }

    /// Create a capability set from a string specification
    pub fn from_spec(spec: &str) -> Result<Capability> {
        let parts: Vec<&str> = spec.split(':').collect();
        
        match parts.as_slice() {
            ["filesystem", "read", path] => {
                Ok(Capability::FileSystem(FileSystemCapability {
                    operation: FileOperation::Read,
                    path: PathBuf::from(path),
                }))
            }
            ["filesystem", "write", path] => {
                Ok(Capability::FileSystem(FileSystemCapability {
                    operation: FileOperation::Write,
                    path: PathBuf::from(path),
                }))
            }
            ["network", protocol, host] => {
                let protocol = match *protocol {
                    "http" => NetworkProtocol::Http,
                    "https" => NetworkProtocol::Https,
                    "tcp" => NetworkProtocol::Tcp,
                    "udp" => NetworkProtocol::Udp,
                    _ => return Err(anyhow::anyhow!("Unknown protocol: {}", protocol)),
                };
                Ok(Capability::Network(NetworkCapability {
                    protocol,
                    host: host.to_string(),
                    port: None,
                }))
            }
            ["memory", "limit", size] => {
                let limit = parse_size(size)?;
                Ok(Capability::Resource(ResourceCapability {
                    resource_type: ResourceType::Memory,
                    limit,
                }))
            }
            ["cpu", "limit", percent] => {
                let limit = percent.trim_end_matches('%').parse::<u64>()?;
                Ok(Capability::Resource(ResourceCapability {
                    resource_type: ResourceType::CpuPercent,
                    limit,
                }))
            }
            ["module", "call", target] => {
                Ok(Capability::Module(ModuleCapability {
                    operation: ModuleOperation::Call,
                    target_module: target.to_string(),
                }))
            }
            _ => Err(anyhow::anyhow!("Invalid capability specification: {}", spec)),
        }
    }

    /// Add a capability to the set
    pub fn add(&mut self, capability: Capability) {
        self.capabilities.insert(capability);
    }

    /// Remove a capability from the set
    pub fn remove(&mut self, capability: &Capability) -> bool {
        self.capabilities.remove(capability)
    }

    /// Check if a capability is in the set
    pub fn contains(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Check if a capability is allowed (with pattern matching)
    pub fn is_allowed(&self, requested: &Capability) -> bool {
        // Direct match
        if self.capabilities.contains(requested) {
            return true;
        }

        // Check for broader permissions
        match requested {
            Capability::FileSystem(fs_cap) => {
                self.check_filesystem_permission(fs_cap)
            }
            Capability::Network(net_cap) => {
                self.check_network_permission(net_cap)
            }
            _ => false,
        }
    }

    fn check_filesystem_permission(&self, requested: &FileSystemCapability) -> bool {
        for capability in &self.capabilities {
            if let Capability::FileSystem(fs_cap) = capability {
                if fs_cap.operation == requested.operation {
                    // Check if the granted path is a parent of the requested path
                    if requested.path.starts_with(&fs_cap.path) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn check_network_permission(&self, requested: &NetworkCapability) -> bool {
        for capability in &self.capabilities {
            if let Capability::Network(net_cap) = capability {
                if net_cap.protocol == requested.protocol {
                    // Check if the host matches (could add wildcard support)
                    if net_cap.host == requested.host || net_cap.host == "*" {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Get all capabilities
    pub fn all(&self) -> &HashSet<Capability> {
        &self.capabilities
    }
}

/// Manages capabilities for multiple modules
pub struct CapabilityManager {
    granted: CapabilitySet,
}

impl CapabilityManager {
    /// Create a new capability manager
    pub fn new(granted: CapabilitySet) -> Self {
        Self { granted }
    }

    /// Check if a capability is allowed
    pub fn is_allowed(&self, capability: &Capability) -> bool {
        self.granted.is_allowed(capability)
    }

    /// Grant a new capability
    pub fn grant(&mut self, capability: Capability) {
        self.granted.add(capability);
    }

    /// Revoke a capability
    pub fn revoke(&mut self, capability: &Capability) -> bool {
        self.granted.remove(capability)
    }

    /// Get all granted capabilities
    pub fn granted_capabilities(&self) -> &HashSet<Capability> {
        self.granted.all()
    }
}

/// Parse size strings like "100MB", "1GB", etc.
fn parse_size(size_str: &str) -> Result<u64> {
    let size_str = size_str.to_uppercase();
    
    if let Some(mb_str) = size_str.strip_suffix("MB") {
        let num = mb_str.parse::<u64>()?;
        Ok(num * 1024 * 1024)
    } else if let Some(gb_str) = size_str.strip_suffix("GB") {
        let num = gb_str.parse::<u64>()?;
        Ok(num * 1024 * 1024 * 1024)
    } else if let Some(kb_str) = size_str.strip_suffix("KB") {
        let num = kb_str.parse::<u64>()?;
        Ok(num * 1024)
    } else {
        // Assume bytes if no suffix
        size_str.parse::<u64>()
            .map_err(|e| anyhow::anyhow!("Invalid size specification: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_from_spec() {
        let cap = CapabilitySet::from_spec("filesystem:read:/docs").unwrap();
        assert!(matches!(cap, Capability::FileSystem(_)));

        let cap = CapabilitySet::from_spec("network:http:localhost").unwrap();
        assert!(matches!(cap, Capability::Network(_)));

        let cap = CapabilitySet::from_spec("memory:limit:100MB").unwrap();
        assert!(matches!(cap, Capability::Resource(_)));

        let cap = CapabilitySet::from_spec("cpu:limit:50%").unwrap();
        assert!(matches!(cap, Capability::Resource(_)));

        let cap = CapabilitySet::from_spec("module:call:parser").unwrap();
        assert!(matches!(cap, Capability::Module(_)));
    }

    #[test]
    fn test_filesystem_permission_inheritance() {
        let mut set = CapabilitySet::new();
        set.add(Capability::FileSystem(FileSystemCapability {
            operation: FileOperation::Read,
            path: PathBuf::from("/docs"),
        }));

        let requested = Capability::FileSystem(FileSystemCapability {
            operation: FileOperation::Read,
            path: PathBuf::from("/docs/api/reference.md"),
        });

        assert!(set.is_allowed(&requested));
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("100MB").unwrap(), 100 * 1024 * 1024);
        assert_eq!(parse_size("1GB").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_size("512KB").unwrap(), 512 * 1024);
        assert_eq!(parse_size("1000").unwrap(), 1000);
    }
}