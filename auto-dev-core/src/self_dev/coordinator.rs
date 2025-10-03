//! Component coordinator for integrating self-development subsystems

use super::monitor::SafetyAuthority;
use super::orchestrator::{ChangeStatus, ChangeType, PendingChange, RiskLevel, TestResults};
use super::{Result, SafetyLevel, SelfDevConfig, SelfDevError};
use crate::incremental::planner::{IncrementPlan, IncrementPlanner};
use crate::parser::SpecParser;
use crate::parser::model::{Priority, Specification};
use crate::safety::{
    CodeModification, ModificationType, SafetyConfig, SafetyGatekeeper, ValidationReport,
};
use crate::self_target::SelfTargetConfig;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Clone)]
struct ChangeRecord {
    change: PendingChange,
    spec: Specification,
    plan_path: Option<PathBuf>,
    plan_summary: Option<String>,
    generated_artifacts: Vec<PathBuf>,
    safety_report: Option<ValidationReport>,
    last_state_change: DateTime<Utc>,
}

struct DiscoveredPrp {
    id: String,
    description: String,
    path: PathBuf,
    spec: Specification,
    status: Option<String>,
}

pub struct ComponentCoordinator {
    config: Arc<RwLock<SelfDevConfig>>,
    project_root: PathBuf,
    spec_parser: SpecParser,
    planner: IncrementPlanner,
    safety_gate: Arc<RwLock<SafetyGatekeeper>>,
    pending_changes: Arc<RwLock<HashMap<String, ChangeRecord>>>,
    approved_changes: Arc<RwLock<HashSet<String>>>,
    today_changes_count: Arc<RwLock<usize>>,
}

impl ComponentCoordinator {
    pub async fn new(config: Arc<RwLock<SelfDevConfig>>) -> Result<Self> {
        let cfg = config.read().await.clone();

        let target_cfg = SelfTargetConfig::load_or_create().map_err(|e| {
            SelfDevError::Configuration(format!("Failed to load self-target config: {}", e))
        })?;
        let project_root = target_cfg.project.path.clone();

        let safety_gate =
            SafetyGatekeeper::new(Self::build_safety_config(&cfg.safety_level, &project_root))
                .map_err(|e| SelfDevError::SafetyViolation(e.to_string()))?;

        let planner = IncrementPlanner::new(8, cfg.require_tests);

        Ok(Self {
            config,
            project_root,
            spec_parser: SpecParser::new(),
            planner,
            safety_gate: Arc::new(RwLock::new(safety_gate)),
            pending_changes: Arc::new(RwLock::new(HashMap::new())),
            approved_changes: Arc::new(RwLock::new(HashSet::new())),
            today_changes_count: Arc::new(RwLock::new(0)),
        })
    }

    pub async fn has_pending_work(&self) -> bool {
        {
            let pending = self.pending_changes.read().await;
            if pending.values().any(|record| record.change.status.is_actionable()) {
                return true;
            }
        }

        match self.discover_open_prps().await {
            Ok(prps) => !prps.is_empty(),
            Err(err) => {
                warn!("Failed to discover PRPs: {}", err);
                false
            }
        }
    }

    pub async fn analyze_requirements(&self) -> Result<()> {
        info!("Analyzing open PRPs for self-development pipeline");
        let discovered = self.discover_open_prps().await?;

        let mut pending = self.pending_changes.write().await;
        for prp in discovered {
            if pending.contains_key(&prp.id) {
                continue;
            }

            let risk = Self::assess_risk_level(&prp.spec);
            let description = prp.description.clone();
            let change = PendingChange {
                id: prp.id.clone(),
                description,
                file_path: prp.path.to_string_lossy().to_string(),
                change_type: ChangeType::Create,
                risk_level: risk,
                status: ChangeStatus::PendingAnalysis,
                plan_path: None,
                summary: Self::summarize_spec(&prp.spec),
                requires_manual_review: true,
                last_updated: Utc::now(),
            };

            pending.insert(
                prp.id.clone(),
                ChangeRecord {
                    change,
                    spec: prp.spec,
                    plan_path: None,
                    plan_summary: None,
                    generated_artifacts: Vec::new(),
                    safety_report: None,
                    last_state_change: Utc::now(),
                },
            );
        }

        info!("Tracked {} pending self-development tasks", pending.len());
        Ok(())
    }

    pub async fn create_implementation_plan(&self) -> Result<()> {
        let ids: Vec<String> = {
            let pending = self.pending_changes.read().await;
            pending
                .iter()
                .filter_map(|(id, record)| match record.change.status {
                    ChangeStatus::PendingAnalysis | ChangeStatus::Planning => Some(id.clone()),
                    _ => None,
                })
                .collect()
        };

        for change_id in ids {
            if let Err(err) = self.plan_for_change(&change_id).await {
                error!("Failed to generate plan for {}: {}", change_id, err);
            }
        }

        Ok(())
    }

