//! Documentation generation module

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;
use chrono::Local;

use super::{DocEntry, DocConfig, DocFormat, ApiDoc, Example, DocSection, ApiItemKind};

/// Generates documentation in various formats
pub struct DocGenerator {
    config: DocConfig,
}

impl DocGenerator {
    /// Create new documentation generator
    pub fn new(config: DocConfig) -> Self {
        Self { config }
    }

    /// Generate documentation files from entries
    pub fn generate_docs(&self, entries: &[DocEntry], output_dir: &Path) -> Result<Vec<PathBuf>> {
        let mut generated_files = Vec::new();
        
        // Create output directory
        fs::create_dir_all(output_dir)?;
        
        for format in &self.config.formats {
            match format {
                DocFormat::Markdown => {
                    let files = self.generate_markdown(entries, output_dir)?;
                    generated_files.extend(files);
                }
                DocFormat::Html => {
                    let files = self.generate_html(entries, output_dir)?;
                    generated_files.extend(files);
                }
                DocFormat::Json => {
                    let file = self.generate_json(entries, output_dir)?;
                    generated_files.push(file);
                }
            }
        }
        
        // Generate index file
        let index = self.generate_index(entries, output_dir)?;
        generated_files.push(index);
        
        Ok(generated_files)
    }

    /// Generate API documentation
    pub fn generate_api_docs(&self, api_docs: &[ApiDoc], output_dir: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let api_dir = output_dir.join("api");
        fs::create_dir_all(&api_dir)?;
        
        // Group by kind
        let mut by_kind: std::collections::HashMap<ApiItemKind, Vec<&ApiDoc>> = std::collections::HashMap::new();
        for doc in api_docs {
            by_kind.entry(doc.kind.clone()).or_default().push(doc);
        }
        
        // Generate file for each kind
        for (kind, docs) in by_kind {
            let filename = format!("{:?}.md", kind).to_lowercase();
            let filepath = api_dir.join(filename);
            
            let mut content = String::new();
            content.push_str(&format!("# {} Reference\n\n", kind_to_string(&kind)));
            
            for doc in docs {
                content.push_str(&self.format_api_doc(doc));
                content.push_str("\n---\n\n");
            }
            
            fs::write(&filepath, content)?;
            files.push(filepath);
        }
        
        Ok(files)
    }

    /// Update README with auto-generated sections
    pub fn update_readme(&self, readme_path: &Path) -> Result<()> {
        let content = if readme_path.exists() {
            fs::read_to_string(readme_path)?
        } else {
            String::new()
        };
        
        let updated = self.update_readme_content(&content)?;
        fs::write(readme_path, updated)?;
        
        Ok(())
    }

    fn generate_markdown(&self, entries: &[DocEntry], output_dir: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        for entry in entries {
            let filename = format!("{}.md", entry.metadata.name);
            let filepath = output_dir.join(filename);
            
            let content = self.entry_to_markdown(entry);
            fs::write(&filepath, content)?;
            files.push(filepath);
        }
        
        Ok(files)
    }

    fn generate_html(&self, entries: &[DocEntry], output_dir: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let html_dir = output_dir.join("html");
        fs::create_dir_all(&html_dir)?;
        
        for entry in entries {
            let filename = format!("{}.html", entry.metadata.name);
            let filepath = html_dir.join(filename);
            
            let content = self.entry_to_html(entry);
            fs::write(&filepath, content)?;
            files.push(filepath);
        }
        
        Ok(files)
    }

    fn generate_json(&self, entries: &[DocEntry], output_dir: &Path) -> Result<PathBuf> {
        let filepath = output_dir.join("documentation.json");
        let json = serde_json::to_string_pretty(entries)?;
        fs::write(&filepath, json)?;
        Ok(filepath)
    }

