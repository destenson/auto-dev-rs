//! Tool execution for project initialization

use super::detector::ProjectType;
use super::instructions::InstructionDocument;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;
use tracing::{info, warn};

pub struct ToolExecutor;

impl ToolExecutor {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn execute(
        &self,
        project_type: ProjectType,
        output_dir: &Path,
        instructions: &InstructionDocument,
    ) -> Result<()> {
        // Check if directory exists and has content
        let existing_project = self.detect_existing_project(output_dir).await;
        
        if let Some(detected_type) = existing_project {
            info!("Found existing {:?} project at {:?}", detected_type, output_dir);
            
            // If types match, we're good - just add auto-dev config
            if detected_type == project_type {
                println!("✓ Existing {:?} project detected, adding auto-dev configuration", project_type);
                return Ok(());
            } else {
                // Warn about mismatch but continue
                warn!("Project type mismatch: detected {:?} but instructions suggest {:?}", 
                      detected_type, project_type);
                println!("⚠️  Warning: Existing project appears to be {:?} but instructions suggest {:?}", 
                         detected_type, project_type);
                println!("   Proceeding with auto-dev configuration only...");
                return Ok(());
            }
        }
        
        // Create output directory if it doesn't exist
        if !output_dir.exists() {
            tokio::fs::create_dir_all(output_dir).await
                .context("Failed to create output directory")?;
        }
        
        match project_type {
            ProjectType::Rust => self.init_rust(output_dir, instructions).await,
            ProjectType::Python => self.init_python(output_dir, instructions).await,
            ProjectType::JavaScript | ProjectType::TypeScript => {
                self.init_node(output_dir, instructions, project_type == ProjectType::TypeScript).await
            },
            ProjectType::Deno => self.init_deno(output_dir, instructions).await,
            ProjectType::DotNet => self.init_dotnet(output_dir, instructions).await,
            ProjectType::Go => self.init_go(output_dir, instructions).await,
            ProjectType::Java => self.init_java(output_dir, instructions).await,
            ProjectType::Generic => self.init_generic(output_dir, instructions).await,
        }
    }
    
    async fn init_rust(&self, dir: &Path, instructions: &InstructionDocument) -> Result<()> {
        info!("Initializing Rust project with cargo");
        
        // Determine if it's a library or binary
        let is_lib = instructions.raw_content.to_lowercase().contains("library") ||
                     instructions.raw_content.to_lowercase().contains("crate");
        
        let mut cmd = Command::new("cargo");
        cmd.arg("init")
           .arg("--name")
           .arg(instructions.metadata.project_name.as_deref().unwrap_or("my_project"));
        
        if is_lib {
            cmd.arg("--lib");
        }
        
        cmd.arg(dir);
        
        let output = cmd.output()
            .context("Failed to run cargo init. Is Cargo installed?")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("already exists") {
                return Err(anyhow::anyhow!("cargo init failed: {}", stderr));
            }
        }
        