    pub async fn generate_solution(&self) -> Result<()> {
        let cfg = self.config.read().await.clone();
        if !cfg.components.synthesis {
            info!("Synthesis component disabled; skipping automated generation");
            return Ok(());
        }

        let ids: Vec<String> = {
            let pending = self.pending_changes.read().await;
            pending
                .iter()
                .filter_map(|(id, record)| match record.change.status {
                    ChangeStatus::AwaitingImplementation => Some(id.clone()),
                    _ => None,
                })
                .collect()
        };

        for change_id in ids {
            if let Err(err) = self.materialize_outline(&change_id).await {
                error!("Failed to synthesize outline for {}: {}", change_id, err);
            }
        }

        Ok(())
    }

    pub async fn test_solution(&self) -> Result<TestResults> {
        let cfg = self.config.read().await.clone();
        if !cfg.components.testing {
            warn!("Testing component disabled, skipping validation tests");
            return Ok(TestResults::new(1, 0, 0, Some("Testing disabled by configuration".into())));
        }

        info!("Running workspace cargo check as self-development sanity test");
        let mut command = Command::new("cargo");
        command.arg("check").arg("--workspace").current_dir(&self.project_root);

        match command.output().await {
            Ok(output) => {
                if output.status.success() {
                    Ok(TestResults::new(1, 0, 0, Some("cargo check --workspace".into())))
                } else {
                    warn!("cargo check failed during self-development cycle");
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    Ok(TestResults::new(0, 1, 0, Some(stderr)))
                }
            }
            Err(err) => {
                Err(SelfDevError::Coordination(format!("Failed to run cargo check: {}", err)))
            }
        }
    }

    pub async fn deploy_approved_changes(&self) -> Result<()> {
        let cfg = self.config.read().await.clone();
        if !cfg.components.deployment {
            info!("Deployment component disabled; leaving approved changes for manual application");
            return Ok(());
        }

        let approved_ids: Vec<String> = {
            let mut approved = self.approved_changes.write().await;
            let ids: Vec<_> = approved.iter().cloned().collect();
            approved.clear();
            ids
        };

        if approved_ids.is_empty() {
            return Ok(());
        }

        let mut pending = self.pending_changes.write().await;
        let mut count = self.today_changes_count.write().await;

        for id in approved_ids {
            if let Some(record) = pending.get_mut(&id) {
                if *count >= cfg.max_changes_per_day {
                    warn!("Daily change limit reached; deferring deployment for {}", id);
                    break;
                }

                record.change.status = ChangeStatus::Completed;
                record.change.last_updated = Utc::now();
                record.last_state_change = record.change.last_updated;
                *count += 1;
                info!("Marked {} as deployed", id);
            }
        }

        Ok(())
    }

    pub async fn monitor_deployment(&self) -> Result<()> {
        info!("Monitoring deployment effects (placeholder implementation)");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        Ok(())
    }

    pub async fn extract_learning_patterns(&self) -> Result<()> {
        let cfg = self.config.read().await.clone();
        if !cfg.components.learning {
            debug!("Learning component disabled");
            return Ok(());
        }

        info!("Capturing self-development learnings");
        Ok(())
    }

    pub async fn rollback_all(&self) -> Result<()> {
        error!("Rolling back generated plans and artifacts");

        let mut pending = self.pending_changes.write().await;
        let mut approved = self.approved_changes.write().await;

        for record in pending.values_mut() {
            if let Some(path) = record.plan_path.take() {
                Self::remove_file_if_exists(&path).await;
            }
            for artifact in record.generated_artifacts.drain(..) {
                Self::remove_file_if_exists(&artifact).await;
            }
            record.change.status = ChangeStatus::PendingAnalysis;
            record.change.plan_path = None;
            record.change.last_updated = Utc::now();
            record.last_state_change = record.change.last_updated;
        }

        approved.clear();
        *self.today_changes_count.write().await = 0;

        Ok(())
    }

    pub async fn get_active_components(&self) -> Vec<String> {
        let cfg = self.config.read().await.clone();
        Self::active_components_for(&cfg)
    }

    pub async fn get_pending_changes(&self) -> Result<Vec<PendingChange>> {
        let mut changes: Vec<PendingChange> = self
            .pending_changes
            .read()
            .await
            .values()
            .map(|record| record.change.clone())
            .collect();
        changes.sort_by_key(|c| c.last_updated);
        Ok(changes)
    }

