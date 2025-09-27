//! Commit message builder for semantic commits

use serde::{Deserialize, Serialize};
use std::fmt;

use super::CommitStyle;

/// Builds semantic commit messages
pub struct CommitBuilder {
    style: CommitStyle,
}

/// Type of commit according to conventional commits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommitType {
    /// New feature
    Feat,
    /// Bug fix
    Fix,
    /// Code refactoring
    Refactor,
    /// Performance improvement
    Perf,
    /// Documentation changes
    Docs,
    /// Style changes (formatting, etc)
    Style,
    /// Test changes
    Test,
    /// Build system changes
    Build,
    /// CI/CD changes
    Ci,
    /// Chore/maintenance
    Chore,
    /// Revert a previous commit
    Revert,
}

impl fmt::Display for CommitType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            CommitType::Feat => "feat",
            CommitType::Fix => "fix",
            CommitType::Refactor => "refactor",
            CommitType::Perf => "perf",
            CommitType::Docs => "docs",
            CommitType::Style => "style",
            CommitType::Test => "test",
            CommitType::Build => "build",
            CommitType::Ci => "ci",
            CommitType::Chore => "chore",
            CommitType::Revert => "revert",
        };
        write!(f, "{}", s)
    }
}

impl CommitBuilder {
    /// Create a new commit builder
    pub fn new(style: CommitStyle) -> Self {
        Self { style }
    }

    /// Build a commit message
    pub fn build_message(
        &self,
        commit_type: CommitType,
        scope: Option<&str>,
        description: &str,
    ) -> String {
        match self.style {
            CommitStyle::Conventional => {
                self.build_conventional(commit_type, scope, description, None, None, false)
            }
            CommitStyle::Simple => {
                format!("{}: {}", commit_type, description)
            }
        }
    }

    /// Build a conventional commit message with all options
    pub fn build_conventional(
        &self,
        commit_type: CommitType,
        scope: Option<&str>,
        description: &str,
        body: Option<&str>,
        footer: Option<&str>,
        breaking: bool,
    ) -> String {
        let mut message = String::new();
        
        // Type
        message.push_str(&commit_type.to_string());
        
        // Scope
        if let Some(scope) = scope {
            message.push_str(&format!("({})", scope));
        }
        
        // Breaking change indicator
        if breaking {
            message.push('!');
        }
        
        // Description
        message.push_str(&format!(": {}", description));
        
        // Body (optional)
        if let Some(body) = body {
            message.push_str(&format!("\n\n{}", body));
        }
        
        // Footer (optional)
        if let Some(footer) = footer {
            message.push_str(&format!("\n\n{}", footer));
        }
        
        // Auto-generated footer
        message.push_str("\n\n[auto-dev-rs] Automated self-development commit");
        
        message
    }

    /// Build a commit message for a self-modification
    pub fn build_self_modification(
        &self,
        module: &str,
        change_type: &str,
        details: &str,
    ) -> String {
        let commit_type = self.infer_commit_type(change_type);
        let scope = Some(module);
        let description = format!("{}: {}", change_type, details);
        
        self.build_message(commit_type, scope, &description)
    }

    /// Build a merge commit message
    pub fn build_merge_message(&self, source_branch: &str, target_branch: &str) -> String {
        format!(
            "Merge branch '{}' into '{}'\n\n[auto-dev-rs] Automated merge",
            source_branch,
            target_branch
        )
    }

    /// Build a revert commit message
    pub fn build_revert_message(&self, original_commit: &str, reason: &str) -> String {
        format!(
            "Revert \"{}\"\n\nReason: {}\n\n[auto-dev-rs] Automated revert",
            original_commit,
            reason
        )
    }

    /// Infer commit type from change description
    fn infer_commit_type(&self, change_type: &str) -> CommitType {
        let lower = change_type.to_lowercase();
        
        if lower.contains("feature") || lower.contains("add") || lower.contains("implement") {
            CommitType::Feat
        } else if lower.contains("fix") || lower.contains("bug") || lower.contains("patch") {
            CommitType::Fix
        } else if lower.contains("refactor") || lower.contains("restructure") {
            CommitType::Refactor
        } else if lower.contains("perf") || lower.contains("optimize") || lower.contains("speed") {
            CommitType::Perf
        } else if lower.contains("doc") || lower.contains("comment") {
            CommitType::Docs
        } else if lower.contains("test") {
            CommitType::Test
        } else if lower.contains("build") || lower.contains("compile") {
            CommitType::Build
        } else if lower.contains("ci") || lower.contains("pipeline") {
            CommitType::Ci
        } else {
            CommitType::Chore
        }
    }

    /// Validate a commit message follows conventions
    pub fn validate_message(&self, message: &str) -> Result<(), ValidationError> {
        match self.style {
            CommitStyle::Conventional => self.validate_conventional(message),
            CommitStyle::Simple => Ok(()), // Simple style has no strict rules
        }
    }

