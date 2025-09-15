use std::path::Path;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use regex::Regex;
use tokio::fs;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodingConventions {
    pub naming: NamingConventions,
    pub formatting: FormattingRules,
    pub structure: StructureConventions,
    pub documentation: DocConventions,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NamingConventions {
    pub functions: NamingStyle,
    pub types: NamingStyle,
    pub constants: NamingStyle,
    pub files: NamingStyle,
    pub modules: NamingStyle,
    pub variables: NamingStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NamingStyle {
    SnakeCase,      // snake_case
    CamelCase,      // camelCase
    PascalCase,     // PascalCase
    UpperSnakeCase, // UPPER_SNAKE_CASE
    KebabCase,      // kebab-case
    Unknown,
}

impl Default for NamingStyle {
    fn default() -> Self {
        NamingStyle::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FormattingRules {
    pub indent_style: IndentStyle,
    pub indent_size: usize,
    pub line_width: usize,
    pub use_trailing_comma: bool,
    pub use_semicolons: bool,
    pub brace_style: BraceStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IndentStyle {
    Spaces,
    Tabs,
    Mixed,
}

impl Default for IndentStyle {
    fn default() -> Self {
        IndentStyle::Spaces
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BraceStyle {
    SameLine,     // { on same line
    NextLine,     // { on next line
    Allman,       // Allman style
    KAndR,        // K&R style
    Unknown,
}

impl Default for BraceStyle {
    fn default() -> Self {
        BraceStyle::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructureConventions {
    pub module_organization: ModuleOrganization,
    pub file_organization: FileOrganization,
    pub test_location: TestLocation,
    pub max_file_length: Option<usize>,
    pub max_function_length: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleOrganization {
    Flat,           // All modules in one directory
    Hierarchical,   // Nested module structure
    Feature,        // Organized by features
    Layer,          // Organized by layers
    Unknown,
}

impl Default for ModuleOrganization {
    fn default() -> Self {
        ModuleOrganization::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileOrganization {
    SingleClass,     // One class per file
    MultipleClasses, // Multiple classes per file
    Functional,      // Functional organization
    Mixed,
}

impl Default for FileOrganization {
    fn default() -> Self {
        FileOrganization::Mixed
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestLocation {
    SeparateDirectory, // tests/ directory
    SameFile,         // Tests in same file
    AdjacentFile,     // test_*.rs files
    Mixed,
}

impl Default for TestLocation {
    fn default() -> Self {
        TestLocation::Mixed
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocConventions {
    pub style: DocStyle,
    pub require_function_docs: bool,
    pub require_module_docs: bool,
    pub require_type_docs: bool,
    pub example_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocStyle {
    RustDoc,     // /// Rust documentation
    JavaDoc,     // /** JavaDoc style */
    Python,      // """ Python docstrings """
    JSDoc,       // /** @param {type} name */
    Doxygen,     // /*! Doxygen style */
    Unknown,
}

impl Default for DocStyle {
    fn default() -> Self {
        DocStyle::Unknown
    }
}

pub async fn infer_conventions(project_root: &Path) -> anyhow::Result<CodingConventions> {
    let mut conventions = CodingConventions::default();
    let mut samples = ConventionSamples::default();

    // Collect samples from source files
    for entry in WalkDir::new(project_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_ignored(e.path()))
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && is_source_file(path) {
            if let Ok(content) = fs::read_to_string(path).await {
                analyze_file_conventions(path, &content, &mut samples);
            }
        }
    }

    // Infer conventions from samples
    conventions.naming = infer_naming_conventions(&samples);
    conventions.formatting = infer_formatting_rules(&samples);
    conventions.structure = infer_structure_conventions(&samples);
    conventions.documentation = infer_doc_conventions(&samples);

    Ok(conventions)
}

#[derive(Default)]
struct ConventionSamples {
    function_names: Vec<String>,
    type_names: Vec<String>,
    constant_names: Vec<String>,
    variable_names: Vec<String>,
    file_names: Vec<String>,
    module_names: Vec<String>,
    indent_samples: Vec<String>,
    brace_samples: Vec<String>,
    doc_samples: Vec<String>,
    test_locations: Vec<String>,
    file_lengths: Vec<usize>,
    function_lengths: Vec<usize>,
}

fn analyze_file_conventions(path: &Path, content: &str, samples: &mut ConventionSamples) {
    let language = detect_language_from_path(path);
    
    // Collect file name
    if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
        samples.file_names.push(name.to_string());
    }

    // Analyze based on language
    match language.as_deref() {
        Some("Rust") => analyze_rust_conventions(content, samples),
        Some("Python") => analyze_python_conventions(content, samples),
        Some("JavaScript") | Some("TypeScript") => analyze_js_conventions(content, samples),
        Some("Go") => analyze_go_conventions(content, samples),
        Some("Java") => analyze_java_conventions(content, samples),
        _ => {}
    }

    // Collect general samples
    collect_indent_samples(content, samples);
    collect_brace_samples(content, samples);
    
    // File metrics
    samples.file_lengths.push(content.lines().count());
    
    // Test location
    if path.to_string_lossy().contains("test") {
        samples.test_locations.push(path.to_string_lossy().to_string());
    }
}

fn analyze_rust_conventions(content: &str, samples: &mut ConventionSamples) {
    // Function names
    let fn_regex = Regex::new(r"fn\s+([a-zA-Z_]\w*)").unwrap();
    for cap in fn_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            samples.function_names.push(name.as_str().to_string());
        }
    }

    // Struct/Enum names
    let type_regex = Regex::new(r"(?:struct|enum|trait)\s+([a-zA-Z_]\w*)").unwrap();
    for cap in type_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            samples.type_names.push(name.as_str().to_string());
        }
    }

    // Constants
    let const_regex = Regex::new(r"const\s+([A-Z_]\w*)").unwrap();
    for cap in const_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            samples.constant_names.push(name.as_str().to_string());
        }
    }

    // Variables
    let let_regex = Regex::new(r"let\s+(?:mut\s+)?([a-z_]\w*)").unwrap();
    for cap in let_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            samples.variable_names.push(name.as_str().to_string());
        }
    }

    // Module names
    let mod_regex = Regex::new(r"mod\s+([a-z_]\w*)").unwrap();
    for cap in mod_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            samples.module_names.push(name.as_str().to_string());
        }
    }

    // Documentation
    if content.contains("///") {
        samples.doc_samples.push("rustdoc".to_string());
    }
}

fn analyze_python_conventions(content: &str, samples: &mut ConventionSamples) {
    // Function names
    let fn_regex = Regex::new(r"def\s+([a-zA-Z_]\w*)").unwrap();
    for cap in fn_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            samples.function_names.push(name.as_str().to_string());
        }
    }

    // Class names
    let class_regex = Regex::new(r"class\s+([a-zA-Z_]\w*)").unwrap();
    for cap in class_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            samples.type_names.push(name.as_str().to_string());
        }
    }

    // Constants (usually uppercase)
    let const_regex = Regex::new(r"^([A-Z_]+)\s*=").unwrap();
    for cap in const_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            samples.constant_names.push(name.as_str().to_string());
        }
    }

    // Documentation
    if content.contains(r#""""#) || content.contains("'''") {
        samples.doc_samples.push("python".to_string());
    }
}

