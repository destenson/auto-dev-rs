//! Natural language requirement extraction

use regex::Regex;
use anyhow::Result;
use std::collections::HashSet;

use crate::parser::model::*;

/// Extracts requirements from natural language text
#[derive(Debug)]
pub struct RequirementExtractor {
    requirement_patterns: Vec<Regex>,
    entity_patterns: Vec<Regex>,
    action_patterns: Vec<Regex>,
}

impl RequirementExtractor {
    /// Create a new requirement extractor
    pub fn new() -> Self {
        Self {
            requirement_patterns: Self::compile_requirement_patterns(),
            entity_patterns: Self::compile_entity_patterns(),
            action_patterns: Self::compile_action_patterns(),
        }
    }

    /// Extract requirements from text
    pub fn extract_from_text(&self, text: &str) -> Result<Vec<Requirement>> {
        let mut requirements = Vec::new();
        let mut seen_descriptions = HashSet::new();
        
        // Split text into sentences
        let sentences = self.split_into_sentences(text);
        
        for (idx, sentence) in sentences.iter().enumerate() {
            // Check if sentence contains requirement keywords
            if let Some(req) = self.extract_requirement_from_sentence(sentence, idx) {
                // Avoid duplicates
                if !seen_descriptions.contains(&req.description) {
                    seen_descriptions.insert(req.description.clone());
                    requirements.push(req);
                }
            }
        }
        
        // Extract implied requirements from patterns
        let implied_reqs = self.extract_implied_requirements(text)?;
        for req in implied_reqs {
            if !seen_descriptions.contains(&req.description) {
                seen_descriptions.insert(req.description.clone());
                requirements.push(req);
            }
        }
        
        Ok(requirements)
    }

    /// Extract a requirement from a single sentence
    fn extract_requirement_from_sentence(&self, sentence: &str, index: usize) -> Option<Requirement> {
        let sentence = sentence.trim();
        
        // Check against requirement patterns
        for pattern in &self.requirement_patterns {
            if pattern.is_match(sentence) {
                let priority = self.determine_priority(sentence);
                let category = self.determine_category(sentence);
                
                let mut requirement = Requirement::new(
                    format!("REQ-{:04}", index + 1),
                    sentence.to_string(),
                );
                
                requirement.priority = priority;
                requirement.category = category;
                
                // Extract entities and actions
                requirement.tags = self.extract_entities(sentence);
                
                return Some(requirement);
            }
        }
        
        None
    }

    /// Extract implied requirements from common patterns
    fn extract_implied_requirements(&self, text: &str) -> Result<Vec<Requirement>> {
        let mut requirements = Vec::new();
        
        // Check for authentication mentions
        if text.contains("authentication") || text.contains("login") || text.contains("auth") {
            requirements.push(Requirement {
                id: "REQ-AUTH-001".to_string(),
                description: "System must provide user authentication mechanism".to_string(),
                category: RequirementType::Security,
                priority: Priority::High,
                acceptance_criteria: vec![
                    "Users can login with credentials".to_string(),
                    "Invalid credentials are rejected".to_string(),
                    "Sessions are properly managed".to_string(),
                ],
                source_location: SourceLocation::default(),
                related: Vec::new(),
                tags: vec!["authentication".to_string(), "security".to_string()],
            });
        }
        
        // Check for API mentions
        if text.contains("API") || text.contains("endpoint") || text.contains("REST") {
            requirements.push(Requirement {
                id: "REQ-API-001".to_string(),
                description: "System must provide API endpoints for integration".to_string(),
                category: RequirementType::Api,
                priority: Priority::High,
                acceptance_criteria: vec![
                    "API endpoints are documented".to_string(),
                    "API returns appropriate status codes".to_string(),
                    "API handles errors gracefully".to_string(),
                ],
                source_location: SourceLocation::default(),
                related: Vec::new(),
                tags: vec!["api".to_string(), "integration".to_string()],
            });
        }
        
        // Check for data validation mentions
        if text.contains("validat") || text.contains("verify") || text.contains("check") {
            requirements.push(Requirement {
                id: "REQ-VAL-001".to_string(),
                description: "System must validate all user inputs".to_string(),
                category: RequirementType::Security,
                priority: Priority::High,
                acceptance_criteria: vec![
                    "Input validation is performed on all forms".to_string(),
                    "Invalid input returns clear error messages".to_string(),
                ],
                source_location: SourceLocation::default(),
                related: Vec::new(),
                tags: vec!["validation".to_string(), "security".to_string()],
            });
        }
        
        // Check for performance mentions
        if text.contains("performance") || text.contains("fast") || text.contains("responsive") {
            requirements.push(Requirement {
                id: "REQ-PERF-001".to_string(),
                description: "System must meet performance requirements".to_string(),
                category: RequirementType::Performance,
                priority: Priority::Medium,
                acceptance_criteria: vec![
                    "Page load times under 2 seconds".to_string(),
                    "API response times under 500ms".to_string(),
                ],
                source_location: SourceLocation::default(),
                related: Vec::new(),
                tags: vec!["performance".to_string()],
            });
        }
        
        Ok(requirements)
    }