        Ok(())
    }
    
    async fn init_python(&self, dir: &Path, instructions: &InstructionDocument) -> Result<()> {
        // Try uv first, then pip
        if self.check_command_exists("uv").await {
            info!("Initializing Python project with uv");
            
            let output = Command::new("uv")
                .arg("init")
                .arg(dir)
                .output()
                .context("Failed to run uv init")?;
            
            if output.status.success() {
                return Ok(());
            }
        }
        
        // Fallback to basic Python setup
        info!("Creating basic Python project structure");
        
        // Create main.py
        let main_py = dir.join("main.py");
        let content = format!(
            "#!/usr/bin/env python3\n\
             \"\"\"{}.\"\"\"\n\n\
             def main():\n    \
                 print(\"Hello from {}!\")\n\n\
             if __name__ == \"__main__\":\n    \
                 main()\n",
            instructions.metadata.project_name.as_deref().unwrap_or("Project"),
            instructions.metadata.project_name.as_deref().unwrap_or("project")
        );
        tokio::fs::write(&main_py, content).await?;
        
        // Create requirements.txt
        let requirements = dir.join("requirements.txt");
        tokio::fs::write(&requirements, "# Add your dependencies here\n").await?;
        
        Ok(())
    }
    
    async fn init_node(&self, dir: &Path, instructions: &InstructionDocument, typescript: bool) -> Result<()> {
        info!("Initializing {} project with npm", if typescript { "TypeScript" } else { "JavaScript" });
        
        // Run npm init -y
        let output = Command::new("npm")
            .arg("init")
            .arg("-y")
            .current_dir(dir)
            .output()
            .context("Failed to run npm init. Is npm installed?")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("npm init failed: {}", stderr));
        }
        
        // For TypeScript, add tsconfig.json
        if typescript {
            let tsconfig = dir.join("tsconfig.json");
            let tsconfig_content = r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "lib": ["ES2020"],
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}"#;
            tokio::fs::write(&tsconfig, tsconfig_content).await?;
            
            // Create src directory
            let src_dir = dir.join("src");
            tokio::fs::create_dir_all(&src_dir).await?;
            
            // Create index.ts
            let index = src_dir.join("index.ts");
            let content = format!(
                "// {}\n\
                 console.log('Hello from {}!');\n\
                 export {{}};",
                instructions.metadata.project_name.as_deref().unwrap_or("Project"),
                instructions.metadata.project_name.as_deref().unwrap_or("project")
            );
            tokio::fs::write(&index, content).await?;
        } else {
            // Create index.js for JavaScript
            let index = dir.join("index.js");
            let content = format!(
                "// {}\n\
                 console.log('Hello from {}!');\n\
                 module.exports = {{}};",
                instructions.metadata.project_name.as_deref().unwrap_or("Project"),
                instructions.metadata.project_name.as_deref().unwrap_or("project")
            );
            tokio::fs::write(&index, content).await?;
        }
        
        Ok(())
    }
    
    async fn init_deno(&self, dir: &Path, instructions: &InstructionDocument) -> Result<()> {
        if self.check_command_exists("deno").await {
            info!("Initializing Deno project");
            
            let output = Command::new("deno")
                .arg("init")
                .current_dir(dir)
                .output()
                .context("Failed to run deno init")?;
            
            if output.status.success() {
                return Ok(());
            }
        }
        
        // Fallback: create basic Deno structure
        info!("Creating basic Deno project structure");
        
        let main = dir.join("main.ts");
        let content = format!(
            "// {}\n\
             console.log(\"Hello from {}!\");\n\
             export {{}};",
            instructions.metadata.project_name.as_deref().unwrap_or("Deno Project"),
            instructions.metadata.project_name.as_deref().unwrap_or("deno")
        );
        tokio::fs::write(&main, content).await?;
        
        // Create deno.json
        let deno_json = dir.join("deno.json");
        let config = r#"{
  "tasks": {
    "dev": "deno run --watch main.ts",
    "test": "deno test"
  }
}"#;
        tokio::fs::write(&deno_json, config).await?;
        
        Ok(())
    }
    
    async fn init_dotnet(&self, dir: &Path, instructions: &InstructionDocument) -> Result<()> {
        if self.check_command_exists("dotnet").await {
            info!("Initializing .NET project");
            
            // Determine project template
            let template = if instructions.raw_content.to_lowercase().contains("web") ||
                             instructions.raw_content.to_lowercase().contains("api") {
                "webapi"
            } else {
                "console"
            };
            
            let output = Command::new("dotnet")
                .arg("new")
                .arg(template)
                .arg("-o")
                .arg(dir)
                .output()
                .context("Failed to run dotnet new")?;
            
            if output.status.success() {
                return Ok(());
            }
        }
        
        // Fallback for .NET is just generic
        self.init_generic(dir, instructions).await
    }
    
    async fn init_go(&self, dir: &Path, instructions: &InstructionDocument) -> Result<()> {
        info!("Initializing Go project");
        
        // Create go.mod
        let module_name = instructions.metadata.project_name
            .as_deref()
            .unwrap_or("myproject");
        
        let go_mod = dir.join("go.mod");
        let content = format!("module {}\n\ngo 1.21\n", module_name);
        tokio::fs::write(&go_mod, content).await?;
        
        // Create main.go
        let main_go = dir.join("main.go");
        let main_content = format!(
            "package main\n\n\
             import \"fmt\"\n\n\
             func main() {{\n\t\
                 fmt.Println(\"Hello from {}!\")\n\
             }}\n",
            module_name
        );
        tokio::fs::write(&main_go, main_content).await?;
        
        Ok(())
    }
    
    async fn init_java(&self, dir: &Path, instructions: &InstructionDocument) -> Result<()> {
        info!("Initializing Java project");
        
        // Check for Maven
        if self.check_command_exists("mvn").await {
            warn!("Maven detected but not implemented yet, using generic init");
        }
        
        // Create basic Java structure
        let src_dir = dir.join("src").join("main").join("java");
        tokio::fs::create_dir_all(&src_dir).await?;
        
        let main_java = src_dir.join("Main.java");
        let content = format!(
            "public class Main {{\n    \
                 public static void main(String[] args) {{\n        \
                     System.out.println(\"Hello from {}!\");\n    \
                 }}\n\
             }}\n",
            instructions.metadata.project_name.as_deref().unwrap_or("Java Project")
        );
        tokio::fs::write(&main_java, content).await?;
        
        Ok(())
    }
    
    async fn init_generic(&self, dir: &Path, instructions: &InstructionDocument) -> Result<()> {
        info!("Creating generic project structure");
        
        // Create README
        let readme = dir.join("README.md");
        let readme_content = format!(
            "# {}\n\n\
             ## Description\n\
             {}\n\n\
             ## Instructions\n\
             This project was initialized by auto-dev. \
             The original instructions are saved in `.auto-dev/instructions.md`.\n",
            instructions.metadata.project_name.as_deref().unwrap_or("New Project"),
            instructions.metadata.description.as_deref().unwrap_or("A new project created with auto-dev")
        );
        tokio::fs::write(&readme, readme_content).await?;
        
        // Create src directory
        let src_dir = dir.join("src");
        tokio::fs::create_dir_all(&src_dir).await?;
        
        Ok(())
    }
    
    async fn check_command_exists(&self, command: &str) -> bool {
        Command::new(command)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    async fn detect_existing_project(&self, dir: &Path) -> Option<ProjectType> {
        if !dir.exists() {
            return None;
        }
        
        // Check for various project markers
        if dir.join("Cargo.toml").exists() {
            return Some(ProjectType::Rust);
        }
        
        if dir.join("pyproject.toml").exists() || 
           dir.join("setup.py").exists() || 
           dir.join("requirements.txt").exists() {
            return Some(ProjectType::Python);
        }
        
        if dir.join("package.json").exists() {
            // Check if it's TypeScript
            if dir.join("tsconfig.json").exists() {
                return Some(ProjectType::TypeScript);
            }
            return Some(ProjectType::JavaScript);
        }
        
        if dir.join("deno.json").exists() || dir.join("deno.jsonc").exists() {
            return Some(ProjectType::Deno);
        }
        
        if dir.join("go.mod").exists() {
            return Some(ProjectType::Go);
        }
        
        if dir.join("pom.xml").exists() || dir.join("build.gradle").exists() {
            return Some(ProjectType::Java);
        }
        
        // Check for .NET project files
        let csproj_exists = std::fs::read_dir(dir)
            .map(|entries| {
                entries.filter_map(Result::ok)
                    .any(|entry| {
                        entry.path().extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| ext == "csproj" || ext == "fsproj" || ext == "vbproj")
                            .unwrap_or(false)
                    })
            })
            .unwrap_or(false);
            
        if csproj_exists || dir.join("project.json").exists() {
            return Some(ProjectType::DotNet);
        }
        
        // No specific project type detected
        None
    }
}