    pub async fn get_today_changes_count(&self) -> usize {
        *self.today_changes_count.read().await
    }

    pub async fn approve_change(&self, change_id: String) -> Result<()> {
        let mut pending = self.pending_changes.write().await;
        if let Some(record) = pending.get_mut(&change_id) {
            record.change.status = ChangeStatus::AwaitingDeployment;
            record.change.last_updated = Utc::now();
            record.last_state_change = record.change.last_updated;
            self.approved_changes.write().await.insert(change_id);
            info!("Approved change for deployment");
            Ok(())
        } else {
            Err(SelfDevError::Coordination(format!("Change {} not found", change_id)))
        }
    }

    pub async fn reject_change(&self, change_id: String) -> Result<()> {
        let mut pending = self.pending_changes.write().await;
        if let Some(record) = pending.get_mut(&change_id) {
            record.change.status = ChangeStatus::Rejected;
            record.change.last_updated = Utc::now();
            record.last_state_change = record.change.last_updated;
            info!("Rejected change {}", change_id);
            Ok(())
        } else {
            Err(SelfDevError::Coordination(format!("Change {} not found", change_id)))
        }
    }

    pub async fn validate_change(&self, change_id: &str) -> Result<ValidationReport> {
        let record = {
            let pending = self.pending_changes.read().await;
            pending.get(change_id).cloned().ok_or_else(|| {
                SelfDevError::Coordination(format!("Change {} not found", change_id))
            })?
        };

        if let Some(report) = record.safety_report.clone() {
            return Ok(report);
        }

        let target_path = record
            .plan_path
            .clone()
            .or_else(|| record.generated_artifacts.last().cloned())
            .ok_or_else(|| {
                SelfDevError::Coordination(format!("No artifacts available for {}", change_id))
            })?;

        let modified = fs::read_to_string(&target_path).await.unwrap_or_else(|_| String::new());
        let modification = CodeModification {
            file_path: target_path.clone(),
            original: String::new(),
            modified,
            modification_type: if target_path.exists() {
                ModificationType::Update
            } else {
                ModificationType::Create
            },
            reason: format!("Safety validation for {}", change_id),
            prp_reference: Some(change_id.to_string()),
        };

        let report = self
            .safety_gate
            .read()
            .await
            .validate(&modification)
            .await
            .map_err(|e| SelfDevError::SafetyViolation(e.to_string()))?;

        let mut pending = self.pending_changes.write().await;
        if let Some(inner) = pending.get_mut(change_id) {
            inner.safety_report = Some(report.clone());
        }

        Ok(report)
    }

    pub async fn update_safety_level(&self, level: SafetyLevel) -> Result<()> {
        let gate = SafetyGatekeeper::new(Self::build_safety_config(&level, &self.project_root))
            .map_err(|e| SelfDevError::SafetyViolation(e.to_string()))?;
        {
            let mut cfg = self.config.write().await;
            cfg.safety_level = level.clone();
        }
        let mut guard = self.safety_gate.write().await;
        *guard = gate;
        Ok(())
    }

    pub async fn current_safety_level(&self) -> SafetyLevel {
        self.config.read().await.safety_level.clone()
    }

