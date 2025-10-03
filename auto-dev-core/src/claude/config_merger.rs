//! Configuration merging strategies for Claude settings
//!
//! Provides type-specific merge strategies for different configuration elements.

use crate::claude::{ClaudeCommand, ClaudeMdContent, CommandRegistry};
use std::collections::{HashMap, HashSet};
use tracing::debug;

/// Merger for different configuration types
pub struct ConfigMerger;

impl ConfigMerger {
    /// Merge CLAUDE.md content from multiple sources
    pub fn merge_claude_md(contents: Vec<ClaudeMdContent>) -> ClaudeMdContent {
        if contents.is_empty() {
            return ClaudeMdContent { content: String::new(), sources: vec![], total_size: 0 };
        }

        if contents.len() == 1 {
            return contents.into_iter().next().unwrap();
        }

        let mut merged_content = String::new();
        let mut all_sources = Vec::new();
        let mut total_size = 0;

        for (i, content) in contents.iter().enumerate() {
            if !content.content.is_empty() {
                if i > 0 && !merged_content.is_empty() {
                    merged_content.push_str("\n\n---\n\n");
                }
                merged_content.push_str(&content.content);
            }

            all_sources.extend(content.sources.clone());
            total_size += content.total_size;
        }

        ClaudeMdContent { content: merged_content, sources: all_sources, total_size }
    }

    /// Merge command registries with priority
    pub fn merge_command_registries(
        global_registry: Option<CommandRegistry>,
        project_registry: Option<CommandRegistry>,
    ) -> CommandRegistry {
        let mut merged = CommandRegistry::new();

        // Add global commands first
        if let Some(global) = global_registry {
            for cmd in global.all_commands() {
                debug!("Adding global command: {}", cmd.name);
                merged.add_command(cmd.clone());
            }
        }

        // Project commands override global
        if let Some(project) = project_registry {
            for cmd in project.all_commands() {
                if merged.contains(&cmd.name) {
                    debug!("Project command '{}' overriding global", cmd.name);
                }
                merged.add_command(cmd.clone());
            }
        }

        merged
    }

    /// Merge key-value configurations
    pub fn merge_maps(
        base: HashMap<String, String>,
        override_map: HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut merged = base;

        for (key, value) in override_map {
            debug!("Overriding config key '{}' with new value", key);
            merged.insert(key, value);
        }

        merged
    }

    /// Merge list configurations (append unique)
    pub fn merge_lists(base: Vec<String>, additional: Vec<String>) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut merged = Vec::new();

        // Add base items
        for item in base {
            if seen.insert(item.clone()) {
                merged.push(item);
            }
        }

        // Add additional items
        for item in additional {
            if seen.insert(item.clone()) {
                merged.push(item);
            }
        }

        merged
    }

    /// Deep merge for nested structures
    pub fn deep_merge<T>(base: T, overlay: T) -> T
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de>,
    {
        // Convert to JSON values for merging
        let mut base_value = serde_json::to_value(base).unwrap();
        let overlay_value = serde_json::to_value(overlay).unwrap();

        Self::merge_json_values(&mut base_value, overlay_value);

        // Convert back to original type
        serde_json::from_value(base_value).unwrap()
    }

    /// Merge JSON values recursively
    fn merge_json_values(base: &mut serde_json::Value, overlay: serde_json::Value) {
        use serde_json::Value;

        match (base, overlay) {
            (Value::Object(base_map), Value::Object(overlay_map)) => {
                for (key, value) in overlay_map {
                    match base_map.get_mut(&key) {
                        Some(base_value) => Self::merge_json_values(base_value, value),
                        None => {
                            base_map.insert(key, value);
                        }
                    }
                }
            }
            (base_val, overlay_val) => {
                *base_val = overlay_val;
            }
        }
    }
}

/// Configuration merge context for tracking decisions
pub struct MergeContext {
    decisions: Vec<MergeDecision>,
}

/// A single merge decision
#[derive(Debug, Clone)]
pub struct MergeDecision {
    pub field: String,
    pub action: MergeAction,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub enum MergeAction {
    Keep,
    Override,
    Merge,
    Append,
}

impl MergeContext {
    /// Create a new merge context
    pub fn new() -> Self {
        Self { decisions: Vec::new() }
    }

