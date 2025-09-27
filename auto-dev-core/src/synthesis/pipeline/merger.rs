#![allow(unused)]
//! Code merger for integrating generated code with existing code

use super::{PipelineContext, PipelineStage};
use crate::synthesis::{Result, SynthesisError};
use crate::{debug, info};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Merges generated code with existing code
pub struct CodeMerger {
    strategies: Vec<Box<dyn MergeStrategy>>,
}

impl CodeMerger {
    /// Create a new code merger
    pub fn new() -> Self {
        Self {
            strategies: vec![
                Box::new(FileReplacementStrategy),
                Box::new(FunctionInsertionStrategy),
                Box::new(BlockModificationStrategy),
                Box::new(LinePatchStrategy),
            ],
        }
    }

    /// Merge generated code into target file
    async fn merge_file(
        &self,
        target: &Path,
        generated: &str,
        strategy_type: MergeType,
    ) -> Result<MergeResult> {
        // Select appropriate strategy
        let strategy =
            self.strategies.iter().find(|s| s.supports(&strategy_type)).ok_or_else(|| {
                SynthesisError::MergeError(format!(
                    "No strategy for merge type: {:?}",
                    strategy_type
                ))
            })?;

        // Read existing content if file exists
        let existing = if target.exists() { Some(fs::read_to_string(target).await?) } else { None };

        // Perform merge
        let merged = strategy.merge(existing.as_deref(), generated)?;

        // Write merged content
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(target, &merged.content).await?;

        Ok(merged)
    }

    /// Determine merge type based on context
    fn determine_merge_type(&self, target: &Path, existing: bool) -> MergeType {
        if !existing {
            MergeType::FileReplacement
        } else if target.extension().and_then(|e| e.to_str()) == Some("rs") {
            MergeType::FunctionInsertion
        } else {
            MergeType::BlockModification
        }
    }
}

#[async_trait]
impl PipelineStage for CodeMerger {
    fn name(&self) -> &'static str {
        "CodeMerger"
    }

    async fn execute(&self, mut context: PipelineContext) -> Result<PipelineContext> {
        info!("Merging generated code into project");

        context.metadata.current_stage = self.name().to_string();

        if context.generated_files.is_empty() && context.modified_files.is_empty() {
            context.add_warning("No files to merge".to_string());
            return Ok(context);
        }

        // Merge each generated file
        let generated_files = context.generated_files.clone();
        let mut warnings = Vec::new();
        let mut modified_files = Vec::new();

        for file_path in &generated_files {
            let exists = file_path.exists();
            let merge_type = self.determine_merge_type(file_path, exists);

            // For this example, we'll use placeholder generated content
            // In a real implementation, this would come from the generator stage
            let generated_content = "// Generated code placeholder\n";

            match self.merge_file(file_path, generated_content, merge_type).await {
                Ok(result) => {
                    if exists {
                        modified_files.push(file_path.clone());
                    }

                    debug!(
                        "Merged code into {} (added: {}, modified: {})",
                        file_path.display(),
                        result.lines_added,
                        result.lines_modified
                    );
                }
                Err(e) => {
                    warnings.push(format!("Failed to merge {}: {}", file_path.display(), e));
                }
            }
        }

        // Add warnings and modified files after iteration
        for warning in warnings {
            context.add_warning(warning);
        }
        context.modified_files.extend(modified_files);

        Ok(context)
    }
}

/// Merge strategy trait
trait MergeStrategy: Send + Sync {
    /// Check if this strategy supports the given merge type
    fn supports(&self, merge_type: &MergeType) -> bool;

    /// Perform the merge
    fn merge(&self, existing: Option<&str>, generated: &str) -> Result<MergeResult>;
}

/// Type of merge operation
#[derive(Debug, Clone)]
enum MergeType {
    FileReplacement,
    FunctionInsertion,
    BlockModification,
    LinePatch,
}

/// Result of a merge operation
struct MergeResult {
    content: String,
    lines_added: usize,
    lines_modified: usize,
    conflicts: Vec<String>,
}

/// Strategy for replacing entire files
struct FileReplacementStrategy;

impl MergeStrategy for FileReplacementStrategy {
    fn supports(&self, merge_type: &MergeType) -> bool {
        matches!(merge_type, MergeType::FileReplacement)
    }

    fn merge(&self, _existing: Option<&str>, generated: &str) -> Result<MergeResult> {
        Ok(MergeResult {
            content: generated.to_string(),
            lines_added: generated.lines().count(),
            lines_modified: 0,
            conflicts: Vec::new(),
        })
    }
}

/// Strategy for inserting new functions
struct FunctionInsertionStrategy;

impl MergeStrategy for FunctionInsertionStrategy {
    fn supports(&self, merge_type: &MergeType) -> bool {
        matches!(merge_type, MergeType::FunctionInsertion)
    }

    fn merge(&self, existing: Option<&str>, generated: &str) -> Result<MergeResult> {
        let existing = existing.unwrap_or("");

        // Simple approach: append new functions at the end
        // Real implementation would parse AST and insert appropriately
        let merged = format!("{}\n\n{}", existing, generated);

        Ok(MergeResult {
            content: merged,
            lines_added: generated.lines().count(),
            lines_modified: 0,
            conflicts: Vec::new(),
        })
    }
}

/// Strategy for modifying code blocks
struct BlockModificationStrategy;

impl MergeStrategy for BlockModificationStrategy {
    fn supports(&self, merge_type: &MergeType) -> bool {
        matches!(merge_type, MergeType::BlockModification)
    }

    fn merge(&self, existing: Option<&str>, generated: &str) -> Result<MergeResult> {
        let existing = existing.unwrap_or("");

        // Simplified: replace matching blocks or append
        // Real implementation would use AST-based merging
        if existing.is_empty() {
            Ok(MergeResult {
                content: generated.to_string(),
                lines_added: generated.lines().count(),
                lines_modified: 0,
                conflicts: Vec::new(),
            })
        } else {
            // For now, just append
            let merged = format!("{}\n\n{}", existing, generated);
            Ok(MergeResult {
                content: merged,
                lines_added: generated.lines().count(),
                lines_modified: 0,
                conflicts: Vec::new(),
            })
        }
    }
}

/// Strategy for line-level patches
struct LinePatchStrategy;

impl MergeStrategy for LinePatchStrategy {
    fn supports(&self, merge_type: &MergeType) -> bool {
        matches!(merge_type, MergeType::LinePatch)
    }

    fn merge(&self, existing: Option<&str>, generated: &str) -> Result<MergeResult> {
        let existing = existing.unwrap_or("");

        // Simplified: append patches
        // Real implementation would use diff/patch algorithms
        let merged = if existing.is_empty() {
            generated.to_string()
        } else {
            format!("{}\n{}", existing, generated)
        };

        Ok(MergeResult {
            content: merged,
            lines_added: generated.lines().count(),
            lines_modified: 0,
            conflicts: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_replacement() {
        let strategy = FileReplacementStrategy;
        let result = strategy.merge(None, "fn new() {}").unwrap();
        assert_eq!(result.content, "fn new() {}");
        assert_eq!(result.lines_added, 1);
    }

    #[test]
    fn test_function_insertion() {
        let strategy = FunctionInsertionStrategy;
        let existing = "fn existing() {}";
        let generated = "fn new() {}";
        let result = strategy.merge(Some(existing), generated).unwrap();
        assert!(result.content.contains("existing"));
        assert!(result.content.contains("new"));
    }
}
