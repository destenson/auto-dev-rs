#![allow(unused)]
//! Safety monitoring for self-development activities

use super::{Result, SafetyLevel};
use crate::safety::ValidationReport;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

const FAILURE_ESCALATION_THRESHOLD: usize = 2;
const SUCCESS_RELAXATION_THRESHOLD: usize = 4;
const HISTORY_LIMIT: usize = 50;

#[async_trait]
pub trait SafetyAuthority: Send + Sync {
    async fn validate_change(&self, change_id: &str) -> Result<ValidationReport>;
    async fn update_safety_level(&self, level: SafetyLevel) -> Result<()>;
    async fn current_safety_level(&self) -> Result<SafetyLevel>;
}

pub struct SafetyMonitor {
    authority: Arc<dyn SafetyAuthority>,
    safety_level: Arc<RwLock<SafetyLevel>>,
    validation_history: Arc<RwLock<Vec<ValidationRecord>>>,
    consecutive_failures: Arc<RwLock<usize>>,
    consecutive_successes: Arc<RwLock<usize>>,
    failure_threshold: usize,
    success_threshold: usize,
}

#[derive(Debug, Clone)]
struct ValidationRecord {
    change_id: String,
    timestamp: SystemTime,
    report: ValidationReport,
}

impl SafetyMonitor {
    pub fn new(authority: Arc<dyn SafetyAuthority>, safety_level: SafetyLevel) -> Self {
        Self {
            authority,
            safety_level: Arc::new(RwLock::new(safety_level)),
            validation_history: Arc::new(RwLock::new(Vec::new())),
            consecutive_failures: Arc::new(RwLock::new(0)),
            consecutive_successes: Arc::new(RwLock::new(0)),
            failure_threshold: FAILURE_ESCALATION_THRESHOLD,
            success_threshold: SUCCESS_RELAXATION_THRESHOLD,
        }
    }

    pub async fn validate_change(&self, change_id: &str) -> Result<bool> {
        debug!("Validating change: {}", change_id);

        let report = self.authority.validate_change(change_id).await?;
        let passed = report.passed;

        self.record_history(change_id, report.clone()).await;

        if passed {
            self.handle_success().await?;
            info!("Change {} passed safety validation", change_id);
        } else {
            self.handle_failure(change_id, &report).await?;
        }

        Ok(passed)
    }

    pub async fn set_manual_level(&self, level: SafetyLevel) -> Result<()> {
        self.authority.update_safety_level(level.clone()).await?;
        *self.safety_level.write().await = level;
        self.reset_counters().await;
        Ok(())
    }

    pub async fn current_level(&self) -> SafetyLevel {
        self.safety_level.read().await.clone()
    }

    pub async fn get_validation_history(&self) -> Vec<(String, bool)> {
        self.validation_history
            .read()
            .await
            .iter()
            .map(|r| (r.change_id.clone(), r.report.passed))
            .collect()
    }

    pub async fn clear_validation_history(&self) {
        self.validation_history.write().await.clear();
    }

    async fn record_history(&self, change_id: &str, report: ValidationReport) {
        let mut history = self.validation_history.write().await;
        history.push(ValidationRecord {
            change_id: change_id.to_string(),
            timestamp: SystemTime::now(),
            report,
        });

        if history.len() > HISTORY_LIMIT {
            let excess = history.len() - HISTORY_LIMIT;
            history.drain(0..excess);
        }
    }

    async fn handle_failure(&self, change_id: &str, report: &ValidationReport) -> Result<()> {
        warn!(
            "Change {} failed safety validation with risk {:?}: {:?}",
            change_id, report.risk_level, report.recommendations
        );

        {
            let mut successes = self.consecutive_successes.write().await;
            *successes = 0;
        }

        let mut failures = self.consecutive_failures.write().await;
        *failures += 1;

        if *failures >= self.failure_threshold {
            self.escalate_safety_level().await?;
            *failures = 0;
        }

        Ok(())
    }

    async fn handle_success(&self) -> Result<()> {
        {
            let mut failures = self.consecutive_failures.write().await;
            *failures = 0;
        }

        let mut successes = self.consecutive_successes.write().await;
        *successes += 1;

        if *successes >= self.success_threshold {
            self.relax_safety_level().await?;
            *successes = 0;
        }

        Ok(())
    }