    fn generate_index(&self, entries: &[DocEntry], output_dir: &Path) -> Result<PathBuf> {
        let filepath = output_dir.join("index.md");
        
        let mut content = String::new();
        content.push_str("# Documentation Index\n\n");
        content.push_str(&format!("Generated: {}\n\n", Local::now().format("%Y-%m-%d %H:%M:%S")));
        
        // Table of contents
        content.push_str("## Modules\n\n");
        for entry in entries {
            content.push_str(&format!(
                "- [{}]({}.md) - {}\n",
                entry.metadata.name,
                entry.metadata.name,
                entry.metadata.description
            ));
        }
        
        // Statistics
        content.push_str("\n## Statistics\n\n");
        content.push_str(&format!("- Total modules: {}\n", entries.len()));
        
        let total_apis: usize = entries.iter().map(|e| e.api_docs.len()).sum();
        content.push_str(&format!("- Total API items: {}\n", total_apis));
        
        let total_examples: usize = entries.iter().map(|e| e.examples.len()).sum();
        content.push_str(&format!("- Total examples: {}\n", total_examples));
        
        fs::write(&filepath, content)?;
        Ok(filepath)
    }

    fn entry_to_markdown(&self, entry: &DocEntry) -> String {
        let mut content = String::new();
        
        // Header
        content.push_str(&format!("# {}\n\n", entry.metadata.name));
        
        // Metadata
        if !entry.metadata.description.is_empty() {
            content.push_str(&format!("{}\n\n", entry.metadata.description));
        }
        
        content.push_str(&format!("**Version:** {}\n", entry.metadata.version));
        content.push_str(&format!("**Last Modified:** {}\n\n", entry.metadata.last_modified.format("%Y-%m-%d")));
        
        // Sections
        for section in &entry.sections {
            content.push_str(&self.section_to_markdown(section, 2));
        }
        
        // API Documentation
        if !entry.api_docs.is_empty() {
            content.push_str("## API Reference\n\n");
            for api in &entry.api_docs {
                content.push_str(&self.format_api_doc(api));
            }
        }
        
        // Examples
        if self.config.generate_examples && !entry.examples.is_empty() {
            content.push_str("## Examples\n\n");
            for example in &entry.examples {
                content.push_str(&self.format_example(example));
            }
        }
        
        // See also
        if !entry.see_also.is_empty() {
            content.push_str("## See Also\n\n");
            for link in &entry.see_also {
                content.push_str(&format!("- {}\n", link));
            }
        }
        
        content
    }

    fn section_to_markdown(&self, section: &DocSection, level: usize) -> String {
        let mut content = String::new();
        let heading = "#".repeat(level);
        
        content.push_str(&format!("{} {}\n\n", heading, section.title));
        content.push_str(&format!("{}\n\n", section.content));
        
        for subsection in &section.subsections {
            content.push_str(&self.section_to_markdown(subsection, level + 1));
        }
        
        content
    }

    fn format_api_doc(&self, api: &ApiDoc) -> String {
        let mut content = String::new();
        
        content.push_str(&format!("### `{}`\n\n", api.name));
        
        if !api.signature.is_empty() {
            content.push_str("```rust\n");
            content.push_str(&api.signature);
            content.push_str("\n```\n\n");
        }
        
        if !api.description.is_empty() {
            content.push_str(&format!("{}\n\n", api.description));
        }
        
        // Parameters
        if !api.parameters.is_empty() {
            content.push_str("**Parameters:**\n\n");
            for param in &api.parameters {
                content.push_str(&format!(
                    "- `{}`: `{}` - {}\n",
                    param.name,
                    param.param_type,
                    param.description
                ));
            }
            content.push('\n');
        }
        
        // Returns
        if let Some(ref returns) = api.returns {
            content.push_str(&format!("**Returns:** `{}`\n\n", returns));
        }
        
        // Examples
        if !api.examples.is_empty() {
            content.push_str("**Example:**\n\n");
            content.push_str("```rust\n");
            for example in &api.examples {
                content.push_str(example);
                content.push('\n');
            }
            content.push_str("```\n\n");
        }
        
        content
    }

