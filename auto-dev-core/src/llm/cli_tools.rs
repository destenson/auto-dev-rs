//! CLI tool providers for Claude CLI, Fabric, and other command-line LLM tools

use super::{provider::*, ClassificationResult};
use anyhow::{Result, Context};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, warn};

/// Provider that uses the Claude CLI tool if installed
pub struct ClaudeCLIProvider {
    available: bool,
}

impl ClaudeCLIProvider {
    pub async fn new() -> Self {
        // Check if claude CLI is available - try .cmd version on Windows
        let claude_cmd = if cfg!(windows) { "claude.cmd" } else { "claude" };
        
        let available = Command::new(claude_cmd)
            .arg("--version")
            .output()
            .await
            .map(|output| {
                output.status.success() || 
                !output.stdout.is_empty() || 
                String::from_utf8_lossy(&output.stderr).contains("Claude")
            })
            .unwrap_or(false);
        
        if available {
            debug!("Claude CLI tool found and available");
        } else {
            debug!("Claude CLI tool not found");
        }
        
        Self { available }
    }
    
    async fn run_claude(&self, prompt: &str) -> Result<String> {
        let claude_cmd = if cfg!(windows) { "claude.cmd" } else { "claude" };
        
        let output = Command::new(claude_cmd)
            .arg("--print")
            .arg(prompt)
            .output()
            .await
            .context("Failed to run claude CLI")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Claude CLI failed: {}", stderr));
        }
        
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[async_trait]
impl LLMProvider for ClaudeCLIProvider {
    fn name(&self) -> &str {
        "claude-cli"
    }
    
    fn tier(&self) -> ModelTier {
        ModelTier::Large // Claude CLI uses the full Claude model
    }
    
    async fn is_available(&self) -> bool {
        self.available
    }
    
    fn cost_per_1k_tokens(&self) -> f32 {
        0.0 // CLI usage is typically free after subscription
    }
    
    async fn generate_code(
        &self,
        spec: &Specification,
        context: &ProjectContext,
        _options: &GenerationOptions,
    ) -> Result<GeneratedCode> {
        let prompt = format!(
            "Generate {} code for this specification:

{}

Requirements:
{}",
            context.language,
            spec.content,
            spec.requirements.join("
")
        );
        
        let response = self.run_claude(&prompt).await?;
        
        Ok(GeneratedCode {
            files: vec![GeneratedFile {
                path: "generated.txt".to_string(),
                content: response.clone(),
                language: context.language.clone(),
                is_test: false,
            }],
            explanation: "Generated via Claude CLI".to_string(),
            confidence: 0.85,
            tokens_used: 0,
            model_used: "claude-cli".to_string(),
            cached: false,
        })
    }
    
    async fn explain_implementation(
        &self,
        code: &str,
        spec: &Specification,
    ) -> Result<Explanation> {
        let prompt = format!(
            "Explain how this code implements the specification:

Code:
{}

Spec:
{}",
            code, spec.content
        );
        
        let response = self.run_claude(&prompt).await?;
        
        Ok(Explanation {
            summary: response,
            details: vec![],
            design_decisions: vec![],
            trade_offs: vec![],
        })
    }
    
    async fn review_code(
        &self,
        code: &str,
        requirements: &[Requirement],
    ) -> Result<ReviewResult> {
        let req_text = requirements.iter()
            .map(|r| format!("{}: {}", r.id, r.description))
            .collect::<Vec<_>>()
            .join("
");
        
        let prompt = format!(
            "Review this code against these requirements:

Code:
{}

Requirements:
{}",
            code, req_text
        );
        
        let response = self.run_claude(&prompt).await?;
        
        Ok(ReviewResult {
            issues: vec![],
            suggestions: vec![response],
            meets_requirements: true,
            confidence: 0.8,
        })
    }
    
    async fn answer_question(&self, question: &str) -> Result<Option<String>> {
        let response = self.run_claude(question).await?;
        Ok(Some(response))
    }
    
    async fn classify_content(&self, _content: &str) -> Result<ClassificationResult> {
        // Claude CLI is overkill for classification
        Err(anyhow::anyhow!("Claude CLI not suitable for simple classification"))
    }
    
    async fn assess_complexity(&self, task: &str) -> Result<TaskComplexity> {
        let prompt = format!("Assess if this task is simple, moderate, complex, or very complex: {}", task);
        let response = self.run_claude(&prompt).await?;
        
        let tier = if response.contains("simple") {
            ModelTier::Small
        } else if response.contains("very complex") {
            ModelTier::Large
        } else if response.contains("complex") {
            ModelTier::Medium
        } else {
            ModelTier::Small
        };
        
        Ok(TaskComplexity {
            tier,
            reasoning: response,
            estimated_tokens: 0,
            confidence: 0.7,
        })
    }
}

/// Provider that uses the Fabric CLI tool if installed
pub struct FabricProvider {
    available: bool,
    patterns: Vec<String>,
    models: Vec<String>,
    current_model: Option<String>,
}

impl FabricProvider {
    /// Get list of available models
    pub fn get_models(&self) -> &[String] {
        &self.models
    }
    
    /// Get current selected model
    pub fn get_current_model(&self) -> Option<&String> {
        self.current_model.as_ref()
    }
    
    pub async fn new() -> Self {
        // Check if fabric is available
        let available = Command::new("fabric")
            .arg("--help")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false);
        
        let mut patterns = Vec::new();
        let mut models = Vec::new();
        
        if available {
            debug!("Fabric CLI tool found, discovering patterns and models");
            
            // Try to list available patterns
            if let Ok(output) = Command::new("fabric")
                .arg("-l")
                .stdin(Stdio::null())  // Don't wait for input
                .output()
                .await
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                patterns = stdout.lines()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                
                debug!("Found {} Fabric patterns", patterns.len());
            }
            
            // Try to list available models
            if let Ok(output) = Command::new("fabric")
                .arg("-L")
                .stdin(Stdio::null())  // Don't wait for input
                .output()
                .await
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Parse model names from the output
                // Format is: \t[number]\tmodel-name
                models = stdout.lines()
                    .filter(|line| line.starts_with("\t["))
                    .filter_map(|line| {
                        // Split by tab and get the third part (after \t[number]\t)
                        line.split('\t')
                            .nth(2)
                            .map(|s| s.trim().to_string())
                    })
                    .collect();
                
                debug!("Found {} Fabric models", models.len());
            }
        } else {
            debug!("Fabric CLI tool not found");
        }
        
