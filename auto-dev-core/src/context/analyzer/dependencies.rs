#![allow(unused)]
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DependencyGraph {
    pub modules: Vec<Module>,
    pub edges: Vec<DependencyEdge>,
    pub external_dependencies: Vec<ExternalDependency>,
    pub circular_dependencies: Vec<CircularDependency>,
    #[serde(skip)]
    graph: Option<DiGraph<String, DependencyType>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    pub name: String,
    pub path: PathBuf,
    pub module_type: ModuleType,
    pub imports: Vec<Import>,
    pub exports: Vec<Export>,
    pub size: usize,
    pub complexity: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleType {
    Library,
    Binary,
    Test,
    Example,
    Build,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    pub from: String,
    pub items: Vec<String>,
    pub import_type: ImportType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportType {
    Direct,   // use foo::bar
    Wildcard, // use foo::*
    Aliased,  // use foo::bar as baz
    External, // external crate
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Export {
    pub name: String,
    pub export_type: ExportType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportType {
    Function,
    Struct,
    Enum,
    Trait,
    Module,
    Constant,
    Type,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
    pub dependency_type: DependencyType,
    pub weight: usize, // Number of imports
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    Uses,       // Normal dependency
    Tests,      // Test dependency
    Implements, // Trait implementation
    Extends,    // Inheritance
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDependency {
    pub name: String,
    pub version: Option<String>,
    pub source: DependencySource,
    pub used_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencySource {
    Crates,   // crates.io
    Git,      // Git repository
    Path,     // Local path
    Registry, // Custom registry
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircularDependency {
    pub cycle: Vec<String>,
    pub severity: CircularDependencySeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CircularDependencySeverity {
    Low,    // Indirect or test-only
    Medium, // Direct but limited scope
    High,   // Core modules involved
}

pub async fn analyze_dependencies(project_root: &Path) -> anyhow::Result<DependencyGraph> {
    let mut graph = DependencyGraph::default();

    // Detect project type and analyze accordingly
    if project_root.join("Cargo.toml").exists() {
        analyze_rust_dependencies(project_root, &mut graph).await?;
    } else if project_root.join("package.json").exists() {
        analyze_js_dependencies(project_root, &mut graph).await?;
    } else if project_root.join("go.mod").exists() {
        analyze_go_dependencies(project_root, &mut graph).await?;
    } else if project_root.join("pom.xml").exists() {
        analyze_java_dependencies(project_root, &mut graph).await?;
    } else if project_root.join("requirements.txt").exists()
        || project_root.join("setup.py").exists()
    {
        analyze_python_dependencies(project_root, &mut graph).await?;
    }

    // Build the actual graph
    build_dependency_graph(&mut graph);

    // Detect circular dependencies
    detect_circular_dependencies(&mut graph);

    // Calculate metrics
    calculate_dependency_metrics(&mut graph);

    Ok(graph)
}

async fn analyze_rust_dependencies(
    project_root: &Path,
    graph: &mut DependencyGraph,
) -> anyhow::Result<()> {
    // Parse Cargo.toml for external dependencies
    let cargo_toml_path = project_root.join("Cargo.toml");
    if let Ok(content) = fs::read_to_string(&cargo_toml_path).await {
        parse_cargo_toml(&content, graph)?;
    }

    // Analyze Rust source files
    let src_dir = project_root.join("src");
    if src_dir.exists() {
        analyze_rust_modules(&src_dir, graph).await?;
    }

    // Also check for workspace members
    let cargo_toml_path = project_root.join("Cargo.toml");
    if let Ok(content) = fs::read_to_string(&cargo_toml_path).await {
        if content.contains("[workspace]") {
            // Parse workspace members
            let workspace_regex = Regex::new(r#"members\s*=\s*\[(.*?)\]"#).unwrap();
            if let Some(cap) = workspace_regex.captures(&content.replace('\n', " ")) {
                if let Some(members) = cap.get(1) {
                    let members_str = members.as_str();
                    let member_regex = Regex::new(r#""([^"]+)""#).unwrap();
                    for cap in member_regex.captures_iter(members_str) {
                        if let Some(member) = cap.get(1) {
                            let member_path = project_root.join(member.as_str());
                            if member_path.exists() {
                                analyze_rust_modules(&member_path.join("src"), graph).await?;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn parse_cargo_toml(content: &str, graph: &mut DependencyGraph) -> anyhow::Result<()> {
    // Simple regex-based parsing for dependencies
    let dep_regex =
        Regex::new(r#"(\w+)\s*=\s*(?:"([^"]+)"|\{[^}]+version\s*=\s*"([^"]+)"\})"#).unwrap();

    let mut in_dependencies = false;
    let mut in_dev_dependencies = false;

    for line in content.lines() {
        if line.starts_with("[dependencies]") {
            in_dependencies = true;
            in_dev_dependencies = false;
        } else if line.starts_with("[dev-dependencies]") {
            in_dependencies = false;
            in_dev_dependencies = true;
        } else if line.starts_with('[') {
            in_dependencies = false;
            in_dev_dependencies = false;
        } else if (in_dependencies || in_dev_dependencies) && !line.trim().is_empty() {
            if let Some(cap) = dep_regex.captures(line) {
                let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                let version = cap.get(2).or_else(|| cap.get(3)).map(|m| m.as_str().to_string());

                if !name.is_empty() {
                    graph.external_dependencies.push(ExternalDependency {
                        name,
                        version,
                        source: DependencySource::Crates,
                        used_by: Vec::new(),
                    });
                }
            }
        }
    }

    Ok(())
}

async fn analyze_rust_modules(src_dir: &Path, graph: &mut DependencyGraph) -> anyhow::Result<()> {
    use walkdir::WalkDir;

    for entry in WalkDir::new(src_dir)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_ignored(e.path()))
    {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            if let Ok(content) = fs::read_to_string(path).await {
                let module = analyze_rust_file(path, &content)?;
                graph.modules.push(module);
            }
        }
    }

    Ok(())
}

fn analyze_rust_file(path: &Path, content: &str) -> anyhow::Result<Module> {
    let name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();

    let module_type = if path.to_string_lossy().contains("test") {
        ModuleType::Test
    } else if name == "main" {
        ModuleType::Binary
    } else if path.to_string_lossy().contains("examples") {
        ModuleType::Example
    } else if path.to_string_lossy().contains("build") {
        ModuleType::Build
    } else {
        ModuleType::Library
    };

    let mut imports = Vec::new();
    let mut exports = Vec::new();

    // Parse imports
    let use_regex = Regex::new(r"use\s+([\w:]+)(?:\s+as\s+\w+)?").unwrap();
    for cap in use_regex.captures_iter(content) {
        if let Some(import) = cap.get(1) {
            let from = import.as_str().to_string();
            let import_type = if from.contains('*') {
                ImportType::Wildcard
            } else if content.contains(&format!("{} as", from)) {
                ImportType::Aliased
            } else if from.starts_with("crate::")
                || from.starts_with("super::")
                || from.starts_with("self::")
            {
                ImportType::Direct
            } else {
                ImportType::External
            };

            imports.push(Import { from: from.clone(), items: vec![from], import_type });
        }
    }

    // Parse exports (public items)
    let pub_fn_regex = Regex::new(r"pub\s+(?:async\s+)?fn\s+(\w+)").unwrap();
    for cap in pub_fn_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            exports.push(Export {
                name: name.as_str().to_string(),
                export_type: ExportType::Function,
            });
        }
    }

    let pub_struct_regex = Regex::new(r"pub\s+struct\s+(\w+)").unwrap();
    for cap in pub_struct_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            exports
                .push(Export { name: name.as_str().to_string(), export_type: ExportType::Struct });
        }
    }

    let pub_enum_regex = Regex::new(r"pub\s+enum\s+(\w+)").unwrap();
    for cap in pub_enum_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            exports.push(Export { name: name.as_str().to_string(), export_type: ExportType::Enum });
        }
    }

    let pub_trait_regex = Regex::new(r"pub\s+trait\s+(\w+)").unwrap();
    for cap in pub_trait_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            exports
                .push(Export { name: name.as_str().to_string(), export_type: ExportType::Trait });
        }
    }

    // Calculate basic metrics
    let size = content.len();
    let complexity = calculate_complexity(content);

    Ok(Module { name, path: path.to_path_buf(), module_type, imports, exports, size, complexity })
}

async fn analyze_js_dependencies(
    project_root: &Path,
    graph: &mut DependencyGraph,
) -> anyhow::Result<()> {
    // Parse package.json
    let package_json_path = project_root.join("package.json");
    if let Ok(content) = fs::read_to_string(&package_json_path).await {
        parse_package_json(&content, graph)?;
    }

    // Analyze JavaScript/TypeScript files
    analyze_js_modules(project_root, graph).await?;

    Ok(())
}

fn parse_package_json(content: &str, graph: &mut DependencyGraph) -> anyhow::Result<()> {
    // Simple regex-based parsing
    let dep_regex = Regex::new(r#""(\w[\w-]*)":\s*"([^"]+)""#).unwrap();

    let mut in_dependencies = false;
    let mut in_dev_dependencies = false;

    for line in content.lines() {
        if line.contains("\"dependencies\"") {
            in_dependencies = true;
            in_dev_dependencies = false;
        } else if line.contains("\"devDependencies\"") {
            in_dependencies = false;
            in_dev_dependencies = true;
        } else if line.contains('}') && (in_dependencies || in_dev_dependencies) {
            in_dependencies = false;
            in_dev_dependencies = false;
        } else if (in_dependencies || in_dev_dependencies) {
            if let Some(cap) = dep_regex.captures(line) {
                let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                let version = cap.get(2).map(|m| m.as_str().to_string());

                if !name.is_empty() {
                    graph.external_dependencies.push(ExternalDependency {
                        name,
                        version,
                        source: DependencySource::Registry,
                        used_by: Vec::new(),
                    });
                }
            }
        }
    }

    Ok(())
}

async fn analyze_js_modules(
    project_root: &Path,
    graph: &mut DependencyGraph,
) -> anyhow::Result<()> {
    use walkdir::WalkDir;

    for entry in WalkDir::new(project_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_ignored(e.path()))
    {
        let entry = entry?;
        let path = entry.path();

        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if matches!(ext, "js" | "jsx" | "ts" | "tsx" | "mjs") {
                if let Ok(content) = fs::read_to_string(path).await {
                    let module = analyze_js_file(path, &content)?;
                    graph.modules.push(module);
                }
            }
        }
    }

    Ok(())
}

fn analyze_js_file(path: &Path, content: &str) -> anyhow::Result<Module> {
    let name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();

    let module_type =
        if path.to_string_lossy().contains("test") || path.to_string_lossy().contains("spec") {
            ModuleType::Test
        } else {
            ModuleType::Library
        };

    let mut imports = Vec::new();
    let mut exports = Vec::new();

    // Parse imports
    let import_regex =
        Regex::new(r#"import\s+(?:\{[^}]+\}|\w+)\s+from\s+['"]([^'"]+)['"]"#).unwrap();
    for cap in import_regex.captures_iter(content) {
        if let Some(from) = cap.get(1) {
            imports.push(Import {
                from: from.as_str().to_string(),
                items: Vec::new(),
                import_type: ImportType::Direct,
            });
        }
    }

    // Parse require statements
    let require_regex = Regex::new(r#"require\(['"]([^'"]+)['"]\)"#).unwrap();
    for cap in require_regex.captures_iter(content) {
        if let Some(from) = cap.get(1) {
            imports.push(Import {
                from: from.as_str().to_string(),
                items: Vec::new(),
                import_type: ImportType::Direct,
            });
        }
    }

    // Parse exports
    let export_regex =
        Regex::new(r"export\s+(?:default\s+)?(?:function|class|const|let|var)\s+(\w+)").unwrap();
    for cap in export_regex.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            exports.push(Export {
                name: name.as_str().to_string(),
                export_type: ExportType::Function,
            });
        }
    }

    let size = content.len();
    let complexity = calculate_complexity(content);

    Ok(Module { name, path: path.to_path_buf(), module_type, imports, exports, size, complexity })
}

async fn analyze_go_dependencies(
    project_root: &Path,
    graph: &mut DependencyGraph,
) -> anyhow::Result<()> {
    // Parse go.mod
    let go_mod_path = project_root.join("go.mod");
    if let Ok(content) = fs::read_to_string(&go_mod_path).await {
        parse_go_mod(&content, graph)?;
    }

    // Analyze Go files
    analyze_go_modules(project_root, graph).await?;

    Ok(())
}

fn parse_go_mod(content: &str, graph: &mut DependencyGraph) -> anyhow::Result<()> {
    let require_regex = Regex::new(r"require\s+([^\s]+)\s+([^\s]+)").unwrap();

    for cap in require_regex.captures_iter(content) {
        let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
        let version = cap.get(2).map(|m| m.as_str().to_string());

        if !name.is_empty() {
            graph.external_dependencies.push(ExternalDependency {
                name,
                version,
                source: DependencySource::Registry,
                used_by: Vec::new(),
            });
        }
    }

    Ok(())
}

async fn analyze_go_modules(
    project_root: &Path,
    graph: &mut DependencyGraph,
) -> anyhow::Result<()> {
    use walkdir::WalkDir;

    for entry in WalkDir::new(project_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_ignored(e.path()))
    {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("go") {
            if let Ok(content) = fs::read_to_string(path).await {
                let module = analyze_go_file(path, &content)?;
                graph.modules.push(module);
            }
        }
    }

    Ok(())
}

fn analyze_go_file(path: &Path, content: &str) -> anyhow::Result<Module> {
    let name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();

    let module_type = if path.to_string_lossy().contains("test") {
        ModuleType::Test
    } else if name == "main" {
        ModuleType::Binary
    } else {
        ModuleType::Library
    };

    let mut imports = Vec::new();

    // Parse imports
    let import_regex = Regex::new(r#"import\s+(?:\([^)]+\)|"[^"]+")"#).unwrap();
    for cap in import_regex.captures_iter(content) {
        if let Some(import) = cap.get(0) {
            let import_str = import.as_str();
            let path_regex = Regex::new(r#""([^"]+)""#).unwrap();
            for path_cap in path_regex.captures_iter(import_str) {
                if let Some(path) = path_cap.get(1) {
                    imports.push(Import {
                        from: path.as_str().to_string(),
                        items: Vec::new(),
                        import_type: ImportType::Direct,
                    });
                }
            }
        }
    }

    let size = content.len();
    let complexity = calculate_complexity(content);

    Ok(Module {
        name,
        path: path.to_path_buf(),
        module_type,
        imports,
        exports: Vec::new(),
        size,
        complexity,
    })
}

async fn analyze_java_dependencies(
    project_root: &Path,
    graph: &mut DependencyGraph,
) -> anyhow::Result<()> {
    // Parse pom.xml or build.gradle
    if project_root.join("pom.xml").exists() {
        let pom_path = project_root.join("pom.xml");
        if let Ok(content) = fs::read_to_string(&pom_path).await {
            parse_pom_xml(&content, graph)?;
        }
    }

    Ok(())
}

fn parse_pom_xml(content: &str, graph: &mut DependencyGraph) -> anyhow::Result<()> {
    let dep_regex = Regex::new(r"<dependency>.*?<groupId>([^<]+)</groupId>.*?<artifactId>([^<]+)</artifactId>.*?<version>([^<]+)</version>.*?</dependency>").unwrap();

    for cap in dep_regex.captures_iter(&content.replace('\n', " ")) {
        let group_id = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let artifact_id = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let version = cap.get(3).map(|m| m.as_str().to_string());

        if !group_id.is_empty() && !artifact_id.is_empty() {
            let name = format!("{}:{}", group_id, artifact_id);
            graph.external_dependencies.push(ExternalDependency {
                name,
                version,
                source: DependencySource::Registry,
                used_by: Vec::new(),
            });
        }
    }

    Ok(())
}

async fn analyze_python_dependencies(
    project_root: &Path,
    graph: &mut DependencyGraph,
) -> anyhow::Result<()> {
    // Parse requirements.txt
    let requirements_path = project_root.join("requirements.txt");
    if let Ok(content) = fs::read_to_string(&requirements_path).await {
        parse_requirements_txt(&content, graph)?;
    }

    // Parse setup.py
    let setup_path = project_root.join("setup.py");
    if let Ok(content) = fs::read_to_string(&setup_path).await {
        parse_setup_py(&content, graph)?;
    }

    Ok(())
}

fn parse_requirements_txt(content: &str, graph: &mut DependencyGraph) -> anyhow::Result<()> {
    for line in content.lines() {
        let line = line.trim();
        if !line.is_empty() && !line.starts_with('#') {
            let parts: Vec<&str> = line.split("==").collect();
            let name = parts[0].to_string();
            let version = if parts.len() > 1 { Some(parts[1].to_string()) } else { None };

            graph.external_dependencies.push(ExternalDependency {
                name,
                version,
                source: DependencySource::Registry,
                used_by: Vec::new(),
            });
        }
    }

    Ok(())
}

fn parse_setup_py(content: &str, graph: &mut DependencyGraph) -> anyhow::Result<()> {
    let install_requires_regex = Regex::new(r"install_requires\s*=\s*\[(.*?)\]").unwrap();

    if let Some(cap) = install_requires_regex.captures(&content.replace('\n', " ")) {
        if let Some(deps) = cap.get(1) {
            let dep_regex = Regex::new(r#"'([^']+)'|"([^"]+)""#).unwrap();
            for cap in dep_regex.captures_iter(deps.as_str()) {
                let dep = cap
                    .get(1)
                    .or_else(|| cap.get(2))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();

                if !dep.is_empty() {
                    let parts: Vec<&str> = dep.split("==").collect();
                    let name = parts[0].to_string();
                    let version = if parts.len() > 1 { Some(parts[1].to_string()) } else { None };

                    graph.external_dependencies.push(ExternalDependency {
                        name,
                        version,
                        source: DependencySource::Registry,
                        used_by: Vec::new(),
                    });
                }
            }
        }
    }

    Ok(())
}

fn build_dependency_graph(graph: &mut DependencyGraph) {
    let mut petgraph = DiGraph::new();
    let mut node_map: HashMap<String, NodeIndex> = HashMap::new();

    // Add nodes
    for module in &graph.modules {
        let node = petgraph.add_node(module.name.clone());
        node_map.insert(module.name.clone(), node);
    }

    // Add edges based on imports
    for module in &graph.modules {
        for import in &module.imports {
            // Try to find the target module
            let target = extract_module_name(&import.from);
            if let Some(target_node) = node_map.get(&target) {
                if let Some(source_node) = node_map.get(&module.name) {
                    petgraph.add_edge(*source_node, *target_node, DependencyType::Uses);

                    // Add to edges list
                    graph.edges.push(DependencyEdge {
                        from: module.name.clone(),
                        to: target.clone(),
                        dependency_type: DependencyType::Uses,
                        weight: 1,
                    });
                }
            }
        }
    }

    graph.graph = Some(petgraph);
}

fn extract_module_name(import_path: &str) -> String {
    // Extract the module name from an import path
    import_path
        .split("::")
        .last()
        .or_else(|| import_path.split('/').last())
        .or_else(|| import_path.split('.').last())
        .unwrap_or(import_path)
        .to_string()
}

fn detect_circular_dependencies(graph: &mut DependencyGraph) {
    if let Some(ref petgraph) = graph.graph {
        // Try topological sort - if it fails, there are cycles
        if toposort(&petgraph, None).is_err() {
            // Find cycles using DFS
            let cycles = find_cycles(petgraph);

            for cycle in cycles {
                let severity = if cycle.len() <= 2 {
                    CircularDependencySeverity::Low
                } else if cycle.len() <= 4 {
                    CircularDependencySeverity::Medium
                } else {
                    CircularDependencySeverity::High
                };

                graph.circular_dependencies.push(CircularDependency { cycle, severity });
            }
        }
    }
}

fn find_cycles(graph: &DiGraph<String, DependencyType>) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut rec_stack = Vec::new();

    for node in graph.node_indices() {
        if !visited.contains(&node) {
            dfs_find_cycles(graph, node, &mut visited, &mut rec_stack, &mut cycles);
        }
    }

    cycles
}

fn dfs_find_cycles(
    graph: &DiGraph<String, DependencyType>,
    node: NodeIndex,
    visited: &mut HashSet<NodeIndex>,
    rec_stack: &mut Vec<NodeIndex>,
    cycles: &mut Vec<Vec<String>>,
) {
    visited.insert(node);
    rec_stack.push(node);

    for neighbor in graph.neighbors(node) {
        if let Some(pos) = rec_stack.iter().position(|&n| n == neighbor) {
            // Found a cycle
            let cycle: Vec<String> =
                rec_stack[pos..].iter().filter_map(|&n| graph.node_weight(n).cloned()).collect();

            if !cycle.is_empty() {
                cycles.push(cycle);
            }
        } else if !visited.contains(&neighbor) {
            dfs_find_cycles(graph, neighbor, visited, rec_stack, cycles);
        }
    }

    rec_stack.pop();
}

fn calculate_dependency_metrics(graph: &mut DependencyGraph) {
    // Update external dependency usage
    for module in &graph.modules {
        for import in &module.imports {
            if matches!(import.import_type, ImportType::External) {
                let import_name = extract_module_name(&import.from);
                for ext_dep in &mut graph.external_dependencies {
                    if ext_dep.name == import_name || import.from.contains(&ext_dep.name) {
                        ext_dep.used_by.push(module.name.clone());
                    }
                }
            }
        }
    }

    // Consolidate duplicate edges
    let mut edge_map: HashMap<(String, String), usize> = HashMap::new();
    for edge in &graph.edges {
        let key = (edge.from.clone(), edge.to.clone());
        *edge_map.entry(key).or_insert(0) += 1;
    }

    graph.edges.clear();
    for ((from, to), weight) in edge_map {
        graph.edges.push(DependencyEdge {
            from,
            to,
            dependency_type: DependencyType::Uses,
            weight,
        });
    }
}

fn calculate_complexity(content: &str) -> usize {
    // Simple cyclomatic complexity calculation
    let mut complexity = 1;

    // Count decision points
    let decision_keywords = [
        "if ", "else if", "elif ", "for ", "while ", "match ", "case ", "catch ", "except ", "&&",
        "||", "?",
    ];

    for keyword in &decision_keywords {
        complexity += content.matches(keyword).count();
    }

    complexity
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
            || name == "vendor"
    })
}