fn analyze_js_conventions(content: &str, samples: &mut ConventionSamples) {
    // Function names
    let fn_regex = Regex::new(r"function\s+([a-zA-Z_$]\w*)").unwrap();
    for cap in fn_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            samples.function_names.push(name.as_str().to_string());
        }
    }

    // Arrow functions
    let arrow_regex = Regex::new(r"(?:const|let|var)\s+([a-zA-Z_$]\w*)\s*=\s*(?:\([^)]*\)|[^=])\s*=>").unwrap();
    for cap in arrow_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            samples.function_names.push(name.as_str().to_string());
        }
    }

    // Class names
    let class_regex = Regex::new(r"class\s+([a-zA-Z_$]\w*)").unwrap();
    for cap in class_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            samples.type_names.push(name.as_str().to_string());
        }
    }

    // Constants
    let const_regex = Regex::new(r"const\s+([A-Z_]+)\s*=").unwrap();
    for cap in const_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            samples.constant_names.push(name.as_str().to_string());
        }
    }

    // Documentation
    if content.contains("/**") {
        samples.doc_samples.push("jsdoc".to_string());
    }
}

fn analyze_go_conventions(content: &str, samples: &mut ConventionSamples) {
    // Function names
    let fn_regex = Regex::new(r"func\s+(?:\([^)]+\)\s+)?([a-zA-Z_]\w*)").unwrap();
    for cap in fn_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            samples.function_names.push(name.as_str().to_string());
        }
    }

    // Type names
    let type_regex = Regex::new(r"type\s+([a-zA-Z_]\w*)").unwrap();
    for cap in type_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            samples.type_names.push(name.as_str().to_string());
        }
    }

    // Constants
    let const_regex = Regex::new(r"const\s+([a-zA-Z_]\w*)").unwrap();
    for cap in const_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            samples.constant_names.push(name.as_str().to_string());
        }
    }
}