    async fn escalate_safety_level(&self) -> Result<()> {
        let mut level = self.safety_level.write().await;
        let new_level = match *level {
            SafetyLevel::Permissive => SafetyLevel::Standard,
            SafetyLevel::Standard => SafetyLevel::Strict,
            SafetyLevel::Strict => SafetyLevel::Strict,
        };

        if new_level != *level {
            warn!(
                "Escalating safety level from {:?} to {:?} after repeated failures",
                *level, new_level
            );
            self.authority.update_safety_level(new_level.clone()).await?;
            *level = new_level;
            self.reset_counters().await;
        }

        Ok(())
    }

    async fn relax_safety_level(&self) -> Result<()> {
        let mut level = self.safety_level.write().await;
        let new_level = match *level {
            SafetyLevel::Strict => SafetyLevel::Standard,
            SafetyLevel::Standard => SafetyLevel::Permissive,
            SafetyLevel::Permissive => SafetyLevel::Permissive,
        };

        if new_level != *level {
            info!(
                "Relaxing safety level from {:?} to {:?} after sustained success",
                *level, new_level
            );
            self.authority.update_safety_level(new_level.clone()).await?;
            *level = new_level;
            self.reset_counters().await;
        }

        Ok(())
    }

    async fn reset_counters(&self) {
        *self.consecutive_failures.write().await = 0;
        *self.consecutive_successes.write().await = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::safety::{RiskLevel, ValidationReport};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    struct MockAuthority {
        level: Arc<RwLock<SafetyLevel>>,
        report: Arc<RwLock<ValidationReport>>,
    }

    impl MockAuthority {
        fn new(level: SafetyLevel, report: ValidationReport) -> Self {
            Self { level: Arc::new(RwLock::new(level)), report: Arc::new(RwLock::new(report)) }
        }

        async fn set_report(&self, report: ValidationReport) {
            *self.report.write().await = report;
        }
    }

    #[async_trait]
    impl SafetyAuthority for MockAuthority {
        async fn validate_change(&self, _change_id: &str) -> Result<ValidationReport> {
            Ok(self.report.read().await.clone())
        }

        async fn update_safety_level(&self, level: SafetyLevel) -> Result<()> {
            *self.level.write().await = level;
            Ok(())
        }

        async fn current_safety_level(&self) -> Result<SafetyLevel> {
            Ok(self.level.read().await.clone())
        }
    }

    fn passing_report() -> ValidationReport {
        ValidationReport {
            passed: true,
            gate_results: vec![],
            duration_ms: 0,
            risk_level: RiskLevel::Low,
            recommendations: vec![],
        }
    }

    fn failing_report() -> ValidationReport {
        ValidationReport {
            passed: false,
            gate_results: vec![],
            duration_ms: 0,
            risk_level: RiskLevel::High,
            recommendations: vec!["Fix issues".to_string()],
        }
    }

    #[tokio::test]
    async fn test_safety_monitor_creation() {
        let authority = Arc::new(MockAuthority::new(SafetyLevel::Standard, passing_report()));
        let monitor = SafetyMonitor::new(authority, SafetyLevel::Standard);

        assert!(matches!(monitor.current_level().await, SafetyLevel::Standard));
    }

    #[tokio::test]
    async fn test_validation_success_records_history() {
        let authority = Arc::new(MockAuthority::new(SafetyLevel::Standard, passing_report()));
        let monitor = SafetyMonitor::new(authority, SafetyLevel::Standard);

        let result = monitor.validate_change("change_a").await.unwrap();
        assert!(result);

        let history = monitor.get_validation_history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0], ("change_a".to_string(), true));
    }

    #[tokio::test]
    async fn test_failure_escalates_level() {
        let authority = Arc::new(MockAuthority::new(SafetyLevel::Standard, failing_report()));
        let monitor = SafetyMonitor::new(authority.clone(), SafetyLevel::Standard);

        monitor.validate_change("change_a").await.unwrap();
        authority.set_report(failing_report()).await;
        monitor.validate_change("change_b").await.unwrap();

        assert!(matches!(monitor.current_level().await, SafetyLevel::Strict));
    }
}
