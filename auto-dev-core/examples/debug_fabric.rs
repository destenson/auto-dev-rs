use std::process::Stdio;
use tokio::process::Command;

#[tokio::main]
async fn main() {
    println!("Debugging Fabric output...\n");
    
    if let Ok(output) = Command::new("fabric")
        .arg("--listmodels")
        .stdin(Stdio::null())
        .output()
        .await
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        println!("Raw output (first 20 lines):");
        for (i, line) in stdout.lines().take(20).enumerate() {
            println!("Line {}: {:?}", i, line);
        }
        
        println!("\nLines starting with tab:");
        for line in stdout.lines() {
            if line.starts_with('\t') {
                println!("Found: {:?}", line);
                
                // Try to parse
                if line.contains('[') && line.contains(']') {
                    if let Some(start) = line.find(']') {
                        let model_part = &line[start + 1..];
                        println!("  After ]: {:?}", model_part.trim());
                    }
                }
            }
        }
    }
}