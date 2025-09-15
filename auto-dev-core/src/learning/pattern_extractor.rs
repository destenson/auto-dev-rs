use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::context::analyzer::patterns::CodePattern;
use crate::incremental::Implementation;
use crate::parser::model::Specification;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternExtractor {
    pub min_complexity: usize,
    pub max_complexity: usize,
    pub similarity_threshold: f32,
}

impl PatternExtractor {
    pub fn new() -> Self {
        Self { min_complexity: 3, max_complexity: 100, similarity_threshold: 0.8 }
    }

    pub fn extract_patterns(
        &self,
        implementation: &Implementation,
        context: &crate::learning::learner::EventContext,
    ) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        let code = self.get_code_from_implementation(implementation);
        patterns.extend(self.extract_structural_patterns_from_code(&code));
        patterns.extend(self.extract_behavioral_patterns_from_code(&code));
        patterns.extend(self.extract_idioms_from_code(&code));

        patterns.retain(|p| p.quality_score() > 0.7);

        for pattern in &mut patterns {
            pattern.context = PatternContext::from_event_context(context);
        }

        patterns
    }

    fn get_code_from_implementation(&self, implementation: &Implementation) -> String {
        implementation.files.iter().map(|f| f.content.clone()).collect::<Vec<_>>().join("\n")
    }

    fn extract_structural_patterns(&self, implementation: &Implementation) -> Vec<Pattern> {
        let code = self.get_code_from_implementation(implementation);
        self.extract_structural_patterns_from_code(&code)
    }

    fn extract_structural_patterns_from_code(&self, code: &str) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        if let Some(code_patterns) = self.analyze_code_structure(code) {
            for code_pattern in code_patterns {
                if self.is_valid_pattern(&code_pattern) {
                    patterns.push(self.convert_to_pattern(code_pattern, PatternType::Structural));
                }
            }
        }

        patterns
    }

    fn extract_behavioral_patterns(&self, implementation: &Implementation) -> Vec<Pattern> {
        let code = self.get_code_from_implementation(implementation);
        self.extract_behavioral_patterns_from_code(&code)
    }

    fn extract_behavioral_patterns_from_code(&self, code: &str) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        if let Some(behaviors) = self.analyze_behavior(code) {
            for behavior in behaviors {
                patterns.push(Pattern {
                    id: Uuid::new_v4(),
                    name: behavior.name,
                    description: behavior.description,
                    pattern_type: PatternType::Behavioral,
                    context: PatternContext::default(),
                    implementation: behavior.code,
                    success_rate: 1.0,
                    usage_count: 0,
                    learned_at: Utc::now(),
                    embeddings: None,
                    tags: behavior.tags,
                    complexity: behavior.complexity,
                    reusability_score: 0.8,
                    test_coverage: 0.0,
                });
            }
        }

        patterns
    }

    fn extract_idioms(&self, implementation: &Implementation) -> Vec<Pattern> {
        let code = self.get_code_from_implementation(implementation);
        self.extract_idioms_from_code(&code)
    }

    fn extract_idioms_from_code(&self, code: &str) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        let idioms = vec![
            ("error_handling", r"Result<.*>", "Rust error handling pattern"),
            ("iterator_chain", r"\.iter\(\).*\.map\(.*\)\.collect", "Iterator chain pattern"),
            ("builder_pattern", r"Builder.*\.build\(\)", "Builder pattern"),
            ("async_await", r"async.*await", "Async/await pattern"),
            ("match_expression", r"match.*\{[^}]+\}", "Match expression pattern"),
        ];

        for (name, regex_str, description) in idioms {
            if let Ok(regex) = regex::Regex::new(regex_str) {
                if regex.is_match(code) {
                    patterns.push(Pattern {
                        id: Uuid::new_v4(),
                        name: name.to_string(),
                        description: description.to_string(),
                        pattern_type: PatternType::Idiom,
                        context: PatternContext::default(),
                        implementation: extract_matching_code(code, &regex),
                        success_rate: 1.0,
                        usage_count: 0,
                        learned_at: Utc::now(),
                        embeddings: None,
                        tags: vec![name.to_string()],
                        complexity: 1,
                        reusability_score: 0.9,
                        test_coverage: 0.0,
                    });
                }
            }
        }

        patterns
    }

    fn analyze_code_structure(&self, code: &str) -> Option<Vec<CodeStructure>> {
        let mut structures = Vec::new();

        let function_regex =
            regex::Regex::new(r"(?m)^(?:pub\s+)?(?:async\s+)?fn\s+(\w+)[^{]*\{").ok()?;
        let struct_regex = regex::Regex::new(r"(?m)^(?:pub\s+)?struct\s+(\w+)").ok()?;
        let impl_regex = regex::Regex::new(r"(?m)^impl(?:\s+\w+\s+for)?\s+(\w+)").ok()?;

        for cap in function_regex.captures_iter(code) {
            if let Some(name) = cap.get(1) {
                structures.push(CodeStructure {
                    name: name.as_str().to_string(),
                    structure_type: "function".to_string(),
                    code: extract_block(code, cap.get(0)?.start()),
                    complexity: calculate_complexity(code, cap.get(0)?.start()),
                });
            }
        }

        for cap in struct_regex.captures_iter(code) {
            if let Some(name) = cap.get(1) {
                structures.push(CodeStructure {
                    name: name.as_str().to_string(),
                    structure_type: "struct".to_string(),
                    code: extract_struct_block(code, cap.get(0)?.start()),
                    complexity: 1,
                });
            }
        }

        for cap in impl_regex.captures_iter(code) {
            if let Some(name) = cap.get(1) {
                structures.push(CodeStructure {
                    name: name.as_str().to_string(),
                    structure_type: "impl".to_string(),
                    code: extract_block(code, cap.get(0)?.start()),
                    complexity: calculate_complexity(code, cap.get(0)?.start()),
                });
            }
        }

        if structures.is_empty() { None } else { Some(structures) }
    }

    fn analyze_behavior(&self, code: &str) -> Option<Vec<BehaviorPattern>> {
        let mut behaviors = Vec::new();

        if code.contains("loop") || code.contains("while") {
            behaviors.push(BehaviorPattern {
                name: "iteration".to_string(),
                description: "Iterative processing pattern".to_string(),
                code: extract_loop_pattern(code),
                tags: vec!["loop".to_string(), "iteration".to_string()],
                complexity: 2,
            });
        }

        if code.contains("if") || code.contains("match") {
            behaviors.push(BehaviorPattern {
                name: "conditional".to_string(),
                description: "Conditional logic pattern".to_string(),
                code: extract_conditional_pattern(code),
                tags: vec!["conditional".to_string(), "branching".to_string()],
                complexity: 2,
            });
        }

        if code.contains("async") && code.contains("await") {
            behaviors.push(BehaviorPattern {
                name: "async_operation".to_string(),
                description: "Asynchronous operation pattern".to_string(),
                code: extract_async_pattern(code),
                tags: vec!["async".to_string(), "concurrent".to_string()],
                complexity: 3,
            });
        }

        if behaviors.is_empty() { None } else { Some(behaviors) }
    }

    fn is_valid_pattern(&self, structure: &CodeStructure) -> bool {
        structure.complexity >= self.min_complexity
            && structure.complexity <= self.max_complexity
            && !structure.code.is_empty()
    }

    fn convert_to_pattern(&self, structure: CodeStructure, pattern_type: PatternType) -> Pattern {
        Pattern {
            id: Uuid::new_v4(),
            name: structure.name.clone(),
            description: format!("{} pattern: {}", structure.structure_type, structure.name),
            pattern_type,
            context: PatternContext::default(),
            implementation: structure.code.clone(),
            success_rate: 1.0,
            usage_count: 0,
            learned_at: Utc::now(),
            embeddings: None,
            tags: vec![structure.structure_type.clone()],
            complexity: structure.complexity,
            reusability_score: calculate_reusability(&structure),
            test_coverage: 0.0,
        }
    }

    pub fn evaluate_pattern_quality(&self, pattern: &Pattern) -> f32 {
        let mut score = 0.0;

        score += pattern.reusability_score * 0.3;
        score += pattern.simplicity_score() * 0.2;
        score += pattern.test_coverage * 0.3;
        score += pattern.performance_score() * 0.2;

        score
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub pattern_type: PatternType,
    pub context: PatternContext,
    pub implementation: String,
    pub success_rate: f32,
    pub usage_count: u32,
    pub learned_at: DateTime<Utc>,
    pub embeddings: Option<Vec<f32>>,
    pub tags: Vec<String>,
    pub complexity: usize,
    pub reusability_score: f32,
    pub test_coverage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    Structural,
    Behavioral,
    Idiom,
    Architectural,
    Algorithm,
    ErrorHandling,
    Testing,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PatternContext {
    pub language: String,
    pub framework: Option<String>,
    pub project_type: String,
    pub dependencies: Vec<String>,
    pub constraints: HashMap<String, String>,
}

impl PatternContext {
    pub fn from_event_context(context: &crate::learning::learner::EventContext) -> Self {
        Self {
            language: context.language.clone(),
            framework: context.framework.clone(),
            project_type: context.project_type.clone(),
            dependencies: context.dependencies.clone(),
            constraints: HashMap::new(),
        }
    }

    pub fn matches(&self, other: &PatternContext) -> bool {
        self.language == other.language
            && self.project_type == other.project_type
            && (self.framework.is_none() || self.framework == other.framework)
    }
}

impl Pattern {
    pub fn quality_score(&self) -> f32 {
        let mut score = 0.0;

        score += self.success_rate * 0.3;
        score += self.reusability_score * 0.3;
        score += (self.usage_count.min(100) as f32 / 100.0) * 0.2;
        score += self.test_coverage * 0.1;
        score += self.simplicity_score() * 0.1;

        score
    }

    pub fn simplicity_score(&self) -> f32 {
        let max_complexity = 100.0;
        1.0 - (self.complexity as f32 / max_complexity).min(1.0)
    }

    pub fn performance_score(&self) -> f32 {
        self.success_rate
    }

    pub fn is_applicable(&self, context: &PatternContext) -> bool {
        self.context.matches(context)
    }

    pub fn adapt_to_context(&self, new_context: &PatternContext) -> Self {
        let mut adapted = self.clone();
        adapted.context = new_context.clone();
        adapted
    }
}

#[derive(Debug)]
struct CodeStructure {
    name: String,
    structure_type: String,
    code: String,
    complexity: usize,
}

#[derive(Debug)]
struct BehaviorPattern {
    name: String,
    description: String,
    code: String,
    tags: Vec<String>,
    complexity: usize,
}

fn extract_matching_code(code: &str, regex: &regex::Regex) -> String {
    if let Some(mat) = regex.find(code) {
        code[mat.start()..mat.end()].to_string()
    } else {
        String::new()
    }
}

fn extract_block(code: &str, start: usize) -> String {
    let code_bytes = code.as_bytes();
    let mut brace_count = 0;
    let mut in_block = false;
    let mut end = start;

    for i in start..code_bytes.len() {
        match code_bytes[i] {
            b'{' => {
                brace_count += 1;
                in_block = true;
            }
            b'}' => {
                brace_count -= 1;
                if brace_count == 0 && in_block {
                    end = i + 1;
                    break;
                }
            }
            _ => {}
        }
    }

    if end > start { code[start..end].to_string() } else { String::new() }
}

fn extract_struct_block(code: &str, start: usize) -> String {
    let code_bytes = code.as_bytes();
    let mut end = start;

    for i in start..code_bytes.len() {
        if code_bytes[i] == b'\n' {
            if i + 1 < code_bytes.len() && (code_bytes[i + 1] != b' ' && code_bytes[i + 1] != b'\t')
            {
                end = i;
                break;
            }
        }
    }

    if end > start {
        code[start..end].to_string()
    } else {
        code[start..].lines().next().unwrap_or("").to_string()
    }
}

fn calculate_complexity(code: &str, start: usize) -> usize {
    let block = extract_block(code, start);
    let mut complexity = 1;

    complexity += block.matches("if").count();
    complexity += block.matches("match").count();
    complexity += block.matches("loop").count();
    complexity += block.matches("while").count();
    complexity += block.matches("for").count();
    complexity += block.matches("?").count();

    complexity
}

fn calculate_reusability(structure: &CodeStructure) -> f32 {
    let mut score = 1.0;

    if structure.code.contains("pub") {
        score += 0.2;
    }

    if structure.code.contains("generic") || structure.code.contains("<T>") {
        score += 0.3;
    }

    if structure.complexity < 10 {
        score += 0.2;
    }

    (score / 1.7_f32).min(1.0_f32)
}

fn extract_loop_pattern(code: &str) -> String {
    if let Some(pos) = code.find("loop") {
        extract_block(code, pos)
    } else if let Some(pos) = code.find("while") {
        extract_block(code, pos)
    } else if let Some(pos) = code.find("for") {
        extract_block(code, pos)
    } else {
        String::new()
    }
}

fn extract_conditional_pattern(code: &str) -> String {
    if let Some(pos) = code.find("match") {
        extract_block(code, pos)
    } else if let Some(pos) = code.find("if") {
        let end = code[pos..].find("else").map(|e| pos + e).unwrap_or(code.len());
        code[pos..end].to_string()
    } else {
        String::new()
    }
}

fn extract_async_pattern(code: &str) -> String {
    if let Some(pos) = code.find("async") {
        let end = code[pos..].find("}").map(|e| pos + e + 1).unwrap_or(code.len());
        code[pos..end].to_string()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_extraction() {
        let extractor = PatternExtractor::new();
        let implementation = Implementation {
            code: r#"
                pub async fn process_data(input: &str) -> Result<String> {
                    let data = input.trim();
                    
                    match data {
                        "" => Err("Empty input"),
                        _ => Ok(data.to_string())
                    }
                }
            "#
            .to_string(),
            ..Default::default()
        };

        let context = crate::learning::learner::EventContext {
            project_type: "rust".to_string(),
            language: "rust".to_string(),
            framework: None,
            dependencies: vec![],
            environment: serde_json::json!({}),
        };

        let patterns = extractor.extract_patterns(&implementation, &context);
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_pattern_quality_score() {
        let pattern = Pattern {
            id: Uuid::new_v4(),
            name: "test_pattern".to_string(),
            description: "Test pattern".to_string(),
            pattern_type: PatternType::Structural,
            context: PatternContext::default(),
            implementation: "fn test() {}".to_string(),
            success_rate: 0.9,
            usage_count: 50,
            learned_at: Utc::now(),
            embeddings: None,
            tags: vec![],
            complexity: 5,
            reusability_score: 0.8,
            test_coverage: 0.7,
        };

        let score = pattern.quality_score();
        assert!(score > 0.0 && score <= 1.0);
    }
}
