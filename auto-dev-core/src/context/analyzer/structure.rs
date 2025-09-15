use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectStructure {
    pub root: PathBuf,
    pub directories: Vec<DirectoryInfo>,
    pub files: Vec<FileInfo>,
    pub entry_points: Vec<PathBuf>,
    pub test_directories: Vec<PathBuf>,
    pub documentation_files: Vec<PathBuf>,
    pub configuration_files: Vec<PathBuf>,
    pub statistics: ProjectStatistics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryInfo {
    pub path: PathBuf,
    pub name: String,
    pub purpose: DirectoryPurpose,
    pub file_count: usize,
    pub total_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DirectoryPurpose {
    Source,
    Tests,
    Documentation,
    Configuration,
    Build,
    Assets,
    Dependencies,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: PathBuf,
    pub name: String,
    pub extension: Option<String>,
    pub size: u64,
    pub file_type: FileType,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileType {
    Source,
    Test,
    Documentation,
    Configuration,
    Build,
    Asset,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectStatistics {
    pub total_files: usize,
    pub total_directories: usize,
    pub total_size: u64,
    pub lines_of_code: usize,
    pub language_distribution: HashMap<String, usize>,
    pub file_type_distribution: HashMap<String, usize>,
}

pub async fn analyze_project_structure(project_root: &Path) -> anyhow::Result<ProjectStructure> {
    let mut structure = ProjectStructure { root: project_root.to_path_buf(), ..Default::default() };

    let mut language_stats: HashMap<String, usize> = HashMap::new();
    let mut file_type_stats: HashMap<String, usize> = HashMap::new();

    // Walk through the project directory
    for entry in WalkDir::new(project_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_ignored(e.path()))
    {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            // Analyze directory
            let dir_info = analyze_directory(path, project_root).await?;
            structure.directories.push(dir_info);
            structure.statistics.total_directories += 1;
        } else if metadata.is_file() {
            // Analyze file
            let file_info = analyze_file(path, metadata.len()).await?;

            // Update statistics
            if let Some(ref lang) = file_info.language {
                *language_stats.entry(lang.clone()).or_insert(0) += 1;
            }

            let file_type_name = format!("{:?}", file_info.file_type);
            *file_type_stats.entry(file_type_name).or_insert(0) += 1;

            structure.statistics.total_size += file_info.size;

            // Categorize special files
            match file_info.file_type {
                FileType::Test => {
                    if let Some(parent) = path.parent() {
                        if !structure.test_directories.contains(&parent.to_path_buf()) {
                            structure.test_directories.push(parent.to_path_buf());
                        }
                    }
                }
                FileType::Documentation => {
                    structure.documentation_files.push(path.to_path_buf());
                }
                FileType::Configuration => {
                    structure.configuration_files.push(path.to_path_buf());
                }
                _ => {}
            }

            // Check for entry points
            if is_entry_point(path) {
                structure.entry_points.push(path.to_path_buf());
            }

            structure.files.push(file_info);
            structure.statistics.total_files += 1;
        }
    }

    structure.statistics.language_distribution = language_stats;
    structure.statistics.file_type_distribution = file_type_stats;

    // Calculate lines of code (simplified)
    structure.statistics.lines_of_code = calculate_lines_of_code(&structure.files).await?;

    Ok(structure)
}

async fn analyze_directory(path: &Path, project_root: &Path) -> anyhow::Result<DirectoryInfo> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();

    let purpose = determine_directory_purpose(&name, path, project_root);

    let (file_count, total_size) = count_directory_contents(path).await?;

    Ok(DirectoryInfo { path: path.to_path_buf(), name, purpose, file_count, total_size })
}

async fn analyze_file(path: &Path, size: u64) -> anyhow::Result<FileInfo> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();

    let extension = path.extension().and_then(|e| e.to_str()).map(|e| e.to_string());

    let language = detect_language(&extension);
    let file_type = determine_file_type(path, &name, &extension);

    Ok(FileInfo { path: path.to_path_buf(), name, extension, size, file_type, language })
}

fn determine_directory_purpose(name: &str, path: &Path, project_root: &Path) -> DirectoryPurpose {
    let lower_name = name.to_lowercase();

    if lower_name.contains("test")
        || lower_name == "tests"
        || lower_name == "spec"
        || lower_name == "specs"
    {
        DirectoryPurpose::Tests
    } else if lower_name == "src" || lower_name == "source" || lower_name == "lib" {
        DirectoryPurpose::Source
    } else if lower_name == "docs" || lower_name == "documentation" || lower_name == "doc" {
        DirectoryPurpose::Documentation
    } else if lower_name == "config" || lower_name == "configs" || lower_name == ".config" {
        DirectoryPurpose::Configuration
    } else if lower_name == "build"
        || lower_name == "dist"
        || lower_name == "target"
        || lower_name == "out"
    {
        DirectoryPurpose::Build
    } else if lower_name == "assets"
        || lower_name == "static"
        || lower_name == "public"
        || lower_name == "resources"
    {
        DirectoryPurpose::Assets
    } else if lower_name == "node_modules" || lower_name == "vendor" || lower_name == "packages" {
        DirectoryPurpose::Dependencies
    } else {
        DirectoryPurpose::Unknown
    }
}

fn determine_file_type(path: &Path, name: &str, extension: &Option<String>) -> FileType {
    let lower_name = name.to_lowercase();

    // Check if it's a test file
    if lower_name.contains("test")
        || lower_name.contains("spec")
        || path.components().any(|c| c.as_os_str() == "tests" || c.as_os_str() == "test")
    {
        return FileType::Test;
    }

    // Check by extension
    if let Some(ext) = extension {
        match ext.as_str() {
            "md" | "rst" | "txt" | "adoc" => return FileType::Documentation,
            "toml" | "yaml" | "yml" | "json" | "ini" | "cfg" | "conf" => {
                return FileType::Configuration;
            }
            "rs" | "py" | "js" | "ts" | "go" | "java" | "c" | "cpp" | "cs" => {
                return FileType::Source;
            }
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" => return FileType::Asset,
            _ => {}
        }
    }

    // Check by name patterns
    if name == "Makefile" || name == "CMakeLists.txt" || name.ends_with(".gradle") {
        return FileType::Build;
    }

    if name == "README.md" || name == "LICENSE" || name == "CHANGELOG.md" {
        return FileType::Documentation;
    }

    if name == "Cargo.toml"
        || name == "package.json"
        || name == "pom.xml"
        || name == "go.mod"
        || name == "requirements.txt"
    {
        return FileType::Configuration;
    }

    FileType::Other
}

fn detect_language(extension: &Option<String>) -> Option<String> {
    extension.as_ref().and_then(|ext| match ext.as_str() {
        "rs" => Some("Rust".to_string()),
        "py" => Some("Python".to_string()),
        "js" | "mjs" => Some("JavaScript".to_string()),
        "ts" => Some("TypeScript".to_string()),
        "go" => Some("Go".to_string()),
        "java" => Some("Java".to_string()),
        "c" => Some("C".to_string()),
        "cpp" | "cc" | "cxx" => Some("C++".to_string()),
        "cs" => Some("C#".to_string()),
        "rb" => Some("Ruby".to_string()),
        "php" => Some("PHP".to_string()),
        "swift" => Some("Swift".to_string()),
        "kt" => Some("Kotlin".to_string()),
        "scala" => Some("Scala".to_string()),
        "r" => Some("R".to_string()),
        "sh" | "bash" => Some("Shell".to_string()),
        "sql" => Some("SQL".to_string()),
        "html" => Some("HTML".to_string()),
        "css" | "scss" | "sass" => Some("CSS".to_string()),
        "xml" => Some("XML".to_string()),
        "json" => Some("JSON".to_string()),
        "yaml" | "yml" => Some("YAML".to_string()),
        "toml" => Some("TOML".to_string()),
        _ => None,
    })
}

fn is_entry_point(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        name == "main.rs"
            || name == "lib.rs"
            || name == "mod.rs"
            || name == "main.py"
            || name == "__main__.py"
            || name == "__init__.py"
            || name == "index.js"
            || name == "index.ts"
            || name == "app.js"
            || name == "app.ts"
            || name == "main.go"
            || name == "main.java"
            || name == "Main.java"
            || name == "main.c"
            || name == "main.cpp"
            || name == "Program.cs"
    } else {
        false
    }
}

fn is_ignored(path: &Path) -> bool {
    path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        name.starts_with('.') && name != "." && name != ".."
            || name == "node_modules"
            || name == "target"
            || name == "dist"
            || name == "__pycache__"
            || name == ".git"
            || name == ".svn"
            || name == ".hg"
            || name == "vendor"
    })
}

async fn count_directory_contents(path: &Path) -> anyhow::Result<(usize, u64)> {
    let mut file_count = 0;
    let mut total_size = 0;

    if let Ok(mut entries) = fs::read_dir(path).await {
        while let Some(entry) = entries.next_entry().await? {
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_file() {
                    file_count += 1;
                    total_size += metadata.len();
                }
            }
        }
    }

    Ok((file_count, total_size))
}

async fn calculate_lines_of_code(files: &[FileInfo]) -> anyhow::Result<usize> {
    let mut total_lines = 0;

    for file in files {
        if matches!(file.file_type, FileType::Source | FileType::Test) {
            if let Ok(content) = fs::read_to_string(&file.path).await {
                total_lines += content.lines().count();
            }
        }
    }

    Ok(total_lines)
}
