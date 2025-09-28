//! Configuration priority system for Claude settings
//!
//! Manages precedence and layering of configuration from different sources.

use std::cmp::Ordering;
use std::collections::HashMap;
use tracing::{debug, info};
use serde::{Deserialize, Serialize};

/// Priority levels for configuration sources
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConfigPriority {
    /// Default built-in configuration
    Default = 0,
    /// System-wide configuration
    System = 1,
    /// User home directory configuration (~/.claude)
    User = 2,
    /// Project-specific configuration (.claude in project)
    Project = 3,
    /// Explicit override (command-line or environment)
    Override = 4,
}

impl ConfigPriority {
    /// Get a human-readable name for the priority level
    pub fn name(&self) -> &str {
        match self {
            Self::Default => "default",
            Self::System => "system",
            Self::User => "user",
            Self::Project => "project",
            Self::Override => "override",
        }
    }

    /// Check if this priority overrides another
    pub fn overrides(&self, other: &Self) -> bool {
        self > other
    }
}

/// A configuration layer with its priority
#[derive(Debug, Clone)]
pub struct ConfigLayer<T> {
    /// The configuration value
    pub value: T,
    /// Priority of this layer
    pub priority: ConfigPriority,
    /// Optional source path for debugging
    pub source: Option<String>,
}

impl<T> ConfigLayer<T> {
    /// Create a new configuration layer
    pub fn new(value: T, priority: ConfigPriority) -> Self {
        Self {
            value,
            priority,
            source: None,
        }
    }

    /// Create with source information
    pub fn with_source(value: T, priority: ConfigPriority, source: String) -> Self {
        Self {
            value,
            priority,
            source: Some(source),
        }
    }
}

/// Strategy for merging configuration values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Higher priority completely replaces lower
    Replace,
    /// Merge collections (append unique items)
    Merge,
    /// Append values (preserve all)
    Append,
    /// Deep merge (recursive for nested structures)
    Deep,
}

/// Resolved configuration with metadata
#[derive(Debug, Clone)]
pub struct ResolvedConfig<T> {
    /// The final merged value
    pub value: T,
    /// Sources that contributed to this value
    pub sources: Vec<(ConfigPriority, Option<String>)>,
    /// Strategy used for merging
    pub strategy: MergeStrategy,
}

/// Configuration resolver that handles priority and merging
pub struct ConfigResolver {
    /// Log override decisions
    log_overrides: bool,
}

impl ConfigResolver {
    /// Create a new configuration resolver
    pub fn new() -> Self {
        Self {
            log_overrides: true,
        }
    }

    /// Set whether to log override decisions
    pub fn with_logging(mut self, enabled: bool) -> Self {
        self.log_overrides = enabled;
        self
    }

    /// Resolve configuration layers into a single value
    pub fn resolve<T>(&self, layers: Vec<ConfigLayer<T>>, strategy: MergeStrategy) -> ResolvedConfig<T>
    where
        T: Clone + std::fmt::Debug,
    {
        if layers.is_empty() {
            panic!("Cannot resolve empty configuration layers");
        }

        // Sort by priority (highest first)
        let mut sorted_layers = layers;
        sorted_layers.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Track sources
        let sources: Vec<_> = sorted_layers
            .iter()
            .map(|l| (l.priority, l.source.clone()))
            .collect();

        // Apply strategy
        let value = match strategy {
            MergeStrategy::Replace => {
                // Take the highest priority value
                let highest = &sorted_layers[0];
                if self.log_overrides && sorted_layers.len() > 1 {
                    debug!(
                        "Config override: {} from {:?} overrides lower priority configs",
                        highest.priority.name(),
                        highest.source
                    );
                }
                highest.value.clone()
            }
            _ => {
                // For now, other strategies default to replace
                // In a real implementation, we'd handle each strategy
                sorted_layers[0].value.clone()
            }
        };

        ResolvedConfig {
            value,
            sources,
            strategy,
        }
    }

    /// Resolve string configurations with merge
    pub fn resolve_strings(&self, layers: Vec<ConfigLayer<String>>) -> ResolvedConfig<String> {
        if layers.is_empty() {
            return ResolvedConfig {
                value: String::new(),
                sources: vec![],
                strategy: MergeStrategy::Merge,
            };
        }

        let mut sorted_layers = layers;
        sorted_layers.sort_by(|a, b| a.priority.cmp(&b.priority)); // Lower to higher

        let mut merged = String::new();
        let mut sources = Vec::new();
        let mut first = true;

        for layer in sorted_layers {
            if !layer.value.is_empty() {
                if !first && !merged.is_empty() {
                    merged.push_str("\n\n---\n\n");
                }
                merged.push_str(&layer.value);
                sources.push((layer.priority, layer.source));
                first = false;

                if self.log_overrides {
                    debug!(
                        "Merging config from {} ({:?})",
                        layer.priority.name(),
                        sources.last().unwrap().1
                    );
                }
            }
        }

        ResolvedConfig {
            value: merged,
            sources,
            strategy: MergeStrategy::Merge,
        }
    }

