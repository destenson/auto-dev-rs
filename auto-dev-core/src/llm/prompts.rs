//! Prompt templates optimized for different models and tasks
//!
//! Provides specialized prompts for Qwen and other models based on their strengths

use super::provider::{ModelTier, Specification, ProjectContext};
use serde::{Deserialize, Serialize};

/// Prompt templates optimized for specific tasks and models
pub struct PromptTemplates;

impl PromptTemplates {
    /// Get optimized prompt for classification task
    pub fn classification_prompt(content: &str, tier: ModelTier) -> String {
        match tier {
            ModelTier::Tiny => {
                // Qwen-optimized: Very concise, direct
                format!(
                    "Classify this:\n{}\n\nAnswer: code/doc/test/config/other",
                    &content[..content.len().min(200)]
                )
            }
            ModelTier::Small | ModelTier::Medium | ModelTier::Large => {
                format!(
                    "Classify this content into one of these categories: \
                     code, documentation, test, configuration, or other.\n\n\
                     Content:\n{}\n\n\
                     Classification:",
                    &content[..content.len().min(500)]
                )
            }
            ModelTier::NoLLM => String::new(),
        }
    }
    
    /// Get optimized prompt for yes/no questions
    pub fn yes_no_prompt(question: &str, context: Option<&str>, tier: ModelTier) -> String {
        match tier {
            ModelTier::Tiny => {
                // Qwen-optimized: Ultra-concise
                if let Some(ctx) = context {
                    format!("{}\nContext: {}\nAnswer yes/no:", question, &ctx[..ctx.len().min(100)])
                } else {
                    format!("{}\nAnswer yes/no:", question)
                }
            }
            _ => {
                if let Some(ctx) = context {
                    format!("Question: {}\n\nContext:\n{}\n\nAnswer:", question, ctx)
                } else {
                    format!("Question: {}\n\nAnswer:", question)
                }
            }
        }
    }
    
    /// Get optimized prompt for pattern detection
    pub fn pattern_detection_prompt(code: &str, pattern: &str, tier: ModelTier) -> String {
        match tier {
            ModelTier::Tiny => {
                // Qwen-optimized: Focus on simple pattern matching
                format!(
                    "Does this code contain {}?\n{}\nyes/no:",
                    pattern,
                    &code[..code.len().min(300)]
                )
            }
            _ => {
                format!(
                    "Analyze this code for the presence of {}.\n\n\
                     Code:\n{}\n\n\
                     Does it contain this pattern? Explain briefly.",
                    pattern, code
                )
            }
        }
    }
    
    /// Get optimized prompt for requirement checking
    pub fn requirement_check_prompt(requirement: &str, code: &str, tier: ModelTier) -> String {
        match tier {
            ModelTier::Tiny => {
                // Qwen-optimized: Direct requirement matching
                format!(
                    "Requirement: {}\nCode: {}\nSatisfied? yes/no:",
                    requirement,
                    &code[..code.len().min(300)]
                )
            }
            _ => {
                format!(
                    "Check if this code satisfies the requirement:\n\n\
                     Requirement: {}\n\n\
                     Code:\n{}\n\n\
                     Analysis:",
                    requirement, code
                )
            }
        }
    }
    
    /// Get optimized prompt for simple completions
    pub fn completion_prompt(prefix: &str, suffix: Option<&str>, tier: ModelTier) -> String {
        match tier {
            ModelTier::Tiny => {
                // Qwen-optimized: Minimal context for line completion
                if let Some(s) = suffix {
                    format!("Complete: {}____{}", prefix, s)
                } else {
                    format!("Complete: {}", prefix)
                }
            }
            _ => {
                if let Some(s) = suffix {
                    format!("Complete the missing part:\n{}[COMPLETE HERE]{}", prefix, s)
                } else {
                    format!("Complete this code:\n{}", prefix)
                }
            }
        }
    }
    