        // Default to a good model if available
        let current_model = if models.contains(&"claude-3-5-sonnet-latest".to_string()) {
            Some("claude-3-5-sonnet-latest".to_string())
        } else if !models.is_empty() {
            Some(models[0].clone())
        } else {
            None
        };
        
        Self { available, patterns, models, current_model }
    }
    
    /// Run fabric with a specific pattern
    async fn run_fabric(&self, input: &str, pattern: &str) -> Result<String> {
        let mut cmd = Command::new("fabric");
        cmd.arg("--pattern").arg(pattern);
        
        // Add model if specified
        if let Some(model) = &self.current_model {
            cmd.arg("--model").arg(model);
        }
        
        cmd.arg("--text").arg(input);
        
        let output = cmd
            .output()
            .await
            .context("Failed to run fabric CLI")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Fabric CLI failed: {}", stderr));
        }
        
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
    
    /// Select best model based on task complexity
    pub fn select_model_for_tier(&mut self, tier: ModelTier) {
        let model = match tier {
            ModelTier::NoLLM => None, // No model needed for heuristics
            ModelTier::Tiny => self.find_best_model_for_tier(tier),
            ModelTier::Small => self.find_best_model_for_tier(tier),
            ModelTier::Medium => self.find_best_model_for_tier(tier),
            ModelTier::Large => self.find_best_model_for_tier(tier),
        };
        
        if let Some(m) = model {
            debug!("Selected model {} for tier {:?}", m, tier);
            self.current_model = Some(m);
        }
    }
    
    /// Find best model for a given tier based on actual available models
    fn find_best_model_for_tier(&self, tier: ModelTier) -> Option<String> {
        // Categorize models by analyzing their names
        let mut categorized: Vec<(String, ModelTier)> = Vec::new();
        
        for model in &self.models {
            let model_lower = model.to_lowercase();
            let detected_tier = if model_lower.contains("opus") || 
                                   model_lower.contains("deepseek-v3") ||
                                   model_lower.contains("deepseek-r1") ||
                                   model_lower.contains("405b") ||
                                   model_lower.contains("120b") {
                ModelTier::Large
            } else if model_lower.contains("70b") || 
                      model_lower.contains("sonnet") ||
                      model_lower.contains("32b") {
                ModelTier::Medium
            } else if model_lower.contains("haiku") || 
                      model_lower.contains("8b") ||
                      model_lower.contains("9b") ||
                      model_lower.contains("7b") ||
                      model_lower.contains("gemma") {
                ModelTier::Small
            } else if model_lower.contains("qwen") ||
                      model_lower.contains("0.5b") ||
                      model_lower.contains("whisper") {
                ModelTier::Tiny
            } else {
                // Default based on common patterns
                ModelTier::Medium
            };
            
            categorized.push((model.clone(), detected_tier));
        }
        
        // Find models matching the requested tier
        let matching: Vec<String> = categorized
            .iter()
            .filter(|(_, t)| *t == tier)
            .map(|(m, _)| m.clone())
            .collect();
        
        // Prefer certain models if available
        let preferences = match tier {
            ModelTier::Large => vec!["claude-3-5-sonnet-latest", "claude-3-opus", "deepseek-v3"],
            ModelTier::Medium => vec!["claude-3-5-sonnet", "llama-3.3-70b", "qwen3-32b"],
            ModelTier::Small => vec!["claude-3-5-haiku", "llama-3.1-8b", "hermes3-8b"],
            ModelTier::Tiny => vec!["qwen", "whisper"],
            ModelTier::NoLLM => return None,
        };
        
        // Try to find preferred model first
        for pref in preferences {
            if let Some(model) = matching.iter().find(|m| m.contains(pref)) {
                return Some(model.clone());
            }
        }
        
        // Return first matching model if no preference found
        matching.first().cloned()
    }
    
