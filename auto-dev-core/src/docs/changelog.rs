//! Changelog generation and management

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ChangelogConfig;

/// Builds and maintains changelogs
pub struct ChangelogBuilder {
    config: ChangelogConfig,
    entries: Vec<ChangelogEntry>,
}

/// Changelog entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogEntry {
    /// Version or "Unreleased"
    pub version: String,
    /// Release date
    pub date: Option<DateTime<Local>>,
    /// Changes by category
    pub changes: HashMap<ChangeCategory, Vec<Change>>,
}

/// Category of change
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ChangeCategory {
    Added,
    Changed,
    Deprecated,
    Removed,
    Fixed,
    Security,
}

/// Individual change item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    /// Change description
    pub description: String,
    /// Related issue/PR numbers
    pub references: Vec<String>,
    /// Author
    pub author: Option<String>,
}

impl ChangelogBuilder {
    /// Create new changelog builder
    pub fn new(config: ChangelogConfig) -> Self {
        Self {
            config,
            entries: Vec::new(),
        }
    }

    /// Load existing changelog
    pub fn load(&mut self) -> Result<()> {
        if !self.config.file_path.exists() {
            return Ok(());
        }
        
        let content = fs::read_to_string(&self.config.file_path)
            .context("Failed to read changelog")?;
        
        self.entries = self.parse_changelog(&content)?;
        Ok(())
    }

    /// Add a new change entry
    pub fn add_change(
        &mut self,
        category: ChangeCategory,
        description: String,
        references: Vec<String>,
    ) -> Result<()> {
        // Get or create unreleased entry
        let entry = self.get_or_create_unreleased();
        
        entry.changes.entry(category).or_default().push(Change {
            description,
            references,
            author: None,
        });
        
        Ok(())
    }

    /// Create a new release
    pub fn release(&mut self, version: String) -> Result<()> {
        // Find unreleased entry
        if let Some(unreleased_idx) = self.entries.iter().position(|e| e.version == "Unreleased") {
            let mut entry = self.entries.remove(unreleased_idx);
            entry.version = version;
            entry.date = Some(Local::now());
            
            // Add as first entry (most recent)
            self.entries.insert(0, entry);
        }
        
        Ok(())
    }

    /// Update changelog file
    pub fn update(&self) -> Result<()> {
        let content = self.generate_changelog();
        fs::write(&self.config.file_path, content)
            .context("Failed to write changelog")?;
        Ok(())
    }

    /// Generate changelog content
    pub fn generate_changelog(&self) -> String {
        let mut content = String::new();
        
        // Header
        content.push_str("# Changelog\n\n");
        content.push_str("All notable changes to this project will be documented in this file.\n\n");
        content.push_str("The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),\n");
        content.push_str("and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n\n");
        
        // Entries
        for entry in &self.entries {
            // Skip empty unreleased section
            if entry.version == "Unreleased" && entry.changes.is_empty() {
                continue;
            }
            
            content.push_str(&self.format_entry(entry));
            content.push_str("\n");
        }
        
        content
    }

    /// Extract changes from git commits
    pub async fn extract_from_git(&mut self, since_tag: Option<&str>) -> Result<()> {
        use std::process::Command;
        
        let mut cmd = Command::new("git");
        cmd.arg("log");
        cmd.arg("--pretty=format:%s (%h)");
        
        if let Some(tag) = since_tag {
            cmd.arg(format!("{}..HEAD", tag));
        } else {
            cmd.arg("-20"); // Last 20 commits if no tag specified
        }
        
        let output = cmd.output()
            .context("Failed to run git log")?;
        
        if !output.status.success() {
            return Ok(()); // Git not available or not a git repo
        }
        
        let commits = String::from_utf8_lossy(&output.stdout);
        
        for line in commits.lines() {
            let (category, description) = self.categorize_commit(line);
            self.add_change(category, description, vec![])?;
        }
        
        Ok(())
    }

    fn get_or_create_unreleased(&mut self) -> &mut ChangelogEntry {
        if !self.entries.iter().any(|e| e.version == "Unreleased") {
            self.entries.insert(0, ChangelogEntry {
                version: "Unreleased".to_string(),
                date: None,
                changes: HashMap::new(),
            });
        }
        
        self.entries
            .iter_mut()
            .find(|e| e.version == "Unreleased")
            .unwrap()
    }

