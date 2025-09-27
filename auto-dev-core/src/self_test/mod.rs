//! Self-testing framework for auto-dev-rs
//!
//! Provides comprehensive testing capabilities to validate self-modifications
//! before they are applied, ensuring system stability during self-development.

pub mod compatibility;
pub mod performance;
pub mod regression;
pub mod safety;
pub mod sandbox_env;
pub mod test_runner;

pub use compatibility::{CompatibilityChecker, InterfaceChange};
pub use performance::{BenchmarkResult, PerformanceBenchmark};
pub use regression::{RegressionSuite, RegressionTest};
pub use safety::{SafetyCheck, SafetyValidator};
pub use sandbox_env::{SandboxConfig, TestSandbox};
pub use test_runner::{SelfTestRunner, TestConfig, TestLevel, TestResult};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SelfTestError {
    #[error("Test execution failed: {0}")]
    Execution(String),

    #[error("Sandbox error: {0}")]
    Sandbox(String),

    #[error("Compatibility check failed: {0}")]
    Compatibility(String),

    #[error("Regression detected: {0}")]
    Regression(String),

    #[error("Performance degradation: {0}")]
    Performance(String),

    #[error("Safety violation: {0}")]
    Safety(String),

    #[error("Configuration error: {0}")]
    Configuration(String),
}

pub type Result<T> = std::result::Result<T, SelfTestError>;

/// Configuration for the self-test framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfTestConfig {
    pub enabled: bool,
    pub test_levels: Vec<TestLevel>,
    pub parallel_execution: bool,
    pub timeout_seconds: u64,
    pub fail_fast: bool,
    pub coverage_threshold: f32,
    pub performance_baseline: Option<String>,
}

impl Default for SelfTestConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            test_levels: vec![
                TestLevel::Syntax,
                TestLevel::Unit,
                TestLevel::Integration,
                TestLevel::Compatibility,
                TestLevel::Regression,
            ],
            parallel_execution: true,
            timeout_seconds: 300, // 5 minutes
            fail_fast: false,
            coverage_threshold: 0.8,
            performance_baseline: None,
        }
    }
}

/// Initialize the self-test framework
pub async fn initialize(config: SelfTestConfig) -> Result<SelfTestRunner> {
    SelfTestRunner::new(config).await
}

/// Test report with detailed results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestReport {
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration_ms: u64,
    pub coverage: f32,
    pub test_results: Vec<TestResult>,
    pub recommendations: Vec<String>,
}

impl TestReport {
    pub fn success_rate(&self) -> f32 {
        if self.total_tests == 0 { 0.0 } else { self.passed as f32 / self.total_tests as f32 }
    }

    pub fn is_passing(&self) -> bool {
        self.failed == 0 && self.passed > 0
    }
}
