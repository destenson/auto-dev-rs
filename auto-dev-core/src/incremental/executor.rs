//! Increment execution and code generation

use super::{
    AttemptResult, ChangeType, FileChange, Increment, IncrementStatus, IncrementalError, Result,
    rollback::RollbackManager, validator::IncrementValidator,
};
use crate::llm::provider::LLMProvider;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, error, info, warn};

/// Executes individual increments
pub struct IncrementExecutor {
    llm_provider: Box<dyn LLMProvider>,
    rollback_manager: RollbackManager,
    validator: IncrementValidator,
    config: ExecutorConfig,
}

#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    pub project_root: PathBuf,
    pub enable_rollback: bool,
    pub max_retries: usize,
    pub timeout_seconds: u64,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            project_root: PathBuf::from("."),
            enable_rollback: true,
            max_retries: 3,
            timeout_seconds: 300,
        }
    }
}

impl IncrementExecutor {
    /// Create a new increment executor
    pub fn new(llm_provider: Box<dyn LLMProvider>, config: ExecutorConfig) -> Result<Self> {
        let rollback_manager = RollbackManager::new(config.project_root.clone())?;
        let validator = IncrementValidator::new(config.project_root.clone());

        Ok(Self { llm_provider, rollback_manager, validator, config })
    }

    /// Execute a single increment
    pub async fn execute(&mut self, increment: &mut Increment) -> Result<ExecutionResult> {
        info!("Executing increment: {} - {}", increment.id, increment.specification.description);

        // Start attempt
        increment.status = IncrementStatus::InProgress;
        let attempt_idx = increment.attempts.len();
        increment.add_attempt();

        // Create checkpoint if rollback is enabled
        let checkpoint_id = if self.config.enable_rollback {
            Some(self.rollback_manager.create_checkpoint().await?)
        } else {
            None
        };

        // Try execution with retries
        let mut last_error = None;
        for retry in 0..self.config.max_retries {
            if retry > 0 {
                info!("Retry attempt {} of {}", retry + 1, self.config.max_retries);
            }

            match self.execute_attempt(increment).await {
                Ok(result) => {
                    // Success! Mark increment as completed
                    increment.status = IncrementStatus::Completed;
                    if let Some(attempt) = increment.attempts.get_mut(attempt_idx) {
                        attempt.ended_at = Some(chrono::Utc::now());
                        attempt.result = Some(AttemptResult::Success);
                    }

                    // Clean up old checkpoints
                    if checkpoint_id.is_some() {
                        self.rollback_manager.cleanup_old_checkpoints(5).await?;
                    }

                    info!("Successfully executed increment: {}", increment.id);
                    return Ok(result);
                }
                Err(e) => {
                    warn!("Execution attempt failed: {}", e);
                    last_error = Some(e);
                    if let Some(attempt) = increment.attempts.get_mut(attempt_idx) {
                        attempt.logs.push(format!(
                            "Attempt {} failed: {:?}",
                            retry + 1,
                            last_error
                        ));
                    }

                    // Rollback if enabled
                    if let Some(ref checkpoint) = checkpoint_id {
                        self.rollback_manager.rollback_to(checkpoint.clone()).await?;
                    }
                }
            }
        }

        // All retries failed
        increment.status = IncrementStatus::Failed;

        let error_msg =
            last_error.map(|e| e.to_string()).unwrap_or_else(|| "Unknown error".to_string());
        if let Some(attempt) = increment.attempts.get_mut(attempt_idx) {
            attempt.ended_at = Some(chrono::Utc::now());
            attempt.result = Some(AttemptResult::ValidationFailure(error_msg.clone()));
        }

        error!(
            "Failed to execute increment after {} retries: {}",
            self.config.max_retries, increment.id
        );
        Err(IncrementalError::ExecutionError(error_msg))
    }