    /// Resolve with explicit priority override
    pub fn resolve_with_override<T>(
        &self,
        layers: Vec<ConfigLayer<T>>,
        min_priority: ConfigPriority,
    ) -> Option<ResolvedConfig<T>>
    where
        T: Clone + std::fmt::Debug,
    {
        let filtered: Vec<_> = layers
            .into_iter()
            .filter(|l| l.priority >= min_priority)
            .collect();

        if filtered.is_empty() {
            None
        } else {
            Some(self.resolve(filtered, MergeStrategy::Replace))
        }
    }
}

/// Inspector for debugging configuration
pub struct ConfigInspector;

impl ConfigInspector {
    /// Print configuration layers and their priorities
    pub fn inspect<T: std::fmt::Debug>(layers: &[ConfigLayer<T>]) {
        info!("Configuration layers ({} total):", layers.len());
        for layer in layers {
            info!(
                "  [{}] {:?} from {:?}",
                layer.priority.name(),
                layer.value,
                layer.source
            );
        }
    }

    /// Show resolved configuration details
    pub fn inspect_resolved<T: std::fmt::Debug>(resolved: &ResolvedConfig<T>) {
        info!("Resolved configuration:");
        info!("  Value: {:?}", resolved.value);
        info!("  Strategy: {:?}", resolved.strategy);
        info!("  Sources ({}):", resolved.sources.len());
        for (priority, source) in &resolved.sources {
            info!("    - {} from {:?}", priority.name(), source);
        }
    }

    /// Get active configuration summary
    pub fn summary<T>(layers: &[ConfigLayer<T>]) -> HashMap<String, usize> {
        let mut summary = HashMap::new();
        for layer in layers {
            *summary.entry(layer.priority.name().to_string()).or_insert(0) += 1;
        }
        summary
    }
}

impl Default for ConfigResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        assert!(ConfigPriority::Project > ConfigPriority::User);
        assert!(ConfigPriority::User > ConfigPriority::System);
        assert!(ConfigPriority::Override > ConfigPriority::Project);
        assert!(ConfigPriority::System > ConfigPriority::Default);
    }

    #[test]
    fn test_config_layer_creation() {
        let layer = ConfigLayer::new("test value", ConfigPriority::User);
        assert_eq!(layer.priority, ConfigPriority::User);
        assert!(layer.source.is_none());

        let layer_with_source = ConfigLayer::with_source(
            "test",
            ConfigPriority::Project,
            "/project/.claude".to_string()
        );
        assert!(layer_with_source.source.is_some());
    }

    #[test]
    fn test_resolve_replace_strategy() {
        let resolver = ConfigResolver::new();
        
        let layers = vec![
            ConfigLayer::new("default", ConfigPriority::Default),
            ConfigLayer::new("user", ConfigPriority::User),
            ConfigLayer::new("project", ConfigPriority::Project),
        ];

        let resolved = resolver.resolve(layers, MergeStrategy::Replace);
        assert_eq!(resolved.value, "project");
        assert_eq!(resolved.sources.len(), 3);
    }

    #[test]
    fn test_resolve_strings_merge() {
        let resolver = ConfigResolver::new();
        
        let layers = vec![
            ConfigLayer::new("Default config".to_string(), ConfigPriority::Default),
            ConfigLayer::new("User config".to_string(), ConfigPriority::User),
            ConfigLayer::new("Project config".to_string(), ConfigPriority::Project),
        ];

        let resolved = resolver.resolve_strings(layers);
        assert!(resolved.value.contains("Default config"));
        assert!(resolved.value.contains("User config"));
        assert!(resolved.value.contains("Project config"));
        assert!(resolved.value.contains("---"));
    }

    #[test]
    fn test_resolve_with_override() {
        let resolver = ConfigResolver::new();
        
        let layers = vec![
            ConfigLayer::new("default", ConfigPriority::Default),
            ConfigLayer::new("user", ConfigPriority::User),
            ConfigLayer::new("project", ConfigPriority::Project),
        ];

        // Only consider Project and above
        let resolved = resolver.resolve_with_override(layers, ConfigPriority::Project);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().value, "project");
    }

    #[test]
    fn test_config_inspector_summary() {
        let layers = vec![
            ConfigLayer::new("a", ConfigPriority::Default),
            ConfigLayer::new("b", ConfigPriority::User),
            ConfigLayer::new("c", ConfigPriority::User),
            ConfigLayer::new("d", ConfigPriority::Project),
        ];

        let summary = ConfigInspector::summary(&layers);
        assert_eq!(summary.get("default"), Some(&1));
        assert_eq!(summary.get("user"), Some(&2));
        assert_eq!(summary.get("project"), Some(&1));
    }

    #[test]
    fn test_empty_layers_handling() {
        let resolver = ConfigResolver::new();
        let resolved = resolver.resolve_strings(vec![]);
        assert_eq!(resolved.value, "");
        assert!(resolved.sources.is_empty());
    }
}