    /// Get optimized prompt for language detection
    pub fn language_detection_prompt(code: &str, tier: ModelTier) -> String {
        match tier {
            ModelTier::Tiny => {
                // Qwen-optimized: Just the essential
                format!(
                    "Language?\n{}\nAnswer:",
                    &code[..code.len().min(200)]
                )
            }
            _ => {
                format!(
                    "Identify the programming language:\n\n{}\n\nLanguage:",
                    &code[..code.len().min(500)]
                )
            }
        }
    }
    
    /// Get optimized prompt for code quality check
    pub fn quality_check_prompt(code: &str, tier: ModelTier) -> String {
        match tier {
            ModelTier::Tiny => {
                // Qwen-optimized: Binary quality assessment
                format!(
                    "Code quality?\n{}\ngood/bad:",
                    &code[..code.len().min(300)]
                )
            }
            _ => {
                format!(
                    "Assess the quality of this code:\n\n{}\n\n\
                     Quality assessment:",
                    code
                )
            }
        }
    }
    
    /// Get optimized prompt for definition questions
    pub fn definition_prompt(term: &str, tier: ModelTier) -> String {
        match tier {
            ModelTier::Tiny => {
                // Qwen-optimized: One-line definition
                format!("Define {} in one sentence:", term)
            }
            _ => {
                format!("What is {}? Provide a clear, concise definition.", term)
            }
        }
    }
    
    /// Get system prompt optimized for model tier
    pub fn system_prompt(tier: ModelTier, task_type: &str) -> Option<String> {
        match tier {
            ModelTier::Tiny => {
                // Minimal system prompt for Qwen
                Some("Answer concisely.".to_string())
            }
            ModelTier::Small => {
                Some(format!("You are a helpful assistant for {}. Be concise.", task_type))
            }
            ModelTier::Medium | ModelTier::Large => {
                Some(format!(
                    "You are an expert assistant specializing in {}. \
                     Provide clear, accurate, and well-structured responses.",
                    task_type
                ))
            }
            ModelTier::NoLLM => None,
        }
    }
}

/// Prompt optimization strategies for Qwen 0.5B
pub struct QwenPromptOptimizer;

impl QwenPromptOptimizer {
    /// Optimize any prompt for Qwen's capabilities
    pub fn optimize(prompt: &str) -> String {
        // Remove unnecessary words and formatting
        let optimized = prompt
            .replace("Please ", "")
            .replace("Could you ", "")
            .replace("Would you ", "")
            .replace("Can you ", "")
            .replace("I need you to ", "")
            .replace("I want you to ", "")
            .replace("\n\n\n", "\n")
            .replace("\n\n", "\n");
        
        // Truncate to reasonable length for Qwen
        if optimized.len() > 500 {
            format!("{}...", &optimized[..500])
        } else {
            optimized
        }
    }
    
    /// Create a structured prompt for better Qwen performance
    pub fn structured_prompt(task: &str, input: &str, expected_format: &str) -> String {
        format!(
            "Task: {}\nInput: {}\nFormat: {}\nAnswer:",
            task,
            &input[..input.len().min(200)],
            expected_format
        )
    }
    
    /// Create a few-shot prompt for Qwen
    pub fn few_shot_prompt(examples: &[(String, String)], query: &str) -> String {
        let mut prompt = String::new();
        
        // Limit to 2-3 examples for Qwen
        for (input, output) in examples.iter().take(3) {
            prompt.push_str(&format!("Q: {}\nA: {}\n", input, output));
        }
        
        prompt.push_str(&format!("Q: {}\nA:", query));
        prompt
    }
}

/// Task-specific prompts optimized for different model capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPrompt {
    pub task_type: TaskType,
    pub tier: ModelTier,
    pub template: String,
    pub max_input_length: usize,
    pub expected_output_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    Classification,
    YesNo,
    PatternDetection,
    RequirementCheck,
    Completion,
    LanguageDetection,
    QualityCheck,
    Definition,
    SimpleAnswer,
}

impl TaskPrompt {
    /// Create an optimized prompt for Qwen classification
    pub fn qwen_classification() -> Self {
        Self {
            task_type: TaskType::Classification,
            tier: ModelTier::Tiny,
            template: "Type: {input}\nIs: ".to_string(),
            max_input_length: 200,
            expected_output_format: "code|doc|test|config|other".to_string(),
        }
    }
    