    /// Select the best pattern for a task
    fn select_pattern(&self, task_type: &str) -> &str {
        // Map task types to fabric patterns
        let pattern = match task_type {
            "code" | "generate" => "write_code",
            "explain" => "explain_code",
            "review" => "review_code",
            "improve" => "improve_code",
            "test" => "create_test",
            "document" => "write_docs",
            "summarize" => "summarize",
            "analyze" => "analyze_code",
            _ => "ask", // Default pattern
        };
        
        // Check if pattern exists
        if self.patterns.contains(&pattern.to_string()) {
            pattern
        } else {
            "ask" // Fallback to generic ask pattern
        }
    }
}

#[async_trait]
impl LLMProvider for FabricProvider {
    fn name(&self) -> &str {
        "fabric"
    }
    
    fn tier(&self) -> ModelTier {
        // Determine tier based on current model
        if let Some(model) = &self.current_model {
            if model.contains("opus") || model.contains("deepseek-v3") {
                ModelTier::Large
            } else if model.contains("claude-3-5-sonnet") || model.contains("70b") {
                ModelTier::Medium
            } else if model.contains("haiku") || model.contains("8b") {
                ModelTier::Small
            } else {
                ModelTier::Medium // Default
            }
        } else {
            ModelTier::Medium
        }
    }
    
    async fn is_available(&self) -> bool {
        self.available
    }
    
    fn cost_per_1k_tokens(&self) -> f32 {
        0.001 // Depends on underlying model
    }
    