fn analyze_java_conventions(content: &str, samples: &mut ConventionSamples) {
    // Method names
    let method_regex = Regex::new(r"(?:public|private|protected)\s+\w+\s+([a-zA-Z_]\w*)\s*\(").unwrap();
    for cap in method_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            samples.function_names.push(name.as_str().to_string());
        }
    }

    // Class names
    let class_regex = Regex::new(r"class\s+([a-zA-Z_]\w*)").unwrap();
    for cap in class_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            samples.type_names.push(name.as_str().to_string());
        }
    }

    // Constants
    let const_regex = Regex::new(r"static\s+final\s+\w+\s+([A-Z_]+)").unwrap();
    for cap in const_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            samples.constant_names.push(name.as_str().to_string());
        }
    }

    // Documentation
    if content.contains("/**") {
        samples.doc_samples.push("javadoc".to_string());
    }
}

fn collect_indent_samples(content: &str, samples: &mut ConventionSamples) {
    for line in content.lines().take(100) {
        if line.starts_with(' ') || line.starts_with('\t') {
            let indent = line.chars().take_while(|c| c.is_whitespace()).collect::<String>();
            if !indent.is_empty() {
                samples.indent_samples.push(indent);
            }
        }
    }
}

fn collect_brace_samples(content: &str, samples: &mut ConventionSamples) {
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains('{') {
            if line.trim().ends_with('{') {
                samples.brace_samples.push("same_line".to_string());
            } else if i > 0 && lines[i - 1].trim().ends_with(')') {
                samples.brace_samples.push("next_line".to_string());
            }
        }
    }
}

fn infer_naming_conventions(samples: &ConventionSamples) -> NamingConventions {
    NamingConventions {
        functions: detect_naming_style(&samples.function_names),
        types: detect_naming_style(&samples.type_names),
        constants: detect_naming_style(&samples.constant_names),
        variables: detect_naming_style(&samples.variable_names),
        files: detect_naming_style(&samples.file_names),
        modules: detect_naming_style(&samples.module_names),
    }
}

fn detect_naming_style(names: &[String]) -> NamingStyle {
    let mut style_counts = HashMap::new();
    
    for name in names {
        let style = identify_naming_style(name);
        *style_counts.entry(style).or_insert(0) += 1;
    }
    
    style_counts.into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(style, _)| style)
        .unwrap_or(NamingStyle::Unknown)
}

fn identify_naming_style(name: &str) -> NamingStyle {
    if name.chars().all(|c| c.is_uppercase() || c == '_') {
        NamingStyle::UpperSnakeCase
    } else if name.contains('_') && name.chars().all(|c| c.is_lowercase() || c == '_' || c.is_numeric()) {
        NamingStyle::SnakeCase
    } else if name.contains('-') {
        NamingStyle::KebabCase
    } else if name.chars().next().map_or(false, |c| c.is_uppercase()) {
        NamingStyle::PascalCase
    } else if name.chars().any(|c| c.is_uppercase()) {
        NamingStyle::CamelCase
    } else {
        NamingStyle::Unknown
    }
}

