use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::learning::learner::LearningEvent;
use crate::learning::pattern_extractor::{Pattern, PatternContext, PatternType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureAnalyzer {
    pub failures: Vec<FailureRecord>,
    pub anti_patterns: Vec<AntiPattern>,
    pub failure_causes: HashMap<String, Vec<FailureCause>>,
    pub guard_conditions: Vec<GuardCondition>,
    pub recovery_strategies: HashMap<String, RecoveryStrategy>,
}

impl FailureAnalyzer {
    pub fn new() -> Self {
        Self {
            failures: Vec::new(),
            anti_patterns: Vec::new(),
            failure_causes: HashMap::new(),
            guard_conditions: Vec::new(),
            recovery_strategies: HashMap::new(),
        }
    }

    pub fn analyze_failure(&mut self, event: &LearningEvent) -> FailureCause {
        let cause = self.identify_cause(event);

        let record = FailureRecord {
            id: Uuid::new_v4(),
            event_id: event.id,
            timestamp: event.timestamp,
            cause: cause.clone(),
            context: event.context.clone(),
            error_details: extract_error_details(event),
            stack_trace: extract_stack_trace(event),
        };

        self.failures.push(record);

        let cause_key = format!("{}_{}", event.context.project_type, event.context.language);
        self.failure_causes.entry(cause_key).or_insert_with(Vec::new).push(cause.clone());

        cause
    }

    pub fn identify_cause(&self, event: &LearningEvent) -> FailureCause {
        let error_msg = &event.outcome.message;
        let details = &event.outcome.details;

        if error_msg.contains("compilation") || error_msg.contains("syntax") {
            FailureCause::CompilationError {
                error_type: "syntax".to_string(),
                message: error_msg.clone(),
                location: extract_error_location(details),
            }
        } else if error_msg.contains("test") || error_msg.contains("assertion") {
            FailureCause::TestFailure {
                test_name: extract_test_name(details),
                assertion: extract_assertion(details),
                actual_vs_expected: extract_test_diff(details),
            }
        } else if error_msg.contains("timeout") || error_msg.contains("performance") {
            FailureCause::PerformanceIssue {
                metric: "execution_time".to_string(),
                threshold: extract_threshold(details),
                actual: extract_actual_value(details),
            }
        } else if error_msg.contains("security") || error_msg.contains("vulnerability") {
            FailureCause::SecurityViolation {
                vulnerability_type: extract_vulnerability_type(details),
                severity: extract_severity(details),
                cwe_id: extract_cwe_id(details),
            }
        } else if error_msg.contains("specification") || error_msg.contains("requirement") {
            FailureCause::SpecificationMismatch {
                expected: extract_expected_spec(details),
                actual: extract_actual_spec(details),
                missing_features: extract_missing_features(details),
            }
        } else {
            FailureCause::Unknown { message: error_msg.clone(), details: details.clone() }
        }
    }

    pub fn extract_anti_pattern(
        &self,
        event: &LearningEvent,
        cause: &FailureCause,
    ) -> Option<AntiPattern> {
        if let Some(implementation) = &event.implementation {
            let code = implementation
                .files
                .iter()
                .map(|f| f.content.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let problematic_code = identify_problematic_code(&code, cause);

            if !problematic_code.is_empty() {
                return Some(AntiPattern {
                    id: Uuid::new_v4(),
                    name: generate_anti_pattern_name(cause),
                    description: format!("Anti-pattern causing {}", cause.category()),
                    pattern_type: AntiPatternType::from_cause(cause),
                    problematic_code,
                    failure_cause: cause.clone(),
                    context: PatternContext::from_event_context(&event.context),
                    occurrences: 1,
                    severity: calculate_severity(cause),
                    fix_suggestion: generate_fix_suggestion(cause),
                    learned_at: Utc::now(),
                });
            }
        }

        None
    }

    pub fn extract_anti_pattern_from_event(&self, event: &LearningEvent) -> Option<AntiPattern> {
        let cause = self.identify_cause(event);
        self.extract_anti_pattern(event, &cause)
    }

    pub fn analyze_test_failure(&self, event: &LearningEvent) -> TestFailureCause {
        let test_output = extract_test_output(event);

        if test_output.contains("assertion failed") {
            TestFailureCause::AssertionFailure {
                test_name: extract_test_name(&event.outcome.details),
                expected: extract_expected_value(&test_output),
                actual: extract_actual_value(&event.outcome.details),
            }
        } else if test_output.contains("panic") {
            TestFailureCause::Panic {
                message: extract_panic_message(&test_output),
                location: extract_panic_location(&test_output),
            }
        } else if test_output.contains("timeout") {
            TestFailureCause::Timeout {
                duration: event.metrics.duration,
                limit: extract_timeout_limit(&test_output),
            }
        } else {
            TestFailureCause::Other { output: test_output }
        }
    }

    pub fn learn_from_test_failure(&mut self, event: &LearningEvent, cause: &TestFailureCause) {
        let pattern_name = match cause {
            TestFailureCause::AssertionFailure { test_name, .. } => {
                format!("assertion_failure_{}", test_name)
            }
            TestFailureCause::Panic { .. } => "panic_in_test".to_string(),
            TestFailureCause::Timeout { .. } => "test_timeout".to_string(),
            TestFailureCause::Other { .. } => "unknown_test_failure".to_string(),
        };

        let guard = GuardCondition {
            id: Uuid::new_v4(),
            condition_type: GuardType::TestValidation,
            check: format!("Ensure test {} passes", pattern_name),
            applies_to: vec![event.context.project_type.clone()],
            severity: Severity::High,
            created_at: Utc::now(),
        };

        self.guard_conditions.push(guard);
    }

    pub fn add_guard_conditions(&mut self, event: &LearningEvent, cause: &FailureCause) {
        let guards = generate_guard_conditions(cause, &event.context);
        self.guard_conditions.extend(guards);
    }

    pub fn adjust_decision_strategy(&mut self, cause: &FailureCause) {
        let strategy_key = cause.category();

        let strategy = self
            .recovery_strategies
            .entry(strategy_key.clone())
            .or_insert_with(|| RecoveryStrategy::default());

        strategy.failures += 1;
        strategy.last_failure = Utc::now();

        if strategy.failures > 3 {
            strategy.approach = RecoveryApproach::Conservative;
        }
    }

    pub fn get_recovery_strategy(&self, context: &str) -> Option<&RecoveryStrategy> {
        self.recovery_strategies.get(context)
    }

    pub fn find_similar_failures(&self, event: &LearningEvent) -> Vec<&FailureRecord> {
        self.failures
            .iter()
            .filter(|f| {
                f.context.language == event.context.language
                    && f.context.project_type == event.context.project_type
            })
            .collect()
    }

    pub fn get_anti_patterns_for_context(&self, context: &PatternContext) -> Vec<&AntiPattern> {
        self.anti_patterns.iter().filter(|ap| ap.context.matches(context)).collect()
    }

    pub fn should_avoid(&self, code: &str, context: &PatternContext) -> Vec<AntiPatternMatch> {
        let mut matches = Vec::new();

        for anti_pattern in self.get_anti_patterns_for_context(context) {
            if code.contains(&anti_pattern.problematic_code) {
                matches.push(AntiPatternMatch {
                    anti_pattern_id: anti_pattern.id,
                    name: anti_pattern.name.clone(),
                    severity: anti_pattern.severity.clone(),
                    fix_suggestion: anti_pattern.fix_suggestion.clone(),
                });
            }
        }

        matches
    }

    pub fn get_failure_statistics(&self) -> FailureStatistics {
        let mut cause_distribution = HashMap::new();

        for record in &self.failures {
            *cause_distribution.entry(record.cause.category()).or_insert(0) += 1;
        }

        FailureStatistics {
            total_failures: self.failures.len(),
            anti_patterns_identified: self.anti_patterns.len(),
            guard_conditions_active: self.guard_conditions.len(),
            cause_distribution,
            most_common_cause: self.get_most_common_cause(),
        }
    }

    fn get_most_common_cause(&self) -> Option<String> {
        let mut cause_counts = HashMap::new();

        for record in &self.failures {
            *cause_counts.entry(record.cause.category()).or_insert(0) += 1;
        }

        cause_counts.into_iter().max_by_key(|(_, count)| *count).map(|(cause, _)| cause)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureRecord {
    pub id: Uuid,
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub cause: FailureCause,
    pub context: crate::learning::learner::EventContext,
    pub error_details: String,
    pub stack_trace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailureCause {
    CompilationError { error_type: String, message: String, location: Option<String> },
    TestFailure { test_name: String, assertion: String, actual_vs_expected: String },
    PerformanceIssue { metric: String, threshold: String, actual: String },
    SecurityViolation { vulnerability_type: String, severity: String, cwe_id: Option<String> },
    SpecificationMismatch { expected: String, actual: String, missing_features: Vec<String> },
    Unknown { message: String, details: serde_json::Value },
}

impl FailureCause {
    pub fn category(&self) -> String {
        match self {
            Self::CompilationError { .. } => "compilation".to_string(),
            Self::TestFailure { .. } => "test".to_string(),
            Self::PerformanceIssue { .. } => "performance".to_string(),
            Self::SecurityViolation { .. } => "security".to_string(),
            Self::SpecificationMismatch { .. } => "specification".to_string(),
            Self::Unknown { .. } => "unknown".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiPattern {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub pattern_type: AntiPatternType,
    pub problematic_code: String,
    pub failure_cause: FailureCause,
    pub context: PatternContext,
    pub occurrences: u32,
    pub severity: Severity,
    pub fix_suggestion: String,
    pub learned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AntiPatternType {
    Performance,
    Security,
    Correctness,
    Maintainability,
    TestFailure,
}

impl AntiPatternType {
    fn from_cause(cause: &FailureCause) -> Self {
        match cause {
            FailureCause::PerformanceIssue { .. } => Self::Performance,
            FailureCause::SecurityViolation { .. } => Self::Security,
            FailureCause::TestFailure { .. } => Self::TestFailure,
            _ => Self::Correctness,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestFailureCause {
    AssertionFailure { test_name: String, expected: String, actual: String },
    Panic { message: String, location: String },
    Timeout { duration: std::time::Duration, limit: std::time::Duration },
    Other { output: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardCondition {
    pub id: Uuid,
    pub condition_type: GuardType,
    pub check: String,
    pub applies_to: Vec<String>,
    pub severity: Severity,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GuardType {
    PreCondition,
    PostCondition,
    Invariant,
    TestValidation,
    SecurityCheck,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStrategy {
    pub approach: RecoveryApproach,
    pub failures: u32,
    pub last_failure: DateTime<Utc>,
    pub suggestions: Vec<String>,
}

impl Default for RecoveryStrategy {
    fn default() -> Self {
        Self {
            approach: RecoveryApproach::Standard,
            failures: 0,
            last_failure: Utc::now(),
            suggestions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryApproach {
    Standard,
    Conservative,
    Experimental,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiPatternMatch {
    pub anti_pattern_id: Uuid,
    pub name: String,
    pub severity: Severity,
    pub fix_suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureStatistics {
    pub total_failures: usize,
    pub anti_patterns_identified: usize,
    pub guard_conditions_active: usize,
    pub cause_distribution: HashMap<String, usize>,
    pub most_common_cause: Option<String>,
}

fn extract_error_details(event: &LearningEvent) -> String {
    event.outcome.message.clone()
}

fn extract_stack_trace(event: &LearningEvent) -> Option<String> {
    event.outcome.details.get("stack_trace").and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn extract_error_location(details: &serde_json::Value) -> Option<String> {
    details.get("location").and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn extract_test_name(details: &serde_json::Value) -> String {
    details.get("test_name").and_then(|v| v.as_str()).unwrap_or("unknown").to_string()
}

fn extract_assertion(details: &serde_json::Value) -> String {
    details.get("assertion").and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn extract_test_diff(details: &serde_json::Value) -> String {
    details.get("diff").and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn extract_threshold(details: &serde_json::Value) -> String {
    details.get("threshold").and_then(|v| v.as_str()).unwrap_or("unknown").to_string()
}

fn extract_actual_value(details: &serde_json::Value) -> String {
    details.get("actual").and_then(|v| v.as_str()).unwrap_or("unknown").to_string()
}

fn extract_vulnerability_type(details: &serde_json::Value) -> String {
    details.get("vulnerability_type").and_then(|v| v.as_str()).unwrap_or("unknown").to_string()
}

fn extract_severity(details: &serde_json::Value) -> String {
    details.get("severity").and_then(|v| v.as_str()).unwrap_or("medium").to_string()
}

fn extract_cwe_id(details: &serde_json::Value) -> Option<String> {
    details.get("cwe_id").and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn extract_expected_spec(details: &serde_json::Value) -> String {
    details.get("expected").and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn extract_actual_spec(details: &serde_json::Value) -> String {
    details.get("actual").and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn extract_missing_features(details: &serde_json::Value) -> Vec<String> {
    details
        .get("missing_features")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
        .unwrap_or_default()
}

fn identify_problematic_code(code: &str, cause: &FailureCause) -> String {
    match cause {
        FailureCause::CompilationError { location, .. } => {
            if let Some(loc) = location {
                extract_code_at_location(code, loc)
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

fn extract_code_at_location(code: &str, location: &str) -> String {
    code.lines().find(|line| line.contains(location)).unwrap_or("").to_string()
}

fn generate_anti_pattern_name(cause: &FailureCause) -> String {
    format!("anti_{}", cause.category())
}

fn calculate_severity(cause: &FailureCause) -> Severity {
    match cause {
        FailureCause::SecurityViolation { .. } => Severity::Critical,
        FailureCause::CompilationError { .. } => Severity::High,
        FailureCause::TestFailure { .. } => Severity::Medium,
        _ => Severity::Low,
    }
}

fn generate_fix_suggestion(cause: &FailureCause) -> String {
    match cause {
        FailureCause::CompilationError { error_type, .. } => {
            format!("Fix {} error", error_type)
        }
        FailureCause::TestFailure { .. } => {
            "Update implementation to match test expectations".to_string()
        }
        FailureCause::PerformanceIssue { metric, .. } => {
            format!("Optimize {} performance", metric)
        }
        FailureCause::SecurityViolation { vulnerability_type, .. } => {
            format!("Address {} vulnerability", vulnerability_type)
        }
        _ => "Review and fix the issue".to_string(),
    }
}

fn extract_test_output(event: &LearningEvent) -> String {
    event
        .outcome
        .details
        .get("output")
        .and_then(|v| v.as_str())
        .unwrap_or(&event.outcome.message)
        .to_string()
}

fn extract_expected_value(output: &str) -> String {
    output.lines().find(|line| line.contains("expected")).unwrap_or("").to_string()
}

fn extract_panic_message(output: &str) -> String {
    output.lines().find(|line| line.contains("panicked at")).unwrap_or("").to_string()
}

fn extract_panic_location(output: &str) -> String {
    output.lines().find(|line| line.contains(".rs:")).unwrap_or("").to_string()
}

fn extract_timeout_limit(_output: &str) -> std::time::Duration {
    std::time::Duration::from_secs(60)
}

fn generate_guard_conditions(
    cause: &FailureCause,
    context: &crate::learning::learner::EventContext,
) -> Vec<GuardCondition> {
    let mut guards = Vec::new();

    match cause {
        FailureCause::CompilationError { .. } => {
            guards.push(GuardCondition {
                id: Uuid::new_v4(),
                condition_type: GuardType::PreCondition,
                check: "Validate syntax before compilation".to_string(),
                applies_to: vec![context.language.clone()],
                severity: Severity::High,
                created_at: Utc::now(),
            });
        }
        FailureCause::TestFailure { .. } => {
            guards.push(GuardCondition {
                id: Uuid::new_v4(),
                condition_type: GuardType::PostCondition,
                check: "Ensure all tests pass".to_string(),
                applies_to: vec![context.project_type.clone()],
                severity: Severity::Medium,
                created_at: Utc::now(),
            });
        }
        _ => {}
    }

    guards
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failure_analysis() {
        let mut analyzer = FailureAnalyzer::new();

        let event = LearningEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: crate::learning::learner::LearningEventType::ImplementationFailure,
            specification: Default::default(),
            implementation: Some(Default::default()),
            outcome: crate::learning::learner::Outcome {
                success: false,
                score: 0.2,
                message: "compilation error: syntax error".to_string(),
                details: serde_json::json!({
                    "location": "line 10"
                }),
            },
            metrics: Default::default(),
            context: crate::learning::learner::EventContext {
                project_type: "rust".to_string(),
                language: "rust".to_string(),
                framework: None,
                dependencies: vec![],
                environment: serde_json::json!({}),
            },
        };

        let cause = analyzer.analyze_failure(&event);

        assert!(matches!(cause, FailureCause::CompilationError { .. }));
        assert_eq!(analyzer.failures.len(), 1);
    }

    #[test]
    fn test_anti_pattern_extraction() {
        let analyzer = FailureAnalyzer::new();

        let cause = FailureCause::CompilationError {
            error_type: "syntax".to_string(),
            message: "unexpected token".to_string(),
            location: Some("line 5".to_string()),
        };

        let event = LearningEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: crate::learning::learner::LearningEventType::ImplementationFailure,
            specification: Default::default(),
            implementation: Some(crate::incremental::Implementation {
                files: vec![],
                estimated_complexity: crate::incremental::Complexity::Simple,
                approach: "test".to_string(),
                language: "rust".to_string(),
            }),
            outcome: crate::learning::learner::Outcome {
                success: false,
                score: 0.0,
                message: "compilation error".to_string(),
                details: serde_json::json!({}),
            },
            metrics: Default::default(),
            context: crate::learning::learner::EventContext {
                project_type: "rust".to_string(),
                language: "rust".to_string(),
                framework: None,
                dependencies: vec![],
                environment: serde_json::json!({}),
            },
        };

        let anti_pattern = analyzer.extract_anti_pattern(&event, &cause);
        assert!(anti_pattern.is_some());
    }
}