    /// Determine priority from text
    fn determine_priority(&self, text: &str) -> Priority {
        let text_lower = text.to_lowercase();
        
        if text_lower.contains("must") || text_lower.contains("shall") || 
           text_lower.contains("critical") || text_lower.contains("required") {
            Priority::Critical
        } else if text_lower.contains("should") || text_lower.contains("important") {
            Priority::High
        } else if text_lower.contains("could") || text_lower.contains("may") {
            Priority::Medium
        } else if text_lower.contains("might") || text_lower.contains("nice to have") {
            Priority::Low
        } else {
            Priority::Medium
        }
    }

    /// Determine requirement category from text
    fn determine_category(&self, text: &str) -> RequirementType {
        let text_lower = text.to_lowercase();
        
        if text_lower.contains("api") || text_lower.contains("endpoint") || 
           text_lower.contains("interface") {
            RequirementType::Api
        } else if text_lower.contains("data") || text_lower.contains("model") || 
                  text_lower.contains("schema") {
            RequirementType::DataModel
        } else if text_lower.contains("secure") || text_lower.contains("auth") || 
                  text_lower.contains("encrypt") || text_lower.contains("password") {
            RequirementType::Security
        } else if text_lower.contains("perform") || text_lower.contains("fast") || 
                  text_lower.contains("speed") || text_lower.contains("response") {
            RequirementType::Performance
        } else if text_lower.contains("behav") || text_lower.contains("when") || 
                  text_lower.contains("scenario") {
            RequirementType::Behavior
        } else if text_lower.contains("user") && text_lower.contains("interface") || 
                  text_lower.contains("ui") || text_lower.contains("ux") {
            RequirementType::Usability
        } else {
            RequirementType::Functional
        }
    }

    /// Extract entities from text
    fn extract_entities(&self, text: &str) -> Vec<String> {
        let mut entities = Vec::new();
        
        for pattern in &self.entity_patterns {
            for cap in pattern.captures_iter(text) {
                if let Some(entity) = cap.get(1) {
                    entities.push(entity.as_str().to_lowercase());
                }
            }
        }
        
        // Also extract common nouns as potential entities
        let words: Vec<&str> = text.split_whitespace().collect();
        for word in words {
            let word_lower = word.to_lowercase();
            if Self::is_likely_entity(&word_lower) {
                if !entities.contains(&word_lower) {
                    entities.push(word_lower);
                }
            }
        }
        
        entities
    }

    /// Check if a word is likely an entity
    fn is_likely_entity(word: &str) -> bool {
        // Common entities in software specifications
        matches!(word, 
            "user" | "users" | "system" | "database" | "api" | "server" | 
            "client" | "admin" | "administrator" | "account" | "profile" |
            "data" | "file" | "document" | "report" | "dashboard" | "page" |
            "form" | "button" | "field" | "table" | "list" | "menu" |
            "service" | "component" | "module" | "feature" | "function"
        )
    }

    /// Split text into sentences
    fn split_into_sentences(&self, text: &str) -> Vec<String> {
        let mut sentences = Vec::new();
        let mut current = String::new();
        
        for line in text.lines() {
            let line = line.trim();
            
            // Skip empty lines
            if line.is_empty() {
                if !current.is_empty() {
                    sentences.push(current.clone());
                    current.clear();
                }
                continue;
            }
            
            // Check if line starts with a list marker
            if line.starts_with('-') || line.starts_with('*') || line.starts_with("•") {
                if !current.is_empty() {
                    sentences.push(current.clone());
                    current.clear();
                }
                sentences.push(line.trim_start_matches(|c| c == '-' || c == '*' || c == '•').trim().to_string());
            } else {
                // Add to current sentence
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(line);
                
                // Check for sentence end
                if line.ends_with('.') || line.ends_with('!') || line.ends_with('?') {
                    sentences.push(current.clone());
                    current.clear();
                }
            }
        }
        
        // Add any remaining content
        if !current.is_empty() {
            sentences.push(current);
        }
        
        sentences
    }

