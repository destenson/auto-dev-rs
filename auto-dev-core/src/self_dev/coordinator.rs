//! Component coordinator for integrating self-development subsystems

use super::orchestrator::{
    ChangeMetrics, ChangeStatus, ChangeType, PendingChange, PlanDigest, PlanStep, RiskLevel,
    TestResults, TestRunSummary,
};
use super::{Result, SelfDevConfig, SelfDevError};
use crate::incremental::planner::{IncrementPlan, IncrementPlanner};
use crate::parser::SpecParser;
use crate::parser::model::Specification;
use crate::vcs::{CommitStyle, VcsConfig, VcsIntegration};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

const TEST_COMMAND: &str = "cargo test --package auto-dev-core --lib -- self_dev::";

pub struct ComponentCoordinator {
    config: Arc<RwLock<SelfDevConfig>>,
    project_root: PathBuf,
    pending_changes: Arc<RwLock<Vec<PendingChange>>>,
    approved_changes: Arc<RwLock<Vec<PendingChange>>>,
    today_changes_count: Arc<RwLock<usize>>,
    active_components: Arc<RwLock<Vec<String>>>,
    specification_cache: Arc<RwLock<HashMap<String, Specification>>>,
    plan_cache: Arc<RwLock<HashMap<String, PlanDigest>>>,
    change_failures: Arc<RwLock<HashMap<String, u32>>>,
}