fn infer_formatting_rules(samples: &ConventionSamples) -> FormattingRules {
    let mut rules = FormattingRules::default();
    
    // Detect indent style
    let mut space_count = 0;
    let mut tab_count = 0;
    for indent in &samples.indent_samples {
        if indent.contains('\t') {
            tab_count += 1;
        } else {
            space_count += 1;
        }
    }
    
    rules.indent_style = if tab_count > space_count {
        IndentStyle::Tabs
    } else if space_count > 0 {
        IndentStyle::Spaces
    } else {
        IndentStyle::Mixed
    };
    
    // Detect indent size (for spaces)
    if rules.indent_style == IndentStyle::Spaces {
        let mut size_counts = HashMap::new();
        for indent in &samples.indent_samples {
            if !indent.contains('\t') {
                let size = indent.len();
                if size > 0 && size <= 8 {
                    *size_counts.entry(size).or_insert(0) += 1;
                }
            }
        }
        
        if let Some((size, _)) = size_counts.into_iter().max_by_key(|(_, count)| *count) {
            rules.indent_size = size;
        } else {
            rules.indent_size = 4; // Default
        }
    }
    
    // Detect brace style
    let mut same_line = 0;
    let mut next_line = 0;
    for sample in &samples.brace_samples {
        match sample.as_str() {
            "same_line" => same_line += 1,
            "next_line" => next_line += 1,
            _ => {}
        }
    }
    
    rules.brace_style = if same_line > next_line {
        BraceStyle::SameLine
    } else if next_line > same_line {
        BraceStyle::NextLine
    } else {
        BraceStyle::Unknown
    };
    
    rules
}

fn infer_structure_conventions(samples: &ConventionSamples) -> StructureConventions {
    let mut conventions = StructureConventions::default();
    
    // Detect test location
    let mut separate_dir = 0;
    let mut same_file = 0;
    for location in &samples.test_locations {
        if location.contains("/tests/") || location.contains("/test/") {
            separate_dir += 1;
        } else if location.contains("_test") || location.contains("test_") {
            same_file += 1;
        }
    }
    
    conventions.test_location = if separate_dir > same_file {
        TestLocation::SeparateDirectory
    } else if same_file > 0 {
        TestLocation::SameFile
    } else {
        TestLocation::Mixed
    };
    
    // Calculate average file length
    if !samples.file_lengths.is_empty() {
        let avg_length = samples.file_lengths.iter().sum::<usize>() / samples.file_lengths.len();
        conventions.max_file_length = Some(avg_length * 2); // Set max as 2x average
    }
    
    conventions
}

fn infer_doc_conventions(samples: &ConventionSamples) -> DocConventions {
    let mut conventions = DocConventions::default();
    
    // Detect documentation style
    let mut style_counts = HashMap::new();
    for sample in &samples.doc_samples {
        *style_counts.entry(sample.as_str()).or_insert(0) += 1;
    }
    
    if let Some((style, _)) = style_counts.into_iter().max_by_key(|(_, count)| *count) {
        conventions.style = match style {
            "rustdoc" => DocStyle::RustDoc,
            "javadoc" => DocStyle::JavaDoc,
            "python" => DocStyle::Python,
            "jsdoc" => DocStyle::JSDoc,
            _ => DocStyle::Unknown,
        };
    }
    
    // Set documentation requirements based on presence
    conventions.require_function_docs = !samples.doc_samples.is_empty();
    
    conventions
}

fn detect_language_from_path(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| match ext {
            "rs" => Some("Rust".to_string()),
            "py" => Some("Python".to_string()),
            "js" | "mjs" => Some("JavaScript".to_string()),
            "ts" => Some("TypeScript".to_string()),
            "go" => Some("Go".to_string()),
            "java" => Some("Java".to_string()),
            "cpp" | "cc" | "cxx" => Some("C++".to_string()),
            "c" => Some("C".to_string()),
            "cs" => Some("C#".to_string()),
            _ => None,
        })
}

fn is_source_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        matches!(
            ext,
            "rs" | "py" | "js" | "ts" | "go" | "java" | "cpp" | "c" | "cs" | "rb" | "php" | "swift" | "kt"
        )
    } else {
        false
    }
}

fn is_ignored(path: &Path) -> bool {
    path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        name.starts_with('.') && name != "." && name != ".." ||
        name == "node_modules" ||
        name == "target" ||
        name == "dist" ||
        name == "__pycache__" ||
        name == ".git" ||
        name == "vendor"
    })
}