    /// Execute a single attempt
    async fn execute_attempt(&mut self, increment: &mut Increment) -> Result<ExecutionResult> {
        // Generate implementation
        let implementation = self.generate_implementation(increment).await?;

        // Apply file changes
        self.apply_changes(&implementation).await?;

        // Validate compilation
        increment.status = IncrementStatus::Testing;
        if increment.validation.must_compile {
            debug!("Validating compilation...");
            let compilation_result = self.validator.validate_compilation().await?;
            if !compilation_result.success {
                return Err(IncrementalError::CompilationError(compilation_result.message));
            }
        }

        // Run tests
        if !increment.tests.is_empty() {
            debug!("Running {} tests...", increment.tests.len());
            let test_results = self.validator.run_tests(&increment.tests).await?;

            let failed_tests: Vec<_> =
                test_results.iter().filter(|r| !r.passed).map(|r| r.test_id.clone()).collect();

            if !failed_tests.is_empty() {
                return Err(IncrementalError::TestFailure(format!(
                    "Tests failed: {:?}",
                    failed_tests
                )));
            }
        }

        // Run security checks
        for check in &increment.validation.security_checks {
            debug!("Running security check: {}", check.name);
            let result = self.validator.run_security_check(check).await?;
            if !result.passed {
                return Err(IncrementalError::ValidationError(format!(
                    "Security check '{}' failed: {}",
                    check.name, result.message
                )));
            }
        }

        Ok(ExecutionResult {
            increment_id: increment.id,
            files_changed: implementation.iter().map(|fc| fc.path.clone()).collect(),
            tests_passed: increment.tests.len(),
            duration: std::time::Duration::from_secs(0), // TODO: Track actual duration
        })
    }

    /// Generate implementation for an increment
    async fn generate_implementation(&mut self, increment: &Increment) -> Result<Vec<FileChange>> {
        let mut changes = Vec::new();

        // Build prompt for LLM
        let prompt = self.build_generation_prompt(increment);

        // Generate code using LLM
        let response = self
            .llm_provider
            .complete_prompt(&prompt)
            .await
            .map_err(|e| IncrementalError::ExecutionError(e.to_string()))?;

        // Parse response into file changes
        let parsed_changes = self.parse_llm_response(&response, increment)?;
        changes.extend(parsed_changes);

        Ok(changes)
    }

    /// Build prompt for code generation
    fn build_generation_prompt(&self, increment: &Increment) -> String {
        let mut prompt = String::new();

        // Add context
        prompt.push_str("You are implementing a specific increment of functionality.\n\n");

        // Add specification
        prompt.push_str("## Specification\n");
        prompt.push_str(&format!("Description: {}\n", increment.specification.description));
        prompt.push_str(&format!("Requirements:\n"));
        for req in &increment.specification.requirements {
            prompt.push_str(&format!("- {}\n", req));
        }

        // Add examples if available
        if !increment.specification.examples.is_empty() {
            prompt.push_str("\n## Examples\n");
            for example in &increment.specification.examples {
                prompt.push_str(&format!("- {}\n", example));
            }
        }

        // Add approach
        prompt.push_str(&format!("\n## Approach\n{}\n", increment.implementation.approach));

        // Add test-first requirement if applicable
        if !increment.tests.is_empty() {
            prompt.push_str("\n## Tests to Pass\n");
            for test in &increment.tests {
                prompt.push_str(&format!("- {} ({})\n", test.name, test.command));
            }
            prompt.push_str("\nGenerate the minimal implementation that makes these tests pass.\n");
        }

        // Add output format instructions
        prompt.push_str("\n## Output Format\n");
        prompt.push_str("Provide the implementation as file changes in the following format:\n");
        prompt.push_str("```file:path/to/file.rs\n");
        prompt.push_str("// File content here\n");
        prompt.push_str("```\n");
        prompt.push_str("\nFor modifications, use:\n");
        prompt.push_str("```modify:path/to/file.rs:start_line:end_line\n");
        prompt.push_str("// New content for lines\n");
        prompt.push_str("```\n");

        prompt
    }