    fn parse_changelog(&self, content: &str) -> Result<Vec<ChangelogEntry>> {
        let mut entries = Vec::new();
        let mut current_entry: Option<ChangelogEntry> = None;
        let mut current_category: Option<ChangeCategory> = None;
        
        for line in content.lines() {
            // Version header
            if line.starts_with("## ") {
                if let Some(entry) = current_entry.take() {
                    entries.push(entry);
                }
                
                let version_line = &line[3..];
                let (version, date) = self.parse_version_line(version_line);
                
                current_entry = Some(ChangelogEntry {
                    version,
                    date,
                    changes: HashMap::new(),
                });
                current_category = None;
            }
            // Category header
            else if line.starts_with("### ") {
                let category = match &line[4..] {
                    "Added" => ChangeCategory::Added,
                    "Changed" => ChangeCategory::Changed,
                    "Deprecated" => ChangeCategory::Deprecated,
                    "Removed" => ChangeCategory::Removed,
                    "Fixed" => ChangeCategory::Fixed,
                    "Security" => ChangeCategory::Security,
                    _ => continue,
                };
                current_category = Some(category);
            }
            // Change item
            else if line.starts_with("- ") && current_category.is_some() && current_entry.is_some() {
                let description = line[2..].to_string();
                let change = Change {
                    description,
                    references: vec![],
                    author: None,
                };
                
                if let (Some(entry), Some(category)) = (&mut current_entry, &current_category) {
                    entry.changes.entry(category.clone()).or_default().push(change);
                }
            }
        }
        
        if let Some(entry) = current_entry {
            entries.push(entry);
        }
        
        Ok(entries)
    }

    fn parse_version_line(&self, line: &str) -> (String, Option<DateTime<Local>>) {
        if let Some(bracket_pos) = line.find('[') {
            if let Some(close_pos) = line.find(']') {
                let version = line[bracket_pos + 1..close_pos].to_string();
                
                // Try to parse date after the closing bracket
                if let Some(date_start) = line.find("- ") {
                    let _date_str = &line[date_start + 2..];
                    // Simple date parsing (could be improved)
                    return (version, None);
                }
                
                return (version, None);
            }
        }
        
        (line.to_string(), None)
    }

    fn format_entry(&self, entry: &ChangelogEntry) -> String {
        let mut content = String::new();
        
        // Version header
        if let Some(date) = entry.date {
            content.push_str(&format!(
                "## [{}] - {}\n\n",
                entry.version,
                date.format("%Y-%m-%d")
            ));
        } else {
            content.push_str(&format!("## [{}]\n\n", entry.version));
        }
        
        // Categories in standard order
        let category_order = [
            ChangeCategory::Added,
            ChangeCategory::Changed,
            ChangeCategory::Deprecated,
            ChangeCategory::Removed,
            ChangeCategory::Fixed,
            ChangeCategory::Security,
        ];
        
        for category in &category_order {
            if let Some(changes) = entry.changes.get(category) {
                if !changes.is_empty() {
                    content.push_str(&format!("### {}\n", category_to_string(category)));
                    for change in changes {
                        content.push_str(&format!("- {}", change.description));
                        
                        if !change.references.is_empty() {
                            content.push_str(&format!(" ({})", change.references.join(", ")));
                        }
                        
                        content.push('\n');
                    }
                    content.push('\n');
                }
            }
        }
        
        content
    }

    fn categorize_commit(&self, message: &str) -> (ChangeCategory, String) {
        let lower = message.to_lowercase();
        
        if lower.starts_with("feat:") || lower.starts_with("feature:") {
            (ChangeCategory::Added, message.to_string())
        } else if lower.starts_with("fix:") || lower.starts_with("bugfix:") {
            (ChangeCategory::Fixed, message.to_string())
        } else if lower.starts_with("security:") {
            (ChangeCategory::Security, message.to_string())
        } else if lower.starts_with("deprecated:") {
            (ChangeCategory::Deprecated, message.to_string())
        } else if lower.starts_with("remove:") || lower.starts_with("removed:") {
            (ChangeCategory::Removed, message.to_string())
        } else {
            (ChangeCategory::Changed, message.to_string())
        }
    }
}

fn category_to_string(category: &ChangeCategory) -> &'static str {
    match category {
        ChangeCategory::Added => "Added",
        ChangeCategory::Changed => "Changed",
        ChangeCategory::Deprecated => "Deprecated",
        ChangeCategory::Removed => "Removed",
        ChangeCategory::Fixed => "Fixed",
        ChangeCategory::Security => "Security",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_add_change() {
        let dir = tempdir().unwrap();
        let changelog_path = dir.path().join("CHANGELOG.md");
        
        let config = ChangelogConfig {
            file_path: changelog_path.clone(),
            include_unreleased: true,
            categorize: true,
        };
        
        let mut builder = ChangelogBuilder::new(config);
        
        builder.add_change(
            ChangeCategory::Added,
            "New feature X".to_string(),
            vec!["#123".to_string()],
        ).unwrap();
        
        let content = builder.generate_changelog();
        assert!(content.contains("### Added"));
        assert!(content.contains("New feature X"));
    }

    #[test]
    fn test_release() {
        let dir = tempdir().unwrap();
        let changelog_path = dir.path().join("CHANGELOG.md");
        
        let config = ChangelogConfig {
            file_path: changelog_path,
            include_unreleased: true,
            categorize: true,
        };
        
        let mut builder = ChangelogBuilder::new(config);
        
        builder.add_change(
            ChangeCategory::Added,
            "Feature Y".to_string(),
            vec![],
        ).unwrap();
        
        builder.release("1.0.0".to_string()).unwrap();
        
        let content = builder.generate_changelog();
        assert!(content.contains("[1.0.0]"));
        assert!(!content.contains("[Unreleased]"));
    }
}