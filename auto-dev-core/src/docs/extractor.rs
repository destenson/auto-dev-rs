//! Documentation extraction from Rust source code

use anyhow::{Context, Result};
use chrono::Local;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use syn::{Item, ItemEnum, ItemFn, ItemMod, ItemStruct, ItemTrait, Visibility, parse_file};

use super::{ApiDoc, ApiItemKind, DocEntry, DocMetadata, DocSection, Example, Parameter};

/// Extracts documentation from Rust source code
pub struct DocExtractor {
    project_root: PathBuf,
    doc_pattern: Regex,
    example_pattern: Regex,
}

impl DocExtractor {
    /// Create new documentation extractor
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            doc_pattern: Regex::new(r"^///\s?(.*)").unwrap(),
            example_pattern: Regex::new(r"```(?:rust|ignore|no_run)?\n([\s\S]*?)```").unwrap(),
        }
    }

    /// Extract documentation from all modules
    pub fn extract_all(&self, path: &Path) -> Result<Vec<DocEntry>> {
        let mut entries = Vec::new();
        self.walk_directory(path, &mut entries)?;
        Ok(entries)
    }

    /// Extract documentation from a specific module
    pub fn extract_module(&self, module_path: &Path) -> Result<DocEntry> {
        let content = fs::read_to_string(module_path).context("Failed to read module file")?;

        let ast = parse_file(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse Rust file: {}", e))?;

        let module_name = module_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");

        let mut api_docs = Vec::new();
        let mut examples = Vec::new();

        // Extract documentation from AST items
        for item in ast.items {
            match item {
                Item::Fn(item_fn) => {
                    if let Some(doc) = self.extract_fn_doc(&item_fn) {
                        api_docs.push(doc);
                    }
                }
                Item::Struct(item_struct) => {
                    if let Some(doc) = self.extract_struct_doc(&item_struct) {
                        api_docs.push(doc);
                    }
                }
                Item::Enum(item_enum) => {
                    if let Some(doc) = self.extract_enum_doc(&item_enum) {
                        api_docs.push(doc);
                    }
                }
                Item::Trait(item_trait) => {
                    if let Some(doc) = self.extract_trait_doc(&item_trait) {
                        api_docs.push(doc);
                    }
                }
                Item::Mod(item_mod) => {
                    if let Some(doc) = self.extract_mod_doc(&item_mod) {
                        api_docs.push(doc);
                    }
                }
                _ => {}
            }
        }

        // Extract module-level documentation
        let (description, sections) = self.extract_module_docs(&content);

        // Extract examples from documentation
        examples.extend(self.extract_examples(&content));

        Ok(DocEntry {
            metadata: DocMetadata {
                name: module_name.to_string(),
                description,
                version: "0.1.0".to_string(), // Extract from Cargo.toml
                authors: vec![],
                last_modified: Local::now(),
                tags: vec![],
            },
            sections,
            api_docs,
            examples,
            see_also: vec![],
        })
    }

    /// Extract API documentation from entire project
    pub fn extract_api(&self, path: &Path) -> Result<Vec<ApiDoc>> {
        let mut api_docs = Vec::new();
        self.walk_for_api(path, &mut api_docs)?;
        Ok(api_docs)
    }

    /// Validate documentation coverage
    pub fn validate_docs(&self, path: &Path) -> Result<Vec<String>> {
        let mut issues = Vec::new();
        self.check_documentation(path, &mut issues)?;
        Ok(issues)
    }

    /// Calculate documentation coverage percentage
    pub fn calculate_coverage(&self, path: &Path) -> Result<f64> {
        let mut total = 0;
        let mut documented = 0;
        self.count_documentation(path, &mut total, &mut documented)?;

        if total == 0 {
            return Ok(100.0);
        }

        Ok((documented as f64 / total as f64) * 100.0)
    }

    fn walk_directory(&self, path: &Path, entries: &mut Vec<DocEntry>) -> Result<()> {
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(entry) = self.extract_module(path) {
                entries.push(entry);
            }
        } else if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                self.walk_directory(&entry.path(), entries)?;
            }
        }
        Ok(())
    }

    fn walk_for_api(&self, path: &Path, api_docs: &mut Vec<ApiDoc>) -> Result<()> {
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(entry) = self.extract_module(path) {
                api_docs.extend(entry.api_docs);
            }
        } else if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                self.walk_for_api(&entry.path(), api_docs)?;
            }
        }
        Ok(())
    }

    fn extract_fn_doc(&self, item_fn: &ItemFn) -> Option<ApiDoc> {
        // Only document public functions
        if !matches!(item_fn.vis, Visibility::Public(_)) {
            return None;
        }

        let name = item_fn.sig.ident.to_string();
        let signature = self.format_fn_signature(item_fn);

        // Extract parameters
        let parameters = item_fn
            .sig
            .inputs
            .iter()
            .filter_map(|arg| {
                if let syn::FnArg::Typed(pat_type) = arg {
                    if let syn::Pat::Ident(ident) = &*pat_type.pat {
                        return Some(Parameter {
                            name: ident.ident.to_string(),
                            param_type: quote::quote!(#pat_type.ty).to_string(),
                            description: String::new(), // Extract from doc comments
                            required: true,
                        });
                    }
                }
                None
            })
            .collect();

        // Extract return type
        let returns = match &item_fn.sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, ty) => Some(quote::quote!(#ty).to_string()),
        };

        Some(ApiDoc {
            name,
            kind: ApiItemKind::Function,
            signature,
            description: String::new(), // Extract from doc comments
            parameters,
            returns,
            examples: vec![],
            since: None,
        })
    }

    fn extract_struct_doc(&self, item_struct: &ItemStruct) -> Option<ApiDoc> {
        if !matches!(item_struct.vis, Visibility::Public(_)) {
            return None;
        }

        let name = item_struct.ident.to_string();
        let signature = format!("pub struct {}", name);

        Some(ApiDoc {
            name,
            kind: ApiItemKind::Struct,
            signature,
            description: String::new(),
            parameters: vec![],
            returns: None,
            examples: vec![],
            since: None,
        })
    }

    fn extract_enum_doc(&self, item_enum: &ItemEnum) -> Option<ApiDoc> {
        if !matches!(item_enum.vis, Visibility::Public(_)) {
            return None;
        }

        let name = item_enum.ident.to_string();
        let signature = format!("pub enum {}", name);

        Some(ApiDoc {
            name,
            kind: ApiItemKind::Enum,
            signature,
            description: String::new(),
            parameters: vec![],
            returns: None,
            examples: vec![],
            since: None,
        })
    }

    fn extract_trait_doc(&self, item_trait: &ItemTrait) -> Option<ApiDoc> {
        if !matches!(item_trait.vis, Visibility::Public(_)) {
            return None;
        }

        let name = item_trait.ident.to_string();
        let signature = format!("pub trait {}", name);

        Some(ApiDoc {
            name,
            kind: ApiItemKind::Trait,
            signature,
            description: String::new(),
            parameters: vec![],
            returns: None,
            examples: vec![],
            since: None,
        })
    }

    fn extract_mod_doc(&self, item_mod: &ItemMod) -> Option<ApiDoc> {
        if !matches!(item_mod.vis, Visibility::Public(_)) {
            return None;
        }

        let name = item_mod.ident.to_string();
        let signature = format!("pub mod {}", name);

        Some(ApiDoc {
            name,
            kind: ApiItemKind::Module,
            signature,
            description: String::new(),
            parameters: vec![],
            returns: None,
            examples: vec![],
            since: None,
        })
    }

    fn extract_module_docs(&self, content: &str) -> (String, Vec<DocSection>) {
        let mut description = String::new();
        let mut sections = Vec::new();
        let mut current_section: Option<DocSection> = None;

        for line in content.lines() {
            if let Some(captures) = self.doc_pattern.captures(line) {
                if let Some(doc_line) = captures.get(1) {
                    let doc_text = doc_line.as_str();

                    // Check if this is a section header
                    if doc_text.starts_with("# ") {
                        if let Some(section) = current_section.take() {
                            sections.push(section);
                        }
                        current_section = Some(DocSection {
                            title: doc_text[2..].to_string(),
                            content: String::new(),
                            subsections: vec![],
                        });
                    } else if let Some(ref mut section) = current_section {
                        section.content.push_str(doc_text);
                        section.content.push('\n');
                    } else {
                        description.push_str(doc_text);
                        description.push('\n');
                    }
                }
            } else if !line.starts_with("//") {
                // End of doc comments
                break;
            }
        }

        if let Some(section) = current_section {
            sections.push(section);
        }

        (description.trim().to_string(), sections)
    }

    fn extract_examples(&self, content: &str) -> Vec<Example> {
        let mut examples = Vec::new();

        for captures in self.example_pattern.captures_iter(content) {
            if let Some(code_match) = captures.get(1) {
                examples.push(Example {
                    title: format!("Example {}", examples.len() + 1),
                    description: String::new(),
                    code: code_match.as_str().to_string(),
                    output: None,
                    language: "rust".to_string(),
                });
            }
        }

        examples
    }

    fn format_fn_signature(&self, item_fn: &ItemFn) -> String {
        quote::quote!(#item_fn.sig).to_string()
    }

    fn check_documentation(&self, path: &Path, issues: &mut Vec<String>) -> Result<()> {
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let content = fs::read_to_string(path)?;
            if let Ok(ast) = parse_file(&content) {
                for item in ast.items {
                    match item {
                        Item::Fn(item_fn) if matches!(item_fn.vis, Visibility::Public(_)) => {
                            // Check if function has documentation
                            let fn_name = item_fn.sig.ident.to_string();
                            if !self.has_doc_comment(&content, &fn_name) {
                                issues.push(format!(
                                    "Missing documentation for public function: {}:{}",
                                    path.display(),
                                    fn_name
                                ));
                            }
                        }
                        Item::Struct(item_struct)
                            if matches!(item_struct.vis, Visibility::Public(_)) =>
                        {
                            let struct_name = item_struct.ident.to_string();
                            if !self.has_doc_comment(&content, &struct_name) {
                                issues.push(format!(
                                    "Missing documentation for public struct: {}:{}",
                                    path.display(),
                                    struct_name
                                ));
                            }
                        }
                        _ => {}
                    }
                }
            }
        } else if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                self.check_documentation(&entry.path(), issues)?;
            }
        }
        Ok(())
    }

    fn count_documentation(
        &self,
        path: &Path,
        total: &mut usize,
        documented: &mut usize,
    ) -> Result<()> {
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let content = fs::read_to_string(path)?;
            if let Ok(ast) = parse_file(&content) {
                for item in ast.items {
                    match item {
                        Item::Fn(item_fn) if matches!(item_fn.vis, Visibility::Public(_)) => {
                            *total += 1;
                            let fn_name = item_fn.sig.ident.to_string();
                            if self.has_doc_comment(&content, &fn_name) {
                                *documented += 1;
                            }
                        }
                        Item::Struct(item_struct)
                            if matches!(item_struct.vis, Visibility::Public(_)) =>
                        {
                            *total += 1;
                            let struct_name = item_struct.ident.to_string();
                            if self.has_doc_comment(&content, &struct_name) {
                                *documented += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
        } else if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                self.count_documentation(&entry.path(), total, documented)?;
            }
        }
        Ok(())
    }

    fn has_doc_comment(&self, content: &str, item_name: &str) -> bool {
        // Simple heuristic: check if there's a doc comment before the item
        let pattern =
            format!(r"///.*\n.*(?:fn|struct|enum|trait|impl).*{}", regex::escape(item_name));
        Regex::new(&pattern).map(|re| re.is_match(content)).unwrap_or(false)
    }
}