    async fn plan_for_change(&self, change_id: &str) -> Result<()> {
        let record = {
            let pending = self.pending_changes.read().await;
            pending.get(change_id).cloned().ok_or_else(|| {
                SelfDevError::Coordination(format!("Change {} not found", change_id))
            })?
        };

        let plan = self
            .planner
            .plan_increments(&record.spec)
            .map_err(|e| SelfDevError::Coordination(format!("Planner failure: {}", e)))?;

        let plan_text = Self::render_plan(change_id, &plan, &record.spec);
        let plan_path = self.plan_directory().join(format!("{}.md", change_id));
        if let Some(parent) = plan_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                SelfDevError::Coordination(format!("Failed to create plan dir: {}", e))
            })?;
        }

        fs::write(&plan_path, plan_text)
            .await
            .map_err(|e| SelfDevError::Coordination(format!("Failed to write plan: {}", e)))?;

        let mut pending = self.pending_changes.write().await;
        if let Some(inner) = pending.get_mut(change_id) {
            inner.plan_path = Some(plan_path.clone());
            inner.plan_summary = Some(Self::summarize_plan(&plan));
            inner.change.plan_path = self.relative_to_project(&plan_path);
            inner.change.status = ChangeStatus::AwaitingImplementation;
            inner.change.last_updated = Utc::now();
            inner.last_state_change = inner.change.last_updated;
            inner.safety_report = None;
        }

        info!("Generated implementation plan for {}", change_id);
        Ok(())
    }

    async fn materialize_outline(&self, change_id: &str) -> Result<()> {
        let record = {
            let pending = self.pending_changes.read().await;
            pending.get(change_id).cloned().ok_or_else(|| {
                SelfDevError::Coordination(format!("Change {} not found", change_id))
            })?
        };

        let outcome_path = self.outcome_directory().join(format!("{}-outline.md", change_id));
        if let Some(parent) = outcome_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                SelfDevError::Coordination(format!("Failed to create outcome dir: {}", e))
            })?;
        }

        let outline = Self::render_outline(change_id, &record.spec, record.plan_summary.clone());
        fs::write(&outcome_path, outline)
            .await
            .map_err(|e| SelfDevError::Coordination(format!("Failed to write outline: {}", e)))?;

        let mut pending = self.pending_changes.write().await;
        if let Some(inner) = pending.get_mut(change_id) {
            inner.generated_artifacts.push(outcome_path.clone());
            inner.change.status = ChangeStatus::AwaitingReview;
            inner.change.last_updated = Utc::now();
            inner.last_state_change = inner.change.last_updated;
            inner.change.requires_manual_review = true;
        }

        info!("Generated implementation outline for {}", change_id);
        Ok(())
    }

    async fn discover_open_prps(&self) -> Result<Vec<DiscoveredPrp>> {
        let prp_dir = self.project_root.join("PRPs");
        let mut discovered = Vec::new();

        if !prp_dir.exists() {
            return Ok(discovered);
        }

        let mut entries = fs::read_dir(&prp_dir).await.map_err(|e| {
            SelfDevError::Coordination(format!("Failed to read PRPs directory: {}", e))
        })?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| SelfDevError::Coordination(format!("Failed to read PRP entry: {}", e)))?
        {
            let path = entry.path();

            if path.is_dir() {
                continue;
            }

            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }

            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem.eq_ignore_ascii_case("readme") || stem.starts_with('_') {
                    continue;
                }
            }

            let content = fs::read_to_string(&path).await.map_err(|e| {
                SelfDevError::Coordination(format!("Failed to read {}: {}", path.display(), e))
            })?;

            if let Some(status) = Self::extract_status(&content) {
                if status.to_uppercase().contains("COMPLETE") {
                    continue;
                }
            }

            let spec = self.spec_parser.parse_file(&path).await.map_err(|e| {
                SelfDevError::Coordination(format!("Failed to parse {}: {}", path.display(), e))
            })?;

            let id = Self::derive_change_id(&path);
            let description = format!("Implement {}", path.file_name().unwrap().to_string_lossy());

            discovered.push(DiscoveredPrp {
                id,
                description,
                path,
                spec,
                status: Self::extract_status(&content),
            });
        }

        Ok(discovered)
    }

    fn derive_change_id(path: &Path) -> String {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("prp");
        let number = stem.split(|c: char| !c.is_ascii_digit()).find(|part| !part.is_empty());
        match number {
            Some(num) => format!("prp_{}", num),
            None => format!("prp_{}", stem.replace('-', "_")),
        }
    }

    fn extract_status(content: &str) -> Option<String> {
        content.lines().take(20).find_map(|line| {
            if line.to_ascii_lowercase().starts_with("status") {
                line.split(':').nth(1).map(|s| s.trim().to_string())
            } else {
                None
            }
        })
    }

    fn build_safety_config(level: &SafetyLevel, project_root: &Path) -> SafetyConfig {
        let mut config = SafetyConfig::default();
        config.allowed_paths = vec![
            project_root.join("src"),
            project_root.join("docs"),
            project_root.join("tests"),
            project_root.join("auto-dev-core"),
        ];
        config.critical_files.push(project_root.join("Cargo.lock"));

        match level {
            SafetyLevel::Strict => {
                config.max_validation_time = 15;
                config.require_all_gates = true;
            }
            SafetyLevel::Standard => {
                config.max_validation_time = 10;
                config.require_all_gates = false;
            }
            SafetyLevel::Permissive => {
                config.max_validation_time = 5;
                config.require_all_gates = false;
                config.require_reversibility = false;
            }
        }

        config
    }

    fn assess_risk_level(spec: &Specification) -> RiskLevel {
        let max_priority =
            spec.requirements.iter().map(|req| req.priority).max().unwrap_or(Priority::Medium);

        match max_priority {
            Priority::Critical => RiskLevel::Critical,
            Priority::High => RiskLevel::High,
            Priority::Medium => RiskLevel::Medium,
            Priority::Low => RiskLevel::Low,
        }
    }

    fn summarize_spec(spec: &Specification) -> Option<String> {
        spec.requirements
            .first()
            .map(|req| req.description.clone())
            .or_else(|| spec.examples.first().map(|ex| ex.description.clone()))
    }

    fn summarize_plan(plan: &IncrementPlan) -> String {
        if plan.increments.is_empty() {
            return "No increments generated".into();
        }

        let mut summary = String::new();
        for (idx, increment) in plan.increments.iter().take(3).enumerate() {
            summary.push_str(&format!("{}. {}\n", idx + 1, increment.specification.description));
        }
        if plan.increments.len() > 3 {
            summary.push_str("...\n");
        }
        summary
    }

    fn render_plan(change_id: &str, plan: &IncrementPlan, spec: &Specification) -> String {
        let mut content = String::new();
        content.push_str(&format!("# Implementation Plan for {}\n\n", change_id));
        content.push_str(&format!("Generated: {}\n\n", Utc::now().to_rfc3339()));

        if !spec.requirements.is_empty() {
            content.push_str("## Key Requirements\n");
            for req in &spec.requirements {
                content.push_str(&format!("- [{}] {}\n", req.priority, req.description));
            }
            content.push('\n');
        }

        if !plan.increments.is_empty() {
            content.push_str("## Planned Increments\n");
            for (idx, increment) in plan.increments.iter().enumerate() {
                content.push_str(&format!(
                    "{}. {} (complexity: {:?})\n",
                    idx + 1,
                    increment.specification.description,
                    increment.implementation.estimated_complexity
                ));
                if !increment.tests.is_empty() {
                    content.push_str("   - Tests: ");
                    for test in &increment.tests {
                        content.push_str(&format!("`{}` ", test.name));
                    }
                    content.push('\n');
                }
            }
            content.push('\n');
        }

        content.push_str("## Execution Notes\n");
        content.push_str(&format!(
            "- Estimated duration: {:?}\n- Critical path length: {} increments\n",
            plan.estimated_duration,
            plan.critical_path.len()
        ));

        content
    }

    fn render_outline(
        change_id: &str,
        spec: &Specification,
        plan_summary: Option<String>,
    ) -> String {
        let mut content = String::new();
        content.push_str(&format!("# Implementation Outline for {}\n\n", change_id));
        content.push_str("## Summary\n");
        if let Some(summary) = plan_summary {
            content.push_str(&summary);
            content.push('\n');
        }

        content.push_str("## Next Actions\n");
        for requirement in &spec.requirements {
            content.push_str(&format!(
                "- [ ] {} (priority: {})\n",
                requirement.description, requirement.priority
            ));
        }

        if spec.requirements.is_empty() {
            content.push_str("- [ ] Review PRP requirements and populate tasks\n");
        }

        content.push('\n');
        content.push_str("## Safety Checklist\n");
        content.push_str("- [ ] Confirm generated plan stays within docs/self_dev\n");
        content.push_str("- [ ] Ensure manual review before deployment\n");
        content
    }

    fn plan_directory(&self) -> PathBuf {
        self.project_root.join("docs").join("self_dev").join("plans")
    }

    fn outcome_directory(&self) -> PathBuf {
        self.project_root.join("docs").join("self_dev").join("outcomes")
    }

    fn relative_to_project(&self, path: &Path) -> Option<String> {
        path.strip_prefix(&self.project_root).map(|rel| rel.to_string_lossy().to_string()).ok()
    }

    async fn remove_file_if_exists(path: &Path) {
        if fs::metadata(path).await.is_ok() {
            if let Err(err) = fs::remove_file(path).await {
                warn!("Failed to remove {}: {}", path.display(), err);
            }
        }
    }

    fn active_components_for(config: &SelfDevConfig) -> Vec<String> {
        let mut components = Vec::new();
        if config.components.monitoring {
            components.push("monitoring".into());
        }
        if config.components.synthesis {
            components.push("synthesis".into());
        }
        if config.components.testing {
            components.push("testing".into());
        }
        if config.components.deployment {
            components.push("deployment".into());
        }
        if config.components.learning {
            components.push("learning".into());
        }
        components
    }
}

#[async_trait]
impl SafetyAuthority for ComponentCoordinator {
    async fn validate_change(&self, change_id: &str) -> Result<ValidationReport> {
        ComponentCoordinator::validate_change(self, change_id).await
    }

    async fn update_safety_level(&self, level: SafetyLevel) -> Result<()> {
        ComponentCoordinator::update_safety_level(self, level).await
    }

    async fn current_safety_level(&self) -> Result<SafetyLevel> {
        Ok(self.current_safety_level().await)
    }
}
