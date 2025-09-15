//! Fast local classification without LLM for obvious cases

use super::{ClassificationResult, QuestionType};
use anyhow::Result;

/// Heuristic-based classifier for when LLM is unavailable
pub struct HeuristicClassifier;

impl HeuristicClassifier {
    pub fn new() -> Self {
        Self
    }

    /// Check if content is likely code using simple heuristics
    pub fn is_code(&self, content: &str) -> bool {
        // Common code indicators
        let code_patterns = [
            "fn ",
            "def ",
            "class ",
            "struct ",
            "impl ",
            "interface ",
            "function ",
            "const ",
            "let ",
            "var ",
            "import ",
            "export ",
            "if (",
            "for (",
            "while (",
            "return ",
            "async ",
            "await ",
            "{",
            "}",
            "()",
            "[]",
            "=>",
            "->",
            "::",
            "&&",
            "||",
        ];

        let lines: Vec<&str> = content.lines().take(20).collect();
        let mut code_score = 0;
        let mut total_score = 0;

        for line in lines {
            total_score += 1;
            for pattern in &code_patterns {
                if line.contains(pattern) {
                    code_score += 1;
                    break;
                }
            }
        }

        // If more than 30% of lines look like code
        total_score > 0 && (code_score as f32 / total_score as f32) > 0.3
    }

    /// Classify content using heuristics
    pub fn classify_content(&self, content: &str) -> ClassificationResult {
        let lower = content.to_lowercase();

        // Check for test indicators
        let is_test = lower.contains("#[test]")
            || lower.contains("describe(")
            || lower.contains("it(")
            || lower.contains("assert")
            || lower.contains("expect(");

        // Check for config files
        let is_config = content.starts_with('{')
            || content.starts_with('[')
            || lower.contains("version = ")
            || lower.contains("dependencies");

        // Check for documentation
        let is_doc = lower.contains("# ")
            || lower.contains("## ")
            || lower.contains("```")
            || (lower.contains("todo") && !self.is_code(content));

        let is_code = self.is_code(content);

        // Try to detect language
        let language = if is_code { self.detect_language(content) } else { None };

        ClassificationResult {
            is_code,
            is_documentation: is_doc && !is_code,
            is_test,
            is_config,
            language,
            confidence: 0.5, // Lower confidence for heuristics
        }
    }

    /// Simple language detection
    fn detect_language(&self, content: &str) -> Option<String> {
        let indicators = [
            ("rust", vec!["fn ", "impl ", "struct ", "trait ", "pub ", "use ", "mod ", "cargo"]),
            (
                "python",
                vec![
                    "def ",
                    "import ",
                    "from ",
                    "class ",
                    "if __name__",
                    "pip ",
                    "requirements.txt",
                ],
            ),
            (
                "javascript",
                vec!["function ", "const ", "let ", "var ", "=>", "require(", "import ", "export "],
            ),
            (
                "typescript",
                vec!["interface ", "type ", ": string", ": number", ": boolean", "tsx", "ts"],
            ),
            (
                "java",
                vec![
                    "public class",
                    "private ",
                    "protected ",
                    "static void",
                    "package ",
                    "import java",
                ],
            ),
            (
                "go",
                vec![
                    "func ",
                    "package main",
                    "import (",
                    "var ",
                    "type ",
                    "struct {",
                    "interface{}",
                ],
            ),
            ("c", vec!["#include", "int main", "void ", "printf", "malloc", "typedef"]),
            ("cpp", vec!["#include", "std::", "template", "namespace", "cout", "class "]),
        ];

        let mut best_match = None;
        let mut best_score = 0;

        for (lang, patterns) in &indicators {
            let mut score = 0;
            for pattern in patterns {
                if content.contains(pattern) {
                    score += 1;
                }
            }
            if score > best_score {
                best_score = score;
                best_match = Some(lang.to_string());
            }
        }

        best_match
    }

    /// Classify question type using patterns
    pub fn classify_question(&self, question: &str) -> QuestionType {
        let lower = question.to_lowercase();

        if lower.starts_with("what is")
            || lower.starts_with("what are")
            || lower.starts_with("define")
            || lower.starts_with("explain what")
        {
            QuestionType::Definition
        } else if lower.starts_with("is ")
            || lower.starts_with("are ")
            || lower.starts_with("does ")
            || lower.starts_with("can ")
            || lower.starts_with("should ")
            || lower.starts_with("will ")
        {
            QuestionType::YesNo
        } else if lower.contains("what type")
            || lower.contains("what kind")
            || lower.contains("classify")
            || lower.contains("categorize")
        {
            QuestionType::Classification
        } else if lower.len() < 50 && !lower.contains("how") && !lower.contains("why") {
            QuestionType::Simple
        } else {
            QuestionType::Complex
        }
    }
}

impl Default for HeuristicClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_code_detection() {
        let classifier = HeuristicClassifier::new();

        let rust_code = "fn main() {\n    println!(\"Hello\");\n}";
        assert!(classifier.is_code(rust_code));

        let markdown = "# Title\n\nThis is documentation.";
        assert!(!classifier.is_code(markdown));

        let mixed = "Here's an example:\n\n```rust\nfn test() {}\n```";
        // Mixed content may or may not be detected as code
        let _ = classifier.is_code(mixed);
    }

    #[test]
    fn test_language_detection() {
        let classifier = HeuristicClassifier::new();

        let rust = "fn main() { let x = 5; }";
        let result = classifier.classify_content(rust);
        assert_eq!(result.language, Some("rust".to_string()));

        let python = "def main():\n    import os\n    return True";
        let result = classifier.classify_content(python);
        assert_eq!(result.language, Some("python".to_string()));
    }

    #[test]
    fn test_question_classification() {
        let classifier = HeuristicClassifier::new();

        assert_eq!(classifier.classify_question("What is a socket?"), QuestionType::Definition);

        assert_eq!(classifier.classify_question("Is this code valid?"), QuestionType::YesNo);

        assert_eq!(
            classifier.classify_question("What type of file is this?"),
            QuestionType::Classification
        );
    }
}