    async fn generate_code(
        &self,
        spec: &Specification,
        context: &ProjectContext,
        _options: &GenerationOptions,
    ) -> Result<GeneratedCode> {
        let input = format!(
            "Language: {}
Specification:
{}
Requirements:
{}",
            context.language,
            spec.content,
            spec.requirements.join("
")
        );
        
        let pattern = self.select_pattern("code");
        let response = self.run_fabric(&input, pattern).await?;
        
        Ok(GeneratedCode {
            files: vec![GeneratedFile {
                path: "generated.txt".to_string(),
                content: response.clone(),
                language: context.language.clone(),
                is_test: false,
            }],
            explanation: format!("Generated via Fabric pattern: {}", pattern),
            confidence: 0.75,
            tokens_used: 0,
            model_used: format!("fabric:{}", pattern),
            cached: false,
        })
    }
    
    async fn explain_implementation(
        &self,
        code: &str,
        spec: &Specification,
    ) -> Result<Explanation> {
        let input = format!("Code:
{}

Specification:
{}", code, spec.content);
        let pattern = self.select_pattern("explain");
        let response = self.run_fabric(&input, pattern).await?;
        
        Ok(Explanation {
            summary: response,
            details: vec![],
            design_decisions: vec![],
            trade_offs: vec![],
        })
    }
    
    async fn review_code(
        &self,
        code: &str,
        requirements: &[Requirement],
    ) -> Result<ReviewResult> {
        let req_text = requirements.iter()
            .map(|r| format!("{}: {}", r.id, r.description))
            .collect::<Vec<_>>()
            .join("
");
        
        let input = format!("Code:
{}

Requirements:
{}", code, req_text);
        let pattern = self.select_pattern("review");
        let response = self.run_fabric(&input, pattern).await?;
        
        Ok(ReviewResult {
            issues: vec![],
            suggestions: vec![response],
            meets_requirements: true,
            confidence: 0.7,
        })
    }
    
    async fn answer_question(&self, question: &str) -> Result<Option<String>> {
        let pattern = self.select_pattern("ask");
        let response = self.run_fabric(question, pattern).await?;
        Ok(Some(response))
    }
    
    async fn classify_content(&self, content: &str) -> Result<ClassificationResult> {
        // Fabric might have an analyze pattern
        let pattern = self.select_pattern("analyze");
        let response = self.run_fabric(content, pattern).await?;
        
        // Simple heuristic parsing
        let lower = response.to_lowercase();
        Ok(ClassificationResult {
            is_code: lower.contains("code") || lower.contains("function") || lower.contains("class"),
            is_documentation: lower.contains("documentation") || lower.contains("readme"),
            is_test: lower.contains("test") || lower.contains("spec"),
            is_config: lower.contains("config") || lower.contains("settings"),
            language: None,
            confidence: 0.6,
        })
    }
    
    async fn assess_complexity(&self, task: &str) -> Result<TaskComplexity> {
        let pattern = self.select_pattern("analyze");
        let response = self.run_fabric(task, pattern).await?;
        
        let tier = if response.contains("simple") || response.contains("trivial") {
            ModelTier::Small
        } else if response.contains("complex") || response.contains("difficult") {
            ModelTier::Large
        } else {
            ModelTier::Medium
        };
        
        Ok(TaskComplexity {
            tier,
            reasoning: response,
            estimated_tokens: 0,
            confidence: 0.6,
        })
    }
}

/// Generic CLI tool provider for custom LLM CLIs
pub struct GenericCLIProvider {
    command: String,
    args_template: Vec<String>,
    available: bool,
    name: String,
}

impl GenericCLIProvider {
    pub async fn new(command: String, args_template: Vec<String>, name: String) -> Self {
        // Check if command is available
        let available = Command::new(&command)
            .arg("--help")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false);
        
        if available {
            debug!("{} CLI tool found", name);
        } else {
            warn!("{} CLI tool not found", name);
        }
        
        Self {
            command,
            args_template,
            available,
            name,
        }
    }
    
    async fn run_command(&self, input: &str) -> Result<String> {
        let mut cmd = Command::new(&self.command);
        
        // Apply args template, replacing {input} with actual input
        for arg in &self.args_template {
            if arg == "{input}" {
                cmd.arg(input);
            } else {
                cmd.arg(arg);
            }
        }
        
        let output = cmd.output().await
            .context(format!("Failed to run {} CLI", self.name))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("{} CLI failed: {}", self.name, stderr));
        }
        
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[async_trait]
impl LLMProvider for GenericCLIProvider {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn tier(&self) -> ModelTier {
        ModelTier::Medium // Default assumption
    }
    
    async fn is_available(&self) -> bool {
        self.available
    }
    
    fn cost_per_1k_tokens(&self) -> f32 {
        0.0 // CLI tools are typically free to run
    }
    
    async fn generate_code(
        &self,
        spec: &Specification,
        context: &ProjectContext,
        _options: &GenerationOptions,
    ) -> Result<GeneratedCode> {
        let input = format!(
            "Generate {} code: {}",
            context.language,
            spec.content
        );
        
        let response = self.run_command(&input).await?;
        
        Ok(GeneratedCode {
            files: vec![GeneratedFile {
                path: "generated.txt".to_string(),
                content: response.clone(),
                language: context.language.clone(),
                is_test: false,
            }],
            explanation: format!("Generated via {}", self.name),
            confidence: 0.7,
            tokens_used: 0,
            model_used: self.name.clone(),
            cached: false,
        })
    }
    
    async fn explain_implementation(
        &self,
        code: &str,
        spec: &Specification,
    ) -> Result<Explanation> {
        let input = format!("Explain: {} for spec: {}", code, spec.content);
        let response = self.run_command(&input).await?;
        
        Ok(Explanation {
            summary: response,
            details: vec![],
            design_decisions: vec![],
            trade_offs: vec![],
        })
    }
    
    async fn review_code(
        &self,
        code: &str,
        requirements: &[Requirement],
    ) -> Result<ReviewResult> {
        let req_text = requirements.iter()
            .map(|r| r.description.clone())
            .collect::<Vec<_>>()
            .join(", ");
        
        let input = format!("Review code: {} against: {}", code, req_text);
        let response = self.run_command(&input).await?;
        
        Ok(ReviewResult {
            issues: vec![],
            suggestions: vec![response],
            meets_requirements: true,
            confidence: 0.6,
        })
    }
    
    async fn answer_question(&self, question: &str) -> Result<Option<String>> {
        let response = self.run_command(question).await?;
        Ok(Some(response))
    }
    
    async fn classify_content(&self, _content: &str) -> Result<ClassificationResult> {
        Err(anyhow::anyhow!("{} not suitable for classification", self.name))
    }
    
    async fn assess_complexity(&self, task: &str) -> Result<TaskComplexity> {
        let response = self.run_command(task).await?;
        
        Ok(TaskComplexity {
            tier: ModelTier::Medium,
            reasoning: response,
            estimated_tokens: 0,
            confidence: 0.5,
        })
    }
}