    /// Compile requirement detection patterns
    fn compile_requirement_patterns() -> Vec<Regex> {
        vec![
            Regex::new(r"(?i)\b(the\s+)?(system|application|software|service|api|user|admin)\s+(must|shall|should|could|may|might)\s+").unwrap(),
            Regex::new(r"(?i)\b(must|shall|should|could|may)\s+(be\s+able\s+to|support|provide|allow|enable|ensure|validate|verify)\b").unwrap(),
            Regex::new(r"(?i)\b(it\s+is\s+)?(required|necessary|critical|important|essential)\s+(that|to)\b").unwrap(),
            Regex::new(r"(?i)\bneeds?\s+to\s+").unwrap(),
            Regex::new(r"(?i)\b(feature|functionality|capability):\s*").unwrap(),
            Regex::new(r"(?i)^\s*\[?(req|requirement|spec|specification)\]?[:\s]").unwrap(),
        ]
    }

    /// Compile entity detection patterns
    fn compile_entity_patterns() -> Vec<Regex> {
        vec![
            Regex::new(r"(?i)\b(user|admin|customer|client|operator|manager)\b").unwrap(),
            Regex::new(r"(?i)\b(system|application|service|component|module)\b").unwrap(),
            Regex::new(r"(?i)\b(database|api|server|endpoint|interface)\b").unwrap(),
            Regex::new(r"(?i)\b(data|file|document|record|entity)\b").unwrap(),
        ]
    }

    /// Compile action detection patterns
    fn compile_action_patterns() -> Vec<Regex> {
        vec![
            Regex::new(r"(?i)\b(create|read|update|delete|add|remove|modify)\b").unwrap(),
            Regex::new(r"(?i)\b(login|logout|authenticate|authorize|verify)\b").unwrap(),
            Regex::new(r"(?i)\b(validate|check|ensure|confirm|test)\b").unwrap(),
            Regex::new(r"(?i)\b(send|receive|process|handle|manage)\b").unwrap(),
        ]
    }
}

impl Default for RequirementExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_requirement_extractor_creation() {
        let extractor = RequirementExtractor::new();
        assert!(!extractor.requirement_patterns.is_empty());
    }

    #[test]
    fn test_extract_simple_requirements() {
        let extractor = RequirementExtractor::new();
        
        let text = "The system must support user authentication. \
                    Users should be able to reset their passwords. \
                    The API could provide rate limiting.";
        
        let requirements = extractor.extract_from_text(text).unwrap();
        assert!(!requirements.is_empty());
        
        // Check priorities
        let must_req = requirements.iter()
            .find(|r| r.description.contains("must"));
        assert!(must_req.is_some());
        assert_eq!(must_req.unwrap().priority, Priority::Critical);
        
        // Find any requirement with High priority
        let high_priority_req = requirements.iter()
            .find(|r| r.priority == Priority::High);
        assert!(high_priority_req.is_some());
    }

    #[test]
    fn test_determine_category() {
        let extractor = RequirementExtractor::new();
        
        assert_eq!(
            extractor.determine_category("The API must return JSON"),
            RequirementType::Api
        );
        assert_eq!(
            extractor.determine_category("User authentication is required"),
            RequirementType::Security
        );
        assert_eq!(
            extractor.determine_category("Response time must be under 1 second"),
            RequirementType::Performance
        );
        assert_eq!(
            extractor.determine_category("The data model should include user profile"),
            RequirementType::DataModel
        );
    }

    #[test]
    fn test_extract_entities() {
        let extractor = RequirementExtractor::new();
        
        let text = "The user must be able to access the API through the system";
        let entities = extractor.extract_entities(text);
        
        assert!(entities.contains(&"user".to_string()));
        assert!(entities.contains(&"api".to_string()));
        assert!(entities.contains(&"system".to_string()));
    }

    #[test]
    fn test_split_sentences() {
        let extractor = RequirementExtractor::new();
        
        let text = "First sentence. Second sentence!\n- List item one\n- List item two\nThird sentence?";
        let sentences = extractor.split_into_sentences(text);
        
        // The function concatenates sentences on the same line, so we get 4 sentences
        assert_eq!(sentences.len(), 4);
        assert_eq!(sentences[0], "First sentence. Second sentence!");
        assert_eq!(sentences[1], "List item one");
        assert_eq!(sentences[2], "List item two");
        assert_eq!(sentences[3], "Third sentence?");
    }

    #[test]
    fn test_implied_requirements() {
        let extractor = RequirementExtractor::new();
        
        let text = "The system provides authentication and a REST API for integration";
        let requirements = extractor.extract_from_text(text).unwrap();
        
        // Should find implied auth and API requirements
        let auth_req = requirements.iter()
            .find(|r| r.category == RequirementType::Security);
        assert!(auth_req.is_some());
        
        let api_req = requirements.iter()
            .find(|r| r.category == RequirementType::Api);
        assert!(api_req.is_some());
    }
}