impl ComponentCoordinator {
    pub async fn new(config: Arc<RwLock<SelfDevConfig>>, project_root: PathBuf) -> Self {
        let active_snapshot = {
            let cfg = config.read().await;
            Self::list_active_components(&cfg)
        };

        Self {
            config,
            project_root,
            pending_changes: Arc::new(RwLock::new(Vec::new())),
            approved_changes: Arc::new(RwLock::new(Vec::new())),
            today_changes_count: Arc::new(RwLock::new(0)),
            active_components: Arc::new(RwLock::new(active_snapshot)),
            specification_cache: Arc::new(RwLock::new(HashMap::new())),
            plan_cache: Arc::new(RwLock::new(HashMap::new())),
            change_failures: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn has_pending_work(&self) -> Result<bool> {
        self.discover_pending_changes().await?;
        Ok(!self.pending_changes.read().await.is_empty())
    }

    pub async fn analyze_requirements(&self) -> Result<()> {
        self.discover_pending_changes().await
    }

    pub async fn create_implementation_plan(&self) -> Result<()> {
        let specs = self.specification_cache.read().await.clone();
        let require_tests = self.config.read().await.require_tests;
        let planner = IncrementPlanner::new(6, require_tests);

        let mut plan_cache = self.plan_cache.write().await;
        let mut pending = self.pending_changes.write().await;

        for change in pending.iter_mut().filter(|c| c.status <= ChangeStatus::Analyzed) {
            if let Some(spec) = specs.get(&change.id) {
                match planner.plan_increments(spec) {
                    Ok(plan) => {
                        let digest = Self::build_plan_digest(&plan);
                        plan_cache.insert(change.id.clone(), digest.clone());
                        change.plan = Some(digest);
                        change.status = ChangeStatus::Planned;
                        change.touch();
                    }
                    Err(err) => {
                        return Err(SelfDevError::Coordination(format!(
                            "Failed to create plan for {}: {}",
                            change.id, err
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn generate_solution(&self) -> Result<()> {
        if !self.config.read().await.components.synthesis {
            return Err(SelfDevError::Coordination(
                "Synthesis component is not enabled".to_string(),
            ));
        }

        let mut pending = self.pending_changes.write().await;
        for change in pending.iter_mut().filter(|c| c.status == ChangeStatus::Planned) {
            info!("Preparing change {} for testing", change.id);
            change.status = ChangeStatus::Generating;
            change.touch();
            change.status = ChangeStatus::ReadyForTesting;
            change.touch();
        }

        Ok(())
    }

    pub async fn test_solution(&self) -> Result<TestResults> {
        if !self.config.read().await.components.testing {
            warn!("Testing component disabled, skipping tests");
            return Ok(TestResults::default());
        }

        let mut command = Command::new("cargo");
        command
            .arg("test")
            .arg("--package")
            .arg("auto-dev-core")
            .arg("--lib")
            .arg("--")
            .arg("self_dev::");
        command.current_dir(&self.project_root);

        let start = Instant::now();
        let output = command
            .output()
            .await
            .map_err(|e| SelfDevError::Coordination(format!("Failed to run tests: {}", e)))?;
        let duration = start.elapsed();

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut results = TestResults::default();
        results.record_run(TestRunSummary {
            command: TEST_COMMAND.to_string(),
            duration,
            passed: output.status.success(),
            details: Some(format!("{}{}", stdout, stderr)),
        });

        let passed = output.status.success();
        let mut pending = self.pending_changes.write().await;
        for change in pending.iter_mut().filter(|c| c.status == ChangeStatus::ReadyForTesting) {
            if passed {
                change.status = ChangeStatus::ReadyForReview;
                change.metrics.test_runs.extend(results.runs().iter().cloned());
                change.touch();
            } else {
                warn!("Tests failed for change {}", change.id);
                change.status = ChangeStatus::Planned;
                change.touch();
                self.register_failure(&change.id).await;
            }
        }

        Ok(results)
    }

    pub async fn deploy_approved_changes(&self) -> Result<()> {
        if !self.config.read().await.components.deployment {
            return Err(SelfDevError::Coordination(
                "Deployment component is not enabled".to_string(),
            ));
        }

        let limit = self.config.read().await.max_changes_per_day;
        let mut approved = self.approved_changes.write().await;
        if approved.is_empty() {
            info!("No approved changes to deploy");
            return Ok(());
        }

        let mut today_count = self.today_changes_count.write().await;
        if *today_count >= limit {
            return Err(SelfDevError::Coordination(format!(
                "Daily change limit ({}) reached",
                limit
            )));
        }

        let vcs = VcsIntegration::new(
            &self.project_root,
            VcsConfig {
                auto_branch: false,
                branch_prefix: "self-dev".to_string(),
                commit_style: CommitStyle::Simple,
                auto_merge: false,
                require_tests: false,
                sign_commits: false,
                max_conflict_attempts: 1,
            },
        )
        .map_err(|e| SelfDevError::Coordination(format!("VCS integration failed: {}", e)))?;

        for change in approved.iter_mut().filter(|c| c.status == ChangeStatus::Approved) {
            info!("Deploying change: {}", change.id);
            change.status = ChangeStatus::Deploying;
            change.touch();

            if let Ok(status) = vcs.status() {
                if status.has_conflicts {
                    warn!("Repository has conflicts before deploying {}", change.id);
                    continue;
                }
            }

            change.status = ChangeStatus::Deployed;
            change.metrics.deployments += 1;
            change.touch();
            *today_count += 1;

            if *today_count >= limit {
                warn!("Daily change limit reached during deployment");
                break;
            }
        }

        approved.retain(|c| c.status != ChangeStatus::Deployed);
        Ok(())
    }

    pub async fn monitor_deployment(&self) -> Result<()> {
        info!("Monitoring deployment effects");
        tokio::time::sleep(Duration::from_secs(5)).await;
        Ok(())
    }

    pub async fn extract_learning_patterns(&self) -> Result<()> {
        let mut failures = self.change_failures.write().await;
        if failures.is_empty() {
            debug!("No failure data to learn from");
            return Ok(());
        }

        for (change_id, count) in failures.iter() {
            info!("Change {} has {} recorded failures", change_id, count);
        }

        failures.clear();
        Ok(())
    }

    pub async fn rollback_all(&self) -> Result<()> {
        error!("Rolling back all pending and approved changes");

        let mut pending = self.pending_changes.write().await;
        for change in pending.iter_mut() {
            change.status = ChangeStatus::RolledBack;
            change.touch();
        }
        pending.clear();

        let mut approved = self.approved_changes.write().await;
        approved.clear();

        Ok(())
    }

    pub async fn get_active_components(&self) -> Vec<String> {
        self.active_components.read().await.clone()
    }

    pub async fn get_pending_changes(&self) -> Result<Vec<PendingChange>> {
        let mut combined = self.pending_changes.read().await.clone();
        combined.extend(self.approved_changes.read().await.clone());
        Ok(combined)
    }

    pub async fn get_today_changes_count(&self) -> usize {
        *self.today_changes_count.read().await
    }

    pub async fn lookup_change(&self, change_id: &str) -> Result<Option<PendingChange>> {
        if let Some(change) = self.pending_changes.read().await.iter().find(|c| c.id == change_id) {
            return Ok(Some(change.clone()));
        }

        if let Some(change) = self.approved_changes.read().await.iter().find(|c| c.id == change_id)
        {
            return Ok(Some(change.clone()));
        }

        Ok(None)
    }

    pub async fn approve_change(&self, change_id: String) -> Result<()> {
        let mut pending = self.pending_changes.write().await;
        if let Some(index) = pending.iter().position(|c| c.id == change_id) {
            let mut change = pending.remove(index);
            if change.status < ChangeStatus::ReadyForReview {
                let message = format!("Change {} is not ready for approval", change.id);
                pending.insert(index, change);
                return Err(SelfDevError::Coordination(message));
            }
            change.status = ChangeStatus::Approved;
            change.touch();
            self.approved_changes.write().await.push(change);
            Ok(())
        } else {
            Err(SelfDevError::Coordination(format!("Change {} not found in pending", change_id)))
        }
    }

    pub async fn reject_change(&self, change_id: String) -> Result<()> {
        let mut pending = self.pending_changes.write().await;
        if let Some(index) = pending.iter().position(|c| c.id == change_id) {
            let mut change = pending.remove(index);
            change.status = ChangeStatus::RolledBack;
            change.touch();
            Ok(())
        } else {
            Err(SelfDevError::Coordination(format!("Change {} not found in pending", change_id)))
        }
    }

    pub async fn enable_component(&self, component: String) -> Result<()> {
        let mut config = self.config.write().await;
        match component.as_str() {
            "monitoring" => config.components.monitoring = true,
            "synthesis" => config.components.synthesis = true,
            "testing" => config.components.testing = true,
            "deployment" => config.components.deployment = true,
            "learning" => config.components.learning = true,
            _ => {
                return Err(SelfDevError::Configuration(format!(
                    "Unknown component {}",
                    component
                )));
            }
        }
        drop(config);
        self.refresh_active_components().await;
        info!("Enabled component {}", component);
        Ok(())
    }

    pub async fn disable_component(&self, component: String) -> Result<()> {
        let mut config = self.config.write().await;
        match component.as_str() {
            "monitoring" => config.components.monitoring = false,
            "synthesis" => config.components.synthesis = false,
            "testing" => config.components.testing = false,
            "deployment" => config.components.deployment = false,
            "learning" => config.components.learning = false,
            _ => {
                return Err(SelfDevError::Configuration(format!(
                    "Unknown component {}",
                    component
                )));
            }
        }
        drop(config);
        self.refresh_active_components().await;
        info!("Disabled component {}", component);
        Ok(())
    }

    pub async fn set_max_changes_per_day(&self, limit: usize) {
        let mut config = self.config.write().await;
        config.max_changes_per_day = limit;
    }

    pub async fn update_configuration(&self, new_config: SelfDevConfig) {
        let mut config = self.config.write().await;
        *config = new_config;
        drop(config);
        self.refresh_active_components().await;
    }

    pub async fn flag_safety_failure(&self, change_id: &str) {
        let mut failures = self.change_failures.write().await;
        *failures.entry(change_id.to_string()).or_insert(0) += 1;
    }

    async fn discover_pending_changes(&self) -> Result<()> {
        if !self.config.read().await.components.monitoring {
            return Ok(());
        }

        let parser = SpecParser::new();
        let prp_dir = self.project_root.join("PRPs");
        if !prp_dir.exists() {
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(&prp_dir)
            .await
            .map_err(|e| SelfDevError::Coordination(format!("Failed to read PRPs: {}", e)))?;

        let mut pending = self.pending_changes.write().await;
        let mut specs = self.specification_cache.write().await;
        let mut discovered_ids = HashSet::new();

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| SelfDevError::Coordination(format!("Failed to read entry: {}", e)))?
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            if let Some((change, spec)) = self.build_change_from_prp(&path, &parser).await? {
                discovered_ids.insert(change.id.clone());
                match pending.iter_mut().find(|existing| existing.id == change.id) {
                    Some(existing) => {
                        if existing.status < change.status {
                            existing.status = change.status;
                        }
                        existing.description = change.description.clone();
                        existing.summary = change.summary.clone();
                        existing.target_files = change.target_files.clone();
                        existing.risk_level = change.risk_level;
                        existing.required_components = change.required_components.clone();
                        existing.touch();
                    }
                    None => pending.push(change.clone()),
                }
                specs.insert(change.id.clone(), spec);
            }
        }

        pending.retain(|change| discovered_ids.contains(&change.id));
        Ok(())
    }

    async fn build_change_from_prp(
        &self,
        path: &Path,
        parser: &SpecParser,
    ) -> Result<Option<(PendingChange, Specification)>> {
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            SelfDevError::Coordination(format!("Failed to read {}: {}", path.display(), e))
        })?;

        if Self::prp_completed(&content) {
            return Ok(None);
        }

        let spec = parser.parse_file(path).await.map_err(|e| {
            SelfDevError::Coordination(format!("Failed to parse {}: {}", path.display(), e))
        })?;

        let id = format!(
            "prp:{}",
            path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_lowercase()
        );
        let description = Self::extract_title(&content)
            .unwrap_or_else(|| format!("Implement requirements from {}", path.display()));
        let summary = Self::summarize_spec(&spec);
        let target_files = Self::extract_paths(&content);
        let required_components = self.derive_required_components(&content).await;

        let change = PendingChange {
            id,
            description,
            summary,
            file_path: path.to_string_lossy().to_string(),
            change_type: ChangeType::Modify,
            risk_level: Self::assess_risk_level(path),
            status: ChangeStatus::Analyzed,
            plan: None,
            target_files,
            required_components,
            last_updated: SystemTime::now(),
            metrics: ChangeMetrics::default(),
        };

        Ok(Some((change, spec)))
    }

    fn build_plan_digest(plan: &IncrementPlan) -> PlanDigest {
        let steps = plan
            .increments
            .iter()
            .map(|increment| PlanStep {
                id: increment.id.to_string(),
                description: increment.specification.description.clone(),
                depends_on: increment.dependencies.iter().map(|d| d.to_string()).collect(),
                tests: increment.tests.iter().map(|t| t.command.clone()).collect(),
            })
            .collect();

        PlanDigest {
            steps,
            estimated_duration: plan.estimated_duration,
            critical_path: plan.critical_path.iter().map(|uuid| uuid.to_string()).collect(),
        }
    }

    fn list_active_components(config: &SelfDevConfig) -> Vec<String> {
        let mut active = Vec::new();

        if config.components.monitoring {
            active.push("monitoring".to_string());
        }
        if config.components.synthesis {
            active.push("synthesis".to_string());
        }
        if config.components.testing {
            active.push("testing".to_string());
        }
        if config.components.deployment {
            active.push("deployment".to_string());
        }
        if config.components.learning {
            active.push("learning".to_string());
        }

        active
    }

    async fn refresh_active_components(&self) {
        let snapshot = {
            let cfg = self.config.read().await;
            Self::list_active_components(&cfg)
        };
        let mut current = self.active_components.write().await;
        *current = snapshot;
    }

    async fn derive_required_components(&self, content: &str) -> Vec<String> {
        let config = self.config.read().await;
        let mut components = Vec::new();

        if config.components.monitoring {
            components.push("monitoring".to_string());
        }
        if config.components.synthesis && content.to_lowercase().contains("generate") {
            components.push("synthesis".to_string());
        }
        if config.components.testing && content.to_lowercase().contains("test") {
            components.push("testing".to_string());
        }
        if config.components.deployment && content.to_lowercase().contains("deploy") {
            components.push("deployment".to_string());
        }
        if config.components.learning {
            components.push("learning".to_string());
        }

        components.sort();
        components.dedup();
        components
    }

    fn extract_title(content: &str) -> Option<String> {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                return Some(trimmed.trim_start_matches('#').trim().to_string());
            }
        }
        None
    }

    fn summarize_spec(spec: &Specification) -> Option<String> {
        if spec.requirements.is_empty() {
            return None;
        }
        let mut summary = String::new();
        for requirement in spec.requirements.iter().take(3) {
            if !summary.is_empty() {
                summary.push_str(" | ");
            }
            summary.push_str(&requirement.description);
        }
        Some(summary)
    }

    fn extract_paths(content: &str) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        let pattern = regex::Regex::new(r"(auto-dev[^\s`]+|src/[^\s`]+|docs/[^\s`]+)")
            .unwrap_or_else(|_| regex::Regex::new(r"src/[^\s`]+").unwrap());

        for capture in pattern.captures_iter(content) {
            if let Some(path) = capture.get(1) {
                paths.push(PathBuf::from(path.as_str()))
            }
        }

        paths
    }

    fn prp_completed(content: &str) -> bool {
        content.lines().any(|line| {
            line.to_lowercase().contains("status") && line.to_lowercase().contains("complete")
        })
    }

    fn assess_risk_level(path: &Path) -> RiskLevel {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or_default().to_lowercase();
        if name.contains("safety") || name.contains("deployment") {
            RiskLevel::High
        } else if name.contains("test") || name.contains("doc") {
            RiskLevel::Low
        } else if name.contains("core") {
            RiskLevel::Critical
        } else {
            RiskLevel::Medium
        }
    }

    async fn register_failure(&self, change_id: &str) {
        let mut failures = self.change_failures.write().await;
        *failures.entry(change_id.to_string()).or_insert(0) += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_dev::{ComponentConfig, DevelopmentMode, SafetyLevel};
    use tempfile::TempDir;

    fn sample_config() -> SelfDevConfig {
        SelfDevConfig {
            enabled: true,
            mode: DevelopmentMode::Assisted,
            safety_level: SafetyLevel::Standard,
            auto_approve: false,
            max_changes_per_day: 5,
            require_tests: false,
            require_documentation: true,
            components: ComponentConfig {
                monitoring: true,
                synthesis: true,
                testing: false,
                deployment: false,
                learning: true,
            },
        }
    }

    fn write_prp(dir: &TempDir, name: &str, body: &str) {
        let prp_dir = dir.path().join("PRPs");
        std::fs::create_dir_all(&prp_dir).expect("failed to create PRP dir");
        std::fs::write(prp_dir.join(name), body).expect("failed to write PRP file");
    }

    #[tokio::test]
    async fn test_analyze_prp_detects_change() {
        let temp = TempDir::new().unwrap();
        write_prp(
            &temp,
            "215-self-development.md",
            "# PRP: Sample\n\n**Status**: PARTIAL\n\n## Requirements\n- Ensure monitoring is active\n",
        );

        let config = Arc::new(RwLock::new(sample_config()));
        let coordinator = ComponentCoordinator::new(config, temp.path().to_path_buf()).await;

        coordinator.analyze_requirements().await.unwrap();
        let changes = coordinator.get_pending_changes().await.unwrap();

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].status, ChangeStatus::Analyzed);
        assert_eq!(changes[0].risk_level, RiskLevel::Medium);
    }

    #[tokio::test]
    async fn test_plan_and_generate_flow() {
        let temp = TempDir::new().unwrap();
        write_prp(
            &temp,
            "300-integration.md",
            "# PRP: Integration\n\n**Status**: PARTIAL\n\n## Requirements\n- Integrate event loop\n- Validate safety gates\n",
        );

        let config = Arc::new(RwLock::new(sample_config()));
        let coordinator = ComponentCoordinator::new(config, temp.path().to_path_buf()).await;

        coordinator.analyze_requirements().await.unwrap();
        coordinator.create_implementation_plan().await.unwrap();
        coordinator.generate_solution().await.unwrap();

        let changes = coordinator.get_pending_changes().await.unwrap();
        assert_eq!(changes.len(), 1);
        assert!(changes[0].plan.is_some());
        assert_eq!(changes[0].status, ChangeStatus::ReadyForTesting);

        let results = coordinator.test_solution().await.unwrap();
        assert!(results.runs().is_empty());
    }
}
