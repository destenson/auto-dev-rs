#![allow(unused)]
use std::time::Instant;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::{SelfTestError, sandbox_env::TestSandbox};

/// Regression test suite for core functionality
pub struct RegressionSuite {
    test_cases: Vec<RegressionTest>,
}

impl RegressionSuite {
    pub fn new() -> Self {
        Self {
            test_cases: Self::default_test_cases(),
        }
    }
    
    /// Get all regression test cases
    pub fn get_test_cases(&self) -> Vec<RegressionTest> {
        self.test_cases.clone()
    }
    
    /// Run a specific regression test
    pub async fn run_test(&self, test: &RegressionTest, sandbox: &mut TestSandbox) -> Result<TestExecutionResult, SelfTestError> {
        info!("Running regression test: {}", test.name);
        let start = Instant::now();
        
        // Setup test environment
        if let Some(setup) = &test.setup_commands {
            for cmd in setup {
                let args: Vec<&str> = cmd.args.iter().map(|s| s.as_str()).collect();
                let result = sandbox.run_command(&cmd.program, &args).await?;
                if !result.success {
                    return Ok(TestExecutionResult {
                        passed: false,
                        duration_ms: start.elapsed().as_millis() as u64,
                        message: format!("Setup failed: {}", result.stderr),
                    });
                }
            }
        }
        
        // Run the test command
        let test_args: Vec<&str> = test.command.args.iter().map(|s| s.as_str()).collect();
        let test_result = sandbox.run_command(&test.command.program, &test_args).await?;
        
        // Validate result
        let passed = self.validate_result(&test_result, &test.expected);
        
        // Cleanup if needed
        if let Some(cleanup) = &test.cleanup_commands {
            for cmd in cleanup {
                let cleanup_args: Vec<&str> = cmd.args.iter().map(|s| s.as_str()).collect();
                let _ = sandbox.run_command(&cmd.program, &cleanup_args).await;
            }
        }
        
        Ok(TestExecutionResult {
            passed,
            duration_ms: start.elapsed().as_millis() as u64,
            message: if passed {
                "Test passed".to_string()
            } else {
                format!("Expected: {:?}, Got exit code: {:?}", test.expected.exit_code, test_result.exit_code)
            },
        })
    }
    
    /// Validate test result against expected outcome
    fn validate_result(&self, actual: &super::sandbox_env::CommandResult, expected: &ExpectedResult) -> bool {
        // Check exit code
        if let Some(expected_code) = expected.exit_code {
            if actual.exit_code != Some(expected_code) {
                return false;
            }
        }
        
        // Check stdout contains expected strings
        if let Some(ref contains) = expected.stdout_contains {
            for s in contains {
                if !actual.stdout.contains(s) {
                    return false;
                }
            }
        }
        
        // Check stderr
        if let Some(ref stderr_contains) = expected.stderr_contains {
            for s in stderr_contains {
                if !actual.stderr.contains(s) {
                    return false;
                }
            }
        }
        
        // Check that stderr doesn't contain error patterns
        if let Some(ref not_contains) = expected.stderr_not_contains {
            for s in not_contains {
                if actual.stderr.contains(s) {
                    return false;
                }
            }
        }
        
        true
    }
    
    /// Default regression test cases
    fn default_test_cases() -> Vec<RegressionTest> {
        vec![
            // CLI basic functionality
            RegressionTest {
                name: "CLI Help".to_string(),
                description: "CLI shows help text".to_string(),
                command: TestCommand {
                    program: "cargo".to_string(),
                    args: vec!["run".to_string(), "--".to_string(), "--help".to_string()],
                },
                expected: ExpectedResult {
                    exit_code: Some(0),
                    stdout_contains: Some(vec!["auto-dev".to_string(), "Usage".to_string()]),
                    stderr_contains: None,
                    stderr_not_contains: Some(vec!["error".to_string(), "panic".to_string()]),
                },
                setup_commands: None,
                cleanup_commands: None,
            },
            
            // Module loading
            RegressionTest {
                name: "Module Loading".to_string(),
                description: "Can load native modules".to_string(),
                command: TestCommand {
                    program: "cargo".to_string(),
                    args: vec!["test".to_string(), "--".to_string(), "module_loading".to_string()],
                },
                expected: ExpectedResult {
                    exit_code: Some(0),
                    stdout_contains: Some(vec!["test result: ok".to_string()]),
                    stderr_contains: None,
                    stderr_not_contains: Some(vec!["failed".to_string()]),
                },
                setup_commands: None,
                cleanup_commands: None,
            },
            
            // Monitoring functionality
            RegressionTest {
                name: "File Monitoring".to_string(),
                description: "File monitoring detects changes".to_string(),
                command: TestCommand {
                    program: "cargo".to_string(),
                    args: vec!["test".to_string(), "--".to_string(), "monitor::tests::test_file_change_detection".to_string()],
                },
                expected: ExpectedResult {
                    exit_code: Some(0),
                    stdout_contains: Some(vec!["test result: ok".to_string()]),
                    stderr_contains: None,
                    stderr_not_contains: Some(vec!["failed".to_string()]),
                },
                setup_commands: None,
                cleanup_commands: None,
            },
            
            // LLM integration
            RegressionTest {
                name: "LLM Provider Loading".to_string(),
                description: "Can initialize LLM providers".to_string(),
                command: TestCommand {
                    program: "cargo".to_string(),
                    args: vec!["test".to_string(), "--".to_string(), "llm::tests".to_string()],
                },
                expected: ExpectedResult {
                    exit_code: Some(0),
                    stdout_contains: Some(vec!["test result: ok".to_string()]),
                    stderr_contains: None,
                    stderr_not_contains: Some(vec!["failed".to_string()]),
                },
                setup_commands: None,
                cleanup_commands: None,
            },
            
            // Safety gates
            RegressionTest {
                name: "Safety Validation".to_string(),
                description: "Safety gates prevent dangerous operations".to_string(),
                command: TestCommand {
                    program: "cargo".to_string(),
                    args: vec!["test".to_string(), "--".to_string(), "safety::tests::test_gate_validation".to_string()],
                },
                expected: ExpectedResult {
                    exit_code: Some(0),
                    stdout_contains: Some(vec!["test result: ok".to_string()]),
                    stderr_contains: None,
                    stderr_not_contains: Some(vec!["failed".to_string()]),
                },
                setup_commands: None,
                cleanup_commands: None,
            },
        ]
    }
    
    /// Add a custom regression test
    pub fn add_test(&mut self, test: RegressionTest) {
        self.test_cases.push(test);
    }
}

/// A regression test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionTest {
    pub name: String,
    pub description: String,
    pub command: TestCommand,
    pub expected: ExpectedResult,
    pub setup_commands: Option<Vec<TestCommand>>,
    pub cleanup_commands: Option<Vec<TestCommand>>,
}

/// Command to execute for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCommand {
    pub program: String,
    pub args: Vec<String>,
}

/// Expected test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedResult {
    pub exit_code: Option<i32>,
    pub stdout_contains: Option<Vec<String>>,
    pub stderr_contains: Option<Vec<String>>,
    pub stderr_not_contains: Option<Vec<String>>,
}

/// Result of running a test
pub struct TestExecutionResult {
    pub passed: bool,
    pub duration_ms: u64,
    pub message: String,
}
