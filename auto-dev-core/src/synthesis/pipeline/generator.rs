//! Code generation coordinator

use super::{PipelineStage, PipelineContext};
use crate::{
    synthesis::{Result, SynthesisError},
    llm::provider::LLMProvider,
};
use async_trait::async_trait;
use std::path::PathBuf;

/// Coordinates code generation through LLM
pub struct CodeGenerator {
    cache: GenerationCache,
}

impl CodeGenerator {
    /// Create a new code generator
    pub fn new() -> Self {
        Self {
            cache: GenerationCache::new(),
        }
    }
    
    /// Generate code for a task
    async fn generate_for_task(
        &self,
        task: &crate::synthesis::state::ImplementationTask,
        provider: &dyn LLMProvider,
    ) -> Result<GeneratedCode> {
        // Check cache first
        if let Some(cached) = self.cache.get(&task.id) {
            tracing::debug!("Using cached generation for task: {}", task.id);
            return Ok(cached);
        }
        
        // Build generation prompt
        let prompt = self.build_prompt(task);
        
        // Generate code via LLM
        let response = provider.generate(&prompt, None).await
            .map_err(|e| SynthesisError::GenerationError(e.to_string()))?;
        
        let generated = GeneratedCode {
            task_id: task.id.clone(),
            code: response.content,
            language: self.detect_language(&task.target_file),
            imports: Vec::new(),
            exports: Vec::new(),
        };
        
        // Cache the result
        self.cache.store(task.id.clone(), generated.clone());
        
        Ok(generated)
    }
    
    /// Build generation prompt for task
    fn build_prompt(&self, task: &crate::synthesis::state::ImplementationTask) -> String {
        format!(
            "Generate code for the following task:\n\
             Task: {}\n\
             Target file: {}\n\
             \n\
             Requirements:\n\
             - Follow Rust best practices\n\
             - Include proper error handling\n\
             - Add documentation comments\n\
             - Make the code modular and testable\n\
             \n\
             Generate only the code without explanations.",
            task.description,
            task.target_file.display()
        )
    }
    
    /// Detect programming language from file extension
    fn detect_language(&self, path: &PathBuf) -> String {
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("txt")
            .to_string()
    }
    
    /// Process all pending tasks
    async fn process_tasks(
        &self,
        context: &mut PipelineContext,
        provider: &dyn LLMProvider,
    ) -> Result<Vec<GeneratedCode>> {
        let mut generated = Vec::new();
        let mut completed = Vec::new();
        
        // Process tasks based on configuration
        let batch_size = if context.config.incremental {
            1  // Process one at a time for incremental
        } else {
            context.config.parallel_tasks
        };
        
        for chunk in context.pending_tasks.chunks(batch_size) {
            let mut chunk_results = Vec::new();
            
            for task in chunk {
                match self.generate_for_task(task, provider).await {
                    Ok(code) => {
                        chunk_results.push(code);
                        let mut completed_task = task.clone();
                        completed_task.complete();
                        completed.push(completed_task);
                    }
                    Err(e) => {
                        context.add_warning(format!(
                            "Failed to generate code for task {}: {}",
                            task.id, e
                        ));
                    }
                }
            }
            
            generated.extend(chunk_results);
        }
        
        // Move completed tasks
        context.completed_tasks.extend(completed);
        context.pending_tasks.retain(|t| {
            !context.completed_tasks.iter().any(|ct| ct.id == t.id)
        });
        
        Ok(generated)
    }
}

#[async_trait]
impl PipelineStage for CodeGenerator {
    fn name(&self) -> &'static str {
        "CodeGenerator"
    }
    
    async fn execute(&self, mut context: PipelineContext) -> Result<PipelineContext> {
        tracing::info!("Generating code for {} tasks", context.pending_tasks.len());
        
        context.metadata.current_stage = self.name().to_string();
        
        if context.pending_tasks.is_empty() {
            context.add_warning("No tasks to generate code for".to_string());
            return Ok(context);
        }
        
        // Get LLM provider
        // In a real implementation, this would be injected or configured
        let provider = self.get_provider(&context)?;
        
        // Generate code for all tasks
        let generated = self.process_tasks(&mut context, provider.as_ref()).await?;
        
        // Store generated code in context
        // In a real implementation, this would be more sophisticated
        for gen in generated {
            let path = context.pending_tasks.iter()
                .find(|t| t.id == gen.task_id)
                .map(|t| t.target_file.clone())
                .unwrap_or_else(|| PathBuf::from("generated.rs"));
            
            context.add_generated_file(path);
        }
        
        tracing::debug!(
            "Generated code for {} tasks, {} completed",
            context.completed_tasks.len(),
            context.completed_tasks.len()
        );
        
        Ok(context)
    }
}

impl CodeGenerator {
    /// Get LLM provider based on configuration
    fn get_provider(&self, context: &PipelineContext) -> Result<Box<dyn LLMProvider>> {
        // This is a placeholder - in real implementation would use the configured provider
        // For now, return an error indicating provider needs to be configured
        Err(SynthesisError::GenerationError(
            "LLM provider not configured. Please configure an LLM provider in the synthesis config.".to_string()
        ))
    }
}

/// Generated code structure
#[derive(Debug, Clone)]
struct GeneratedCode {
    task_id: String,
    code: String,
    language: String,
    imports: Vec<String>,
    exports: Vec<String>,
}

/// Simple in-memory cache for generated code
struct GenerationCache {
    cache: std::sync::RwLock<HashMap<String, GeneratedCode>>,
}

use std::collections::HashMap;

impl GenerationCache {
    fn new() -> Self {
        Self {
            cache: std::sync::RwLock::new(HashMap::new()),
        }
    }
    
    fn get(&self, key: &str) -> Option<GeneratedCode> {
        self.cache.read().ok()?.get(key).cloned()
    }
    
    fn store(&self, key: String, value: GeneratedCode) {
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(key, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_language_detection() {
        let generator = CodeGenerator::new();
        
        assert_eq!(
            generator.detect_language(&PathBuf::from("test.rs")),
            "rs"
        );
        assert_eq!(
            generator.detect_language(&PathBuf::from("test.py")),
            "py"
        );
        assert_eq!(
            generator.detect_language(&PathBuf::from("test.js")),
            "js"
        );
    }
    
    #[test]
    fn test_cache() {
        let cache = GenerationCache::new();
        
        let code = GeneratedCode {
            task_id: "test".to_string(),
            code: "fn test() {}".to_string(),
            language: "rs".to_string(),
            imports: Vec::new(),
            exports: Vec::new(),
        };
        
        cache.store("test".to_string(), code.clone());
        
        let retrieved = cache.get("test");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().task_id, "test");
    }
}