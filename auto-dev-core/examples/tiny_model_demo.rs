//! Demo of using tiny models for simple classification tasks

use auto_dev_core::llm::{TinyModel, candle::SmartTinyModel, classifier::HeuristicClassifier};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🤖 Tiny Model Demo\n");

    // Create a smart model that uses heuristics (no model file needed)
    let model = SmartTinyModel::new(None);

    if model.is_using_heuristics() {
        println!("Using heuristic classifier (no model loaded)\n");
    } else {
        println!("Using loaded model\n");
    }

    // Test code detection
    println!("=== Code Detection ===");
    let samples = [
        ("fn main() { println!(\"hello\"); }", "Rust code"),
        ("def hello(): return 'world'", "Python code"),
        ("This is a README file with documentation.", "Documentation"),
        ("SELECT * FROM users WHERE id = 1", "SQL query"),
    ];

    for (content, description) in samples {
        let is_code = model.is_code(content).await?;
        println!("  {} -> is_code: {}", description, is_code);
    }

    // Test content classification
    println!("\n=== Content Classification ===");
    let rust_test = r#"
#[test]
fn test_addition() {
    assert_eq!(2 + 2, 4);
}
"#;

    let classification = model.classify_content(rust_test).await?;
    println!("  Test code classification:");
    println!("    is_code: {}", classification.is_code);
    println!("    is_test: {}", classification.is_test);
    println!("    language: {:?}", classification.language);

    // Test question classification
    println!("\n=== Question Classification ===");
    let questions = [
        "What is a socket?",
        "Is this code valid?",
        "What type of file is this?",
        "How do I implement a REST API?",
    ];

    for question in questions {
        let q_type = model.classify_question(question).await?;
        println!("  \"{}\" -> {:?}", question, q_type);
    }

    // Test simple Q&A
    println!("\n=== Simple Q&A ===");
    let qa_questions = [
        "What is a socket?",
        "What is a function?",
        "What is an API?",
        "How do I build a compiler?", // Complex - won't answer
    ];

    for question in qa_questions {
        match model.simple_answer(question).await? {
            Some(answer) => {
                println!("  Q: {}", question);
                println!("  A: {}", answer);
            }
            None => {
                println!("  Q: {}", question);
                println!("  A: [Too complex for tiny model]");
            }
        }
    }

    // Test requirement checking
    println!("\n=== Requirement Checking ===");
    let requirement = "Function must validate email addresses";
    let code_samples = [
        ("fn validate_email(email: &str) -> bool { email.contains('@') }", true),
        ("fn process_data(data: &[u8]) { /* ... */ }", false),
    ];

    for (code, expected) in code_samples {
        let satisfied = model.check_requirement(requirement, code).await?;
        println!("  Requirement: \"{}\"", requirement);
        println!("  Code: {}", &code[..40.min(code.len())]);
        println!("  Satisfied: {} (expected: {})", satisfied, expected);
        println!();
    }

    // Show heuristic classifier directly
    println!("=== Direct Heuristic Classifier ===");
    let classifier = HeuristicClassifier::new();

    let test_content = "import React from 'react';\nconst App = () => <div>Hello</div>;";
    let result = classifier.classify_content(test_content);
    println!("  Content: React component");
    println!("  Detected language: {:?}", result.language);
    println!("  Is code: {}", result.is_code);

    Ok(())
}