    /// Parse LLM response into file changes
    fn parse_llm_response(&self, response: &str, increment: &Increment) -> Result<Vec<FileChange>> {
        let mut changes = Vec::new();
        let mut current_change: Option<FileChange> = None;
        let mut content_lines = Vec::new();
        let mut in_code_block = false;

        for line in response.lines() {
            if line.starts_with("```file:") {
                // Save previous change if any
                if let Some(mut change) = current_change.take() {
                    change.content = content_lines.join("\n");
                    changes.push(change);
                    content_lines.clear();
                }

                // Start new file creation
                let path_str = line.trim_start_matches("```file:").trim();
                current_change = Some(FileChange {
                    path: self.config.project_root.join(path_str),
                    change_type: ChangeType::Create,
                    content: String::new(),
                    line_range: None,
                });
                in_code_block = true;
            } else if line.starts_with("```modify:") {
                // Save previous change if any
                if let Some(mut change) = current_change.take() {
                    change.content = content_lines.join("\n");
                    changes.push(change);
                    content_lines.clear();
                }

                // Parse modification directive
                let parts: Vec<&str> = line.trim_start_matches("```modify:").split(':').collect();
                if parts.len() >= 3 {
                    let path_str = parts[0];
                    let start_line = parts[1].parse().unwrap_or(1);
                    let end_line = parts[2].parse().unwrap_or(start_line);

                    current_change = Some(FileChange {
                        path: self.config.project_root.join(path_str),
                        change_type: ChangeType::Modify,
                        content: String::new(),
                        line_range: Some((start_line, end_line)),
                    });
                    in_code_block = true;
                }
            } else if line == "```" && in_code_block {
                // End of code block
                in_code_block = false;
            } else if in_code_block {
                // Collect content
                content_lines.push(line.to_string());
            }
        }

        // Save last change if any
        if let Some(mut change) = current_change.take() {
            change.content = content_lines.join("\n");
            changes.push(change);
        }

        // If no explicit changes were parsed, create a default one based on increment
        if changes.is_empty() {
            warn!("No explicit file changes found in LLM response, creating default file");
            let default_path = self.determine_default_path(increment);
            changes.push(FileChange {
                path: default_path,
                change_type: ChangeType::Create,
                content: response.to_string(),
                line_range: None,
            });
        }

        Ok(changes)
    }

    /// Determine default file path for increment
    fn determine_default_path(&self, increment: &Increment) -> PathBuf {
        let base_name = increment
            .specification
            .id
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>()
            .to_lowercase();

        self.config.project_root.join("src").join(format!("{}.rs", base_name))
    }

    /// Apply file changes to the filesystem
    async fn apply_changes(&self, changes: &[FileChange]) -> Result<()> {
        for change in changes {
            match change.change_type {
                ChangeType::Create => {
                    debug!("Creating file: {}", change.path.display());
                    if let Some(parent) = change.path.parent() {
                        fs::create_dir_all(parent).await?;
                    }
                    fs::write(&change.path, &change.content).await?;
                }
                ChangeType::Modify => {
                    debug!("Modifying file: {}", change.path.display());
                    if let Some((start, end)) = change.line_range {
                        self.modify_file_lines(&change.path, start, end, &change.content).await?;
                    } else {
                        fs::write(&change.path, &change.content).await?;
                    }
                }
                ChangeType::Delete => {
                    debug!("Deleting file: {}", change.path.display());
                    if change.path.exists() {
                        fs::remove_file(&change.path).await?;
                    }
                }
                ChangeType::Append => {
                    debug!("Appending to file: {}", change.path.display());
                    let mut existing = fs::read_to_string(&change.path).await.unwrap_or_default();
                    existing.push_str(&change.content);
                    fs::write(&change.path, existing).await?;
                }
                ChangeType::Replace => {
                    debug!("Replacing file: {}", change.path.display());
                    fs::write(&change.path, &change.content).await?;
                }
            }
        }

        Ok(())
    }

    /// Modify specific lines in a file
    async fn modify_file_lines(
        &self,
        path: &Path,
        start: usize,
        end: usize,
        new_content: &str,
    ) -> Result<()> {
        let content = fs::read_to_string(path).await?;
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

        // Replace lines (1-indexed to 0-indexed)
        let start_idx = start.saturating_sub(1);
        let end_idx = end.min(lines.len());

        // Remove old lines and insert new content
        lines.drain(start_idx..end_idx);
        for (idx, new_line) in new_content.lines().enumerate() {
            lines.insert(start_idx + idx, new_line.to_string());
        }

        let modified_content = lines.join("\n");
        fs::write(path, modified_content).await?;

        Ok(())
    }
}

/// Result of executing an increment
#[derive(Debug)]
pub struct ExecutionResult {
    pub increment_id: uuid::Uuid,
    pub files_changed: Vec<PathBuf>,
    pub tests_passed: usize,
    pub duration: std::time::Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_generation() {
        // Test prompt generation for increment
        // TODO: Add proper test implementation
    }
}