    /// Create an optimized prompt for Qwen yes/no
    pub fn qwen_yes_no() -> Self {
        Self {
            task_type: TaskType::YesNo,
            tier: ModelTier::Tiny,
            template: "{question}\nyes/no:".to_string(),
            max_input_length: 150,
            expected_output_format: "yes|no".to_string(),
        }
    }
    
    /// Create an optimized prompt for Qwen pattern detection
    pub fn qwen_pattern() -> Self {
        Self {
            task_type: TaskType::PatternDetection,
            tier: ModelTier::Tiny,
            template: "Has {pattern}?\n{code}\nyes/no:".to_string(),
            max_input_length: 300,
            expected_output_format: "yes|no".to_string(),
        }
    }
}

/// Prompt chain for complex tasks that need decomposition
pub struct PromptChain {
    steps: Vec<ChainStep>,
}

impl PromptChain {
    /// Create a chain optimized for Qwen to handle complex tasks
    pub fn for_complex_task(task: &str) -> Self {
        let steps = vec![
            ChainStep {
                prompt: format!("Is this about code? {}\nyes/no:", task),
                tier: ModelTier::Tiny,
                next_on_yes: Some(1),
                next_on_no: Some(2),
            },
            ChainStep {
                prompt: format!("What language? {}\nAnswer:", task),
                tier: ModelTier::Tiny,
                next_on_yes: None,
                next_on_no: None,
            },
            ChainStep {
                prompt: format!("What type of task? {}\nAnswer:", task),
                tier: ModelTier::Tiny,
                next_on_yes: None,
                next_on_no: None,
            },
        ];
        
        Self { steps }
    }
    
    /// Execute the chain and collect results
    pub async fn execute<F>(
        &self,
        executor: F,
    ) -> Result<Vec<String>, anyhow::Error>
    where
        F: Fn(&str, ModelTier) -> Result<String, anyhow::Error>,
    {
        let mut results = Vec::new();
        let mut current_step = 0;
        
        while current_step < self.steps.len() {
            let step = &self.steps[current_step];
            let result = executor(&step.prompt, step.tier)?;
            results.push(result.clone());
            
            // Determine next step
            if result.to_lowercase().contains("yes") {
                if let Some(next) = step.next_on_yes {
                    current_step = next;
                } else {
                    break;
                }
            } else {
                if let Some(next) = step.next_on_no {
                    current_step = next;
                } else {
                    break;
                }
            }
        }
        
        Ok(results)
    }
}

#[derive(Debug, Clone)]
struct ChainStep {
    prompt: String,
    tier: ModelTier,
    next_on_yes: Option<usize>,
    next_on_no: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_qwen_prompt_optimization() {
        let verbose = "Please could you tell me what this code does?";
        let optimized = QwenPromptOptimizer::optimize(verbose);
        assert!(!optimized.contains("Please"));
        assert!(!optimized.contains("could you"));
    }
    
    #[test]
    fn test_classification_prompts() {
        let content = "fn main() { println!(\"Hello\"); }";
        
        let tiny_prompt = PromptTemplates::classification_prompt(content, ModelTier::Tiny);
        assert!(tiny_prompt.len() < 300);
        assert!(tiny_prompt.contains("Answer:"));
        
        let large_prompt = PromptTemplates::classification_prompt(content, ModelTier::Large);
        assert!(large_prompt.len() > tiny_prompt.len());
    }
    
    #[test]
    fn test_few_shot_prompt() {
        let examples = vec![
            ("2+2".to_string(), "4".to_string()),
            ("3+3".to_string(), "6".to_string()),
        ];
        
        let prompt = QwenPromptOptimizer::few_shot_prompt(&examples, "4+4");
        assert!(prompt.contains("Q: 2+2"));
        assert!(prompt.contains("A: 4"));
        assert!(prompt.ends_with("Q: 4+4\nA:"));
    }
}