    /// Record a merge decision
    pub fn record(&mut self, field: String, action: MergeAction, reason: String) {
        self.decisions.push(MergeDecision { field, action, reason });
    }

    /// Get all decisions made
    pub fn decisions(&self) -> &[MergeDecision] {
        &self.decisions
    }

    /// Log all decisions
    pub fn log_decisions(&self) {
        for decision in &self.decisions {
            debug!(
                "Merge decision for '{}': {:?} - {}",
                decision.field, decision.action, decision.reason
            );
        }
    }
}

impl Default for MergeContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_merge_claude_md() {
        let content1 = ClaudeMdContent {
            content: "Global instructions".to_string(),
            sources: vec![PathBuf::from("/home/.claude/CLAUDE.md")],
            total_size: 18,
        };

        let content2 = ClaudeMdContent {
            content: "Project instructions".to_string(),
            sources: vec![PathBuf::from("/project/.claude/CLAUDE.md")],
            total_size: 19,
        };

        let merged = ConfigMerger::merge_claude_md(vec![content1, content2]);

        assert!(merged.content.contains("Global instructions"));
        assert!(merged.content.contains("Project instructions"));
        assert!(merged.content.contains("---"));
        assert_eq!(merged.sources.len(), 2);
        assert_eq!(merged.total_size, 37);
    }

    #[test]
    fn test_merge_command_registries() {
        let mut global_registry = CommandRegistry::new();
        global_registry.add_command(ClaudeCommand::new(
            "global-cmd".to_string(),
            "Global command".to_string(),
        ));
        global_registry.add_command(ClaudeCommand::new(
            "shared-cmd".to_string(),
            "Global version".to_string(),
        ));

        let mut project_registry = CommandRegistry::new();
        project_registry.add_command(ClaudeCommand::new(
            "project-cmd".to_string(),
            "Project command".to_string(),
        ));
        project_registry.add_command(ClaudeCommand::new(
            "shared-cmd".to_string(),
            "Project version".to_string(),
        ));

        let merged =
            ConfigMerger::merge_command_registries(Some(global_registry), Some(project_registry));

        assert!(merged.contains("global-cmd"));
        assert!(merged.contains("project-cmd"));
        assert!(merged.contains("shared-cmd"));

        // Project version should override
        let shared = merged.get("shared-cmd").unwrap();
        assert_eq!(shared.description, "Project version");
    }

    #[test]
    fn test_merge_maps() {
        let mut base = HashMap::new();
        base.insert("key1".to_string(), "base_value1".to_string());
        base.insert("key2".to_string(), "base_value2".to_string());

        let mut overlay = HashMap::new();
        overlay.insert("key2".to_string(), "override_value2".to_string());
        overlay.insert("key3".to_string(), "new_value3".to_string());

        let merged = ConfigMerger::merge_maps(base, overlay);

        assert_eq!(merged.get("key1"), Some(&"base_value1".to_string()));
        assert_eq!(merged.get("key2"), Some(&"override_value2".to_string()));
        assert_eq!(merged.get("key3"), Some(&"new_value3".to_string()));
    }

    #[test]
    fn test_merge_lists() {
        let base = vec!["item1".to_string(), "item2".to_string()];
        let additional = vec!["item2".to_string(), "item3".to_string()];

        let merged = ConfigMerger::merge_lists(base, additional);

        assert_eq!(merged.len(), 3);
        assert!(merged.contains(&"item1".to_string()));
        assert!(merged.contains(&"item2".to_string()));
        assert!(merged.contains(&"item3".to_string()));
    }

    #[test]
    fn test_merge_context() {
        let mut context = MergeContext::new();

        context.record(
            "commands".to_string(),
            MergeAction::Override,
            "Project overrides global".to_string(),
        );

        context.record(
            "settings".to_string(),
            MergeAction::Merge,
            "Merging settings objects".to_string(),
        );

        assert_eq!(context.decisions().len(), 2);

        let first = &context.decisions()[0];
        assert_eq!(first.field, "commands");
        matches!(first.action, MergeAction::Override);
    }
}