    /// Validate conventional commit format
    fn validate_conventional(&self, message: &str) -> Result<(), ValidationError> {
        let lines: Vec<&str> = message.lines().collect();
        
        if lines.is_empty() {
            return Err(ValidationError::EmptyMessage);
        }
        
        let header = lines[0];
        
        // Check for type
        let valid_types = [
            "feat", "fix", "refactor", "perf", "docs", 
            "style", "test", "build", "ci", "chore", "revert"
        ];
        
        let has_valid_type = valid_types.iter()
            .any(|&t| header.starts_with(&format!("{}:", t)) || header.starts_with(&format!("{}(", t)));
        
        if !has_valid_type {
            return Err(ValidationError::InvalidType);
        }
        
        // Check for description after colon
        if !header.contains(": ") {
            return Err(ValidationError::MissingDescription);
        }
        
        // Check description length (should be <= 72 chars for header)
        if header.len() > 72 {
            return Err(ValidationError::HeaderTooLong);
        }
        
        Ok(())
    }

    /// Generate a commit message from file changes
    pub fn from_changes(&self, changes: &[FileChange]) -> String {
        if changes.is_empty() {
            return self.build_message(
                CommitType::Chore,
                None,
                "empty commit"
            );
        }
        
        // Group changes by type
        let mut added = 0;
        let mut modified = 0;
        let mut deleted = 0;
        
        for change in changes {
            match change.change_type {
                FileChangeType::Added => added += 1,
                FileChangeType::Modified => modified += 1,
                FileChangeType::Deleted => deleted += 1,
            }
        }
        
        // Determine primary action
        let (commit_type, description) = if added > modified && added > deleted {
            (CommitType::Feat, format!("add {} new files", added))
        } else if modified > deleted {
            (CommitType::Refactor, format!("update {} files", modified))
        } else {
            (CommitType::Chore, format!("remove {} files", deleted))
        };
        
        // Determine scope from common path prefix
        let scope = self.find_common_scope(changes);
        
        self.build_message(commit_type, scope.as_deref(), &description)
    }

    /// Find common scope from file paths
    fn find_common_scope(&self, changes: &[FileChange]) -> Option<String> {
        if changes.is_empty() {
            return None;
        }
        
        // Extract directory components
        let paths: Vec<Vec<&str>> = changes.iter()
            .map(|c| c.path.split('/').collect())
            .collect();
        
        // Find common prefix
        let mut common = Vec::new();
        let min_len = paths.iter().map(|p| p.len()).min().unwrap_or(0);
        
        for i in 0..min_len {
            let component = paths[0][i];
            if paths.iter().all(|p| p[i] == component) {
                common.push(component);
            } else {
                break;
            }
        }
        
        if !common.is_empty() && common[0] != "." {
            Some(common[0].to_string())
        } else {
            None
        }
    }
}

/// File change information
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: String,
    pub change_type: FileChangeType,
}

/// Type of file change
#[derive(Debug, Clone)]
pub enum FileChangeType {
    Added,
    Modified,
    Deleted,
}

/// Validation error for commit messages
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Empty commit message")]
    EmptyMessage,
    #[error("Invalid commit type")]
    InvalidType,
    #[error("Missing description after type")]
    MissingDescription,
    #[error("Header line too long (max 72 chars)")]
    HeaderTooLong,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conventional_commit() {
        let builder = CommitBuilder::new(CommitStyle::Conventional);
        let message = builder.build_message(
            CommitType::Feat,
            Some("vcs"),
            "add git integration"
        );
        
        assert!(message.starts_with("feat(vcs): add git integration"));
        assert!(message.contains("[auto-dev-rs]"));
    }

    #[test]
    fn test_simple_commit() {
        let builder = CommitBuilder::new(CommitStyle::Simple);
        let message = builder.build_message(
            CommitType::Fix,
            None,
            "resolve merge conflict"
        );
        
        assert_eq!(message, "fix: resolve merge conflict");
    }

    #[test]
    fn test_breaking_change() {
        let builder = CommitBuilder::new(CommitStyle::Conventional);
        let message = builder.build_conventional(
            CommitType::Feat,
            Some("api"),
            "change response format",
            None,
            None,
            true
        );
        
        assert!(message.starts_with("feat(api)!: change response format"));
    }

    #[test]
    fn test_validate_conventional() {
        let builder = CommitBuilder::new(CommitStyle::Conventional);
        
        assert!(builder.validate_message("feat: add new feature").is_ok());
        assert!(builder.validate_message("fix(core): resolve bug").is_ok());
        assert!(builder.validate_message("invalid message").is_err());
        assert!(builder.validate_message("feat no colon").is_err());
    }

    #[test]
    fn test_infer_commit_type() {
        let builder = CommitBuilder::new(CommitStyle::Conventional);
        
        assert!(matches!(
            builder.infer_commit_type("add new feature"),
            CommitType::Feat
        ));
        assert!(matches!(
            builder.infer_commit_type("fix bug"),
            CommitType::Fix
        ));
        assert!(matches!(
            builder.infer_commit_type("optimize performance"),
            CommitType::Perf
        ));
    }
}