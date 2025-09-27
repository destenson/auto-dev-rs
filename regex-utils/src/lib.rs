//! Regex utilities for auto-dev
//! Extracted to a separate crate for compilation optimization

use once_cell::sync::Lazy;
use regex::Regex;

/// Compiled regex patterns for project name extraction
pub mod project_name {
    use super::*;
    
    pub static CREATE_PATTERN: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?:create|build|make|develop)\s+(?:a|an)?\s+.*?(?:called|named)\s+(\w+)")
            .expect("Invalid regex pattern")
    });
    
    pub static NAME_PATTERN: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?:project|name):\s*(\w+)")
            .expect("Invalid regex pattern")
    });
    
    /// Extract project name from text
    pub fn extract(text: &str) -> Option<String> {
        if let Some(caps) = CREATE_PATTERN.captures(text) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        
        if let Some(caps) = NAME_PATTERN.captures(text) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        
        None
    }
}

/// Language detection patterns
pub mod language {
    use super::*;
    
    pub struct LanguageMatcher {
        patterns: Vec<(&'static str, &'static str)>,
    }
    
    impl LanguageMatcher {
        pub fn new() -> Self {
            Self {
                patterns: vec![
                    ("rust", "Rust"),
                    ("python", "Python"),
                    ("javascript", "JavaScript"),
                    ("typescript", "TypeScript"),
                    ("java(?!script)", "Java"),  // Java but not JavaScript
                    (r"\bc#", "C#"),
                    ("csharp", "C#"),
                    (r"\.net", ".NET"),
                    ("golang", "Go"),
                    (r"\bgo\b", "Go"),
                    ("deno", "Deno"),
                ],
            }
        }
        
        pub fn detect(&self, text: &str) -> Option<String> {
            let text_lower = text.to_lowercase();
            
            for (pattern, name) in &self.patterns {
                if Regex::new(pattern)
                    .ok()
                    .and_then(|re| re.find(&text_lower))
                    .is_some()
                {
                    return Some(name.to_string());
                }
            }
            
            None
        }
    }
    
    impl Default for LanguageMatcher {
        fn default() -> Self {
            Self::new()
        }
    }
}

/// Framework detection patterns  
pub mod framework {
    pub fn detect(text: &str) -> Option<String> {
        let text_lower = text.to_lowercase();
        
        let frameworks = [
            ("actix", "Actix-Web"),
            ("axum", "Axum"),
            ("rocket", "Rocket"),
            ("django", "Django"),
            ("flask", "Flask"),
            ("fastapi", "FastAPI"),
            ("express", "Express"),
            ("react", "React"),
            ("vue", "Vue"),
            ("angular", "Angular"),
            ("spring", "Spring"),
            ("gin", "Gin"),
            ("echo", "Echo"),
            ("fresh", "Fresh"),  // Deno framework
            ("oak", "Oak"),      // Deno framework
        ];
        
        for (keyword, name) in &frameworks {
            if text_lower.contains(keyword) {
                return Some(name.to_string());
            }
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_project_name_extraction() {
        assert_eq!(
            project_name::extract("create a rust cli called mycli"),
            Some("mycli".to_string())
        );
        
        assert_eq!(
            project_name::extract("project: myproject"),
            Some("myproject".to_string())
        );
    }
    
    #[test]
    fn test_language_detection() {
        let matcher = language::LanguageMatcher::new();
        
        assert_eq!(
            matcher.detect("build a rust application"),
            Some("Rust".to_string())
        );
        
        assert_eq!(
            matcher.detect("create a java app"),
            Some("Java".to_string())
        );
        
        assert_eq!(
            matcher.detect("javascript project"),
            Some("JavaScript".to_string())
        );
    }
}