    fn format_example(&self, example: &Example) -> String {
        let mut content = String::new();
        
        if !example.title.is_empty() {
            content.push_str(&format!("### {}\n\n", example.title));
        }
        
        if !example.description.is_empty() {
            content.push_str(&format!("{}\n\n", example.description));
        }
        
        content.push_str(&format!("```{}\n", example.language));
        content.push_str(&example.code);
        content.push_str("\n```\n\n");
        
        if let Some(ref output) = example.output {
            content.push_str("**Output:**\n\n");
            content.push_str("```\n");
            content.push_str(output);
            content.push_str("\n```\n\n");
        }
        
        content
    }

    fn entry_to_html(&self, entry: &DocEntry) -> String {
        let mut html = String::new();
        
        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str(&format!("<title>{}</title>\n", entry.metadata.name));
        html.push_str("<style>\n");
        html.push_str("body { font-family: sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }\n");
        html.push_str("pre { background: #f4f4f4; padding: 10px; overflow-x: auto; }\n");
        html.push_str("code { background: #f4f4f4; padding: 2px 4px; }\n");
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");
        
        html.push_str(&format!("<h1>{}</h1>\n", entry.metadata.name));
        html.push_str(&format!("<p>{}</p>\n", entry.metadata.description));
        
        // Convert markdown content to HTML (simplified)
        let markdown_content = self.entry_to_markdown(entry);
        html.push_str(&self.simple_markdown_to_html(&markdown_content));
        
        html.push_str("</body>\n</html>");
        
        html
    }

    fn simple_markdown_to_html(&self, markdown: &str) -> String {
        // Simple markdown to HTML conversion (very basic)
        let mut html = String::new();
        let mut in_code_block = false;
        
        for line in markdown.lines() {
            if line.starts_with("```") {
                if in_code_block {
                    html.push_str("</pre>\n");
                    in_code_block = false;
                } else {
                    html.push_str("<pre>");
                    in_code_block = true;
                }
            } else if in_code_block {
                html.push_str(&html_escape(line));
                html.push('\n');
            } else if line.starts_with("# ") {
                html.push_str(&format!("<h1>{}</h1>\n", &line[2..]));
            } else if line.starts_with("## ") {
                html.push_str(&format!("<h2>{}</h2>\n", &line[3..]));
            } else if line.starts_with("### ") {
                html.push_str(&format!("<h3>{}</h3>\n", &line[4..]));
            } else if line.starts_with("- ") {
                html.push_str(&format!("<li>{}</li>\n", &line[2..]));
            } else if !line.is_empty() {
                html.push_str(&format!("<p>{}</p>\n", line));
            }
        }
        
        html
    }

    fn update_readme_content(&self, content: &str) -> Result<String> {
        let mut lines: Vec<String> = content.lines().map(String::from).collect();
        
        // Find or create auto-generated sections
        let features_start = "<!-- AUTO-GENERATED-FEATURES-START -->";
        let features_end = "<!-- AUTO-GENERATED-FEATURES-END -->";
        
        let start_idx = lines.iter().position(|l| l.contains(features_start));
        let end_idx = lines.iter().position(|l| l.contains(features_end));
        
        let features_content = vec![
            features_start.to_string(),
            String::new(),
            "## Features".to_string(),
            String::new(),
            "- Self-monitoring and modification".to_string(),
            "- Automatic documentation generation".to_string(),
            "- Test generation and validation".to_string(),
            "- Code synthesis from specifications".to_string(),
            String::new(),
            features_end.to_string(),
        ];
        
        match (start_idx, end_idx) {
            (Some(start), Some(end)) if start < end => {
                // Replace existing content
                lines.splice(start..=end, features_content);
            }
            _ => {
                // Add new section
                lines.extend(features_content);
            }
        }
        
        Ok(lines.join("\n"))
    }
}

fn kind_to_string(kind: &ApiItemKind) -> &'static str {
    match kind {
        ApiItemKind::Function => "Functions",
        ApiItemKind::Method => "Methods",
        ApiItemKind::Struct => "Structs",
        ApiItemKind::Enum => "Enums",
        ApiItemKind::Trait => "Traits",
        ApiItemKind::Module => "Modules",
        ApiItemKind::Macro => "Macros",
        ApiItemKind::Constant => "Constants",
    }
}

fn html_escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '&' => "&amp;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#39;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}