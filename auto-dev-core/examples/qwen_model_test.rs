use anyhow::{Result, Context};
use std::path::PathBuf;
use tracing::{info, warn, error};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    info!("=== Qwen 2.5 Coder Model Test Suite ===");
    
    let model_path = PathBuf::from("models/qwen2.5-coder-0.5b-instruct-q4_k_m.gguf");
    
    if !model_path.exists() {
        error!("Model file not found at: {:?}", model_path);
        error!("Please run: .\\scripts\\download_model.ps1");
        return Ok(());
    }

    let file_size = std::fs::metadata(&model_path)?.len() as f64 / (1024.0 * 1024.0);
    info!("Model file found: {:?} ({:.2} MB)", model_path, file_size);
    
    info!("\n=== Testing Model with Candle ===");
    test_candle_loading(&model_path)?;
    
    info!("\n=== Testing Heuristic Classifier (Fallback) ===");
    test_heuristic_classifier().await?;
    
    info!("\n=== Testing Use Cases ===");
    test_use_cases().await?;
    
    info!("\n=== Model Validation Complete ===");
    Ok(())
}

fn test_candle_loading(model_path: &PathBuf) -> Result<()> {
    info!("Attempting to load model with Candle...");
    
    info!("Note: Candle GGUF support is limited. The model may need conversion or alternative loading method.");
    info!("For now, we'll use the heuristic classifier as a fallback.");
    
    Ok(())
}

async fn test_heuristic_classifier() -> Result<()> {
    use auto_dev_core::llm::classifier::TaskClassifier;
    
    let classifier = TaskClassifier::new();
    
    info!("Testing heuristic classifier (no model required):");
    
    let test_cases = vec![
        ("Write a function to sort an array", "Code Generation"),
        ("What does this function do?", "Code Explanation"),
        ("Fix the bug in line 42", "Bug Fix"),
        ("Add unit tests for the User model", "Test Generation"),
        ("Refactor this to use async/await", "Refactoring"),
        ("Review this code for security issues", "Security Review"),
        ("What is a monad?", "Question Answering"),
        ("Convert this Python code to Rust", "Code Translation"),
    ];
    
    for (input, expected) in test_cases {
        let result = classifier.classify(input).await?;
        info!("\nInput: \"{}\"", input);
        info!("  Expected: {}", expected);
        info!("  Task Type: {:?}", result.task_type);
        info!("  Complexity: {:?}", result.complexity);
        info!("  Requires Code Gen: {}", result.requires_code_generation);
        info!("  Confidence: {:.2}", result.confidence);
    }
    
    Ok(())
}

async fn test_use_cases() -> Result<()> {
    use auto_dev_core::llm::classifier::TaskClassifier;
    
    let classifier = TaskClassifier::new();
    
    info!("\n=== Use Case 1: Language Detection ===");
    
    let code_samples = vec![
        ("fn main() { println!(\"Hello\"); }", "Rust"),
        ("def hello():\n    print(\"Hello\")", "Python"),
        ("function hello() { console.log(\"Hello\"); }", "JavaScript"),
        ("public class Hello { public static void main(String[] args) {} }", "Java"),
        ("SELECT * FROM users WHERE id = 1", "SQL"),
        ("#include <stdio.h>\nint main() { return 0; }", "C"),
    ];
    
    for (code, expected_lang) in code_samples {
        let detected = classifier.detect_language(code);
        let is_match = detected.as_deref() == Some(expected_lang);
        info!("Language Detection:");
        info!("  Code: {}", code.replace('\n', " "));
        info!("  Expected: {}", expected_lang);
        info!("  Detected: {:?}", detected);
        info!("  Result: {}", if is_match { "✓ PASS" } else { "✗ FAIL" });
    }
    
    info!("\n=== Use Case 2: Code vs Non-Code Detection ===");
    
    let mixed_content = vec![
        ("let x = 42;", true, "Variable assignment"),
        ("The quick brown fox", false, "Plain text"),
        ("TODO: implement this later", false, "Comment"),
        ("if (x > 0) return true;", true, "Conditional"),
        ("Step 1: Install dependencies", false, "Documentation"),
        ("async function fetchData() {}", true, "Function declaration"),
    ];
    
    for (content, expected_is_code, description) in mixed_content {
        let is_code = classifier.is_code(content);
        let is_match = is_code == expected_is_code;
        info!("Code Detection - {}:", description);
        info!("  Content: \"{}\"", content);
        info!("  Expected: {}", if expected_is_code { "Code" } else { "Not Code" });
        info!("  Detected: {}", if is_code { "Code" } else { "Not Code" });
        info!("  Result: {}", if is_match { "✓ PASS" } else { "✗ FAIL" });
    }
    
    info!("\n=== Use Case 3: Complexity Assessment ===");
    
    let complexity_cases = vec![
        ("What is 2 + 2?", "Low", "Simple arithmetic"),
        ("Write a binary search tree implementation", "High", "Complex data structure"),
        ("Fix this typo", "Low", "Simple fix"),
        ("Implement a distributed consensus algorithm", "High", "Complex system"),
        ("Add a comment here", "Low", "Trivial change"),
        ("Refactor this monolithic service into microservices", "High", "Architecture change"),
    ];
    
    for (task, expected_complexity, description) in complexity_cases {
        let result = classifier.classify(task).await?;
        info!("Complexity Assessment - {}:", description);
        info!("  Task: \"{}\"", task);
        info!("  Expected: {}", expected_complexity);
        info!("  Detected: {:?}", result.complexity);
        
        let matches = match (expected_complexity, result.complexity) {
            ("Low", auto_dev_core::llm::provider::Complexity::Low) => true,
            ("Medium", auto_dev_core::llm::provider::Complexity::Medium) => true,
            ("High", auto_dev_core::llm::provider::Complexity::High) => true,
            _ => false,
        };
        
        info!("  Result: {}", if matches { "✓ PASS" } else { "✗ FAIL" });
    }
    
    info!("\n=== Use Case 4: Question Classification ===");
    
    let questions = vec![
        ("How do I iterate over a vector in Rust?", true, "How-to question"),
        ("What is the time complexity of quicksort?", false, "Conceptual question"),
        ("Can you write a function to validate email?", true, "Code request"),
        ("Why does Rust have ownership?", false, "Conceptual question"),
        ("Show me an example of async/await", true, "Example request"),
        ("What are the SOLID principles?", false, "Theory question"),
    ];
    
    for (question, expects_code, description) in questions {
        let result = classifier.classify(question).await?;
        let matches = result.requires_code_generation == expects_code;
        
        info!("Question Classification - {}:", description);
        info!("  Question: \"{}\"", question);
        info!("  Expects Code: {}", expects_code);
        info!("  Detected: {}", result.requires_code_generation);
        info!("  Task Type: {:?}", result.task_type);
        info!("  Result: {}", if matches { "✓ PASS" } else { "✗ FAIL" });
    }
    
    Ok(())
}