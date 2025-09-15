use anyhow::Result;
use auto_dev_core::llm::{
    candle::TinyModel,
    classifier::TaskClassifier,
    config::{ModelConfig, QwenConfig},
    provider::{LLMProvider, ModelTier},
};
use std::path::PathBuf;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    info!("Starting Qwen Model Validation Tests");

    let model_path = PathBuf::from("models/qwen2.5-coder-0.5b-instruct-q4_k_m.gguf");

    if !model_path.exists() {
        error!("Model file not found at: {:?}", model_path);
        error!("Please run scripts/download_model.ps1 first");
        return Ok(());
    }

    info!(
        "Model file found: {:?} ({:.2} MB)",
        model_path,
        std::fs::metadata(&model_path)?.len() as f64 / (1024.0 * 1024.0)
    );

    run_validation_tests().await?;

    Ok(())
}

async fn run_validation_tests() -> Result<()> {
    info!("\n=== Test 1: Task Classification ===");
    test_task_classification().await?;

    info!("\n=== Test 2: Code Analysis ===");
    test_code_analysis().await?;

    info!("\n=== Test 3: Simple Queries ===");
    test_simple_queries().await?;

    info!("\n=== Test 4: Performance Benchmarks ===");
    test_performance().await?;

    info!("\n=== Test 5: Edge Cases ===");
    test_edge_cases().await?;

    Ok(())
}

async fn test_task_classification() -> Result<()> {
    let classifier = TaskClassifier::new();

    let test_cases = vec![
        ("Write a function to calculate fibonacci", "code_generation"),
        ("What is the purpose of this function?", "code_explanation"),
        ("Fix the bug in this code", "bug_fix"),
        ("Add tests for the User model", "test_generation"),
        ("Refactor this to use async/await", "refactoring"),
        ("Is this code secure?", "security_review"),
        ("What does HTTP stand for?", "question_answering"),
    ];

    for (input, expected_category) in test_cases {
        let result = classifier.classify(input).await?;
        info!("Input: \"{}\"", input);
        info!("  Classification: {:?}", result.task_type);
        info!("  Complexity: {:?}", result.complexity);
        info!("  Expected: {}", expected_category);
        info!(
            "  Match: {}",
            if format!("{:?}", result.task_type).to_lowercase().contains(expected_category) {
                "✓"
            } else {
                "✗"
            }
        );
    }

    Ok(())
}

async fn test_code_analysis() -> Result<()> {
    let classifier = TaskClassifier::new();

    let code_samples = vec![
        ("fn add(a: i32, b: i32) -> i32 { a + b }", "Rust", "Simple addition function"),
        (
            "def factorial(n):\n    if n <= 1:\n        return 1\n    return n * factorial(n-1)",
            "Python",
            "Recursive factorial",
        ),
        (
            "const sum = (arr) => arr.reduce((a, b) => a + b, 0);",
            "JavaScript",
            "Array sum using reduce",
        ),
        ("SELECT * FROM users WHERE age > 18", "SQL", "Query for adult users"),
    ];

    for (code, expected_lang, description) in code_samples {
        info!("Analyzing: {}", description);
        info!("  Code: {}", code.replace('\n', " "));

        let is_code = classifier.is_code(code);
        let language = classifier.detect_language(code);

        info!("  Is Code: {}", is_code);
        info!("  Detected Language: {:?}", language);
        info!("  Expected Language: {}", expected_lang);
        info!("  Match: {}", if language.as_deref() == Some(expected_lang) { "✓" } else { "✗" });
    }

    Ok(())
}

async fn test_simple_queries() -> Result<()> {
    let test_queries = vec![
        "What is 2 + 2?",
        "How do I create a vector in Rust?",
        "Explain async/await",
        "What's the difference between let and const?",
        "How to handle errors in Rust?",
    ];

    info!("Testing simple query classification:");
    let classifier = TaskClassifier::new();

    for query in test_queries {
        let result = classifier.classify(query).await?;
        info!("Query: \"{}\"", query);
        info!("  Type: {:?}", result.task_type);
        info!("  Complexity: {:?}", result.complexity);
        info!("  Requires Code: {}", result.requires_code_generation);
    }

    Ok(())
}

async fn test_performance() -> Result<()> {
    use std::time::Instant;

    let classifier = TaskClassifier::new();
    let test_input = "Write a function to sort an array";

    info!("Running performance benchmarks...");

    let mut total_time = std::time::Duration::ZERO;
    let iterations = 10;

    for i in 1..=iterations {
        let start = Instant::now();
        let _ = classifier.classify(test_input).await?;
        let elapsed = start.elapsed();
        total_time += elapsed;
        info!("  Iteration {}: {:?}", i, elapsed);
    }

    let avg_time = total_time / iterations;
    info!("Average classification time: {:?}", avg_time);

    if avg_time.as_millis() < 100 {
        info!("  Performance: ✓ Excellent (<100ms)");
    } else if avg_time.as_millis() < 500 {
        info!("  Performance: ✓ Good (<500ms)");
    } else {
        warn!("  Performance: ⚠ Slow (>500ms)");
    }

    Ok(())
}

async fn test_edge_cases() -> Result<()> {
    let classifier = TaskClassifier::new();

    let edge_cases = vec![
        ("", "Empty input"),
        ("     ", "Whitespace only"),
        ("🚀🦀💻", "Emojis only"),
        ("a".repeat(1000).as_str(), "Very long input"),
        ("fn()", "Minimal code"),
        ("// TODO: implement this", "Comment only"),
        ("<script>alert('xss')</script>", "Potential security test"),
        ("SELECT * FROM users; DROP TABLE users;", "SQL injection attempt"),
    ];

    info!("Testing edge cases:");

    for (input, description) in edge_cases {
        info!("Testing: {}", description);

        match classifier.classify(input).await {
            Ok(result) => {
                info!("  ✓ Handled successfully");
                info!("    Type: {:?}", result.task_type);
                info!("    Complexity: {:?}", result.complexity);
            }
            Err(e) => {
                warn!("  ✗ Error: {}", e);
            }
        }
    }

    Ok(())
}
