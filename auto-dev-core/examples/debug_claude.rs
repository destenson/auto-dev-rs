use std::process::Stdio;
use tokio::process::Command;

#[tokio::main]
async fn main() {
    println!("Debugging Claude CLI detection...\n");
    
    // Test 1: Basic command
    println!("Test 1: claude --version");
    match Command::new("claude")
        .arg("--version")
        .output()
        .await
    {
        Ok(output) => {
            println!("  Exit status: {}", output.status);
            println!("  Stdout: {}", String::from_utf8_lossy(&output.stdout));
            println!("  Stderr: {}", String::from_utf8_lossy(&output.stderr));
        }
        Err(e) => println!("  Error: {}", e),
    }
    
    // Test 2: Without argument
    println!("\nTest 2: claude (no args, with timeout)");
    match tokio::time::timeout(
        std::time::Duration::from_secs(1),
        Command::new("claude")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    ).await {
        Ok(Ok(output)) => {
            println!("  Exit status: {}", output.status);
            println!("  Stdout length: {} bytes", output.stdout.len());
            println!("  Stderr length: {} bytes", output.stderr.len());
        }
        Ok(Err(e)) => println!("  Command error: {}", e),
        Err(_) => println!("  Timed out (probably waiting for input)"),
    }
    
    // Test 3: Help
    println!("\nTest 3: claude --help");
    match Command::new("claude")
        .arg("--help")
        .output()
        .await
    {
        Ok(output) => {
            println!("  Exit status: {}", output.status);
            println!("  Has output: {}", !output.stdout.is_empty() || !output.stderr.is_empty());
        }
        Err(e) => println!("  Error: {}", e),
    }
}