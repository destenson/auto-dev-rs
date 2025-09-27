//! Documentation formatting utilities

use anyhow::Result;
use std::collections::HashMap;

/// Formats documentation content
pub struct DocFormatter {
    line_width: usize,
    indent_size: usize,
}

impl DocFormatter {
    /// Create new formatter with default settings
    pub fn new() -> Self {
        Self {
            line_width: 80,
            indent_size: 2,
        }
    }

    /// Create formatter with custom settings
    pub fn with_settings(line_width: usize, indent_size: usize) -> Self {
        Self {
            line_width,
            indent_size,
        }
    }

    /// Format markdown content
    pub fn format_markdown(&self, content: &str) -> String {
        let mut formatted = String::new();
        let mut in_code_block = false;
        let mut in_list = false;
        
        for line in content.lines() {
            // Handle code blocks
            if line.starts_with("```") {
                in_code_block = !in_code_block;
                formatted.push_str(line);
                formatted.push('\n');
                continue;
            }
            
            // Don't format inside code blocks
            if in_code_block {
                formatted.push_str(line);
                formatted.push('\n');
                continue;
            }
            
            // Handle lists
            if line.trim_start().starts_with("- ") || line.trim_start().starts_with("* ") {
                if !in_list {
                    formatted.push('\n');
                    in_list = true;
                }
                formatted.push_str(&self.format_list_item(line));
            } else if in_list && line.trim().is_empty() {
                in_list = false;
                formatted.push('\n');
            }
            // Handle headers
            else if line.starts_with('#') {
                if !formatted.is_empty() && !formatted.ends_with("\n\n") {
                    formatted.push('\n');
                }
                formatted.push_str(line);
                formatted.push_str("\n\n");
            }
            // Handle paragraphs
            else if !line.trim().is_empty() {
                formatted.push_str(&self.wrap_text(line));
                formatted.push('\n');
            }
            // Handle empty lines
            else {
                formatted.push('\n');
            }
        }
        
        // Clean up multiple empty lines
        self.clean_empty_lines(&formatted)
    }

    /// Format a list item with proper indentation
    fn format_list_item(&self, line: &str) -> String {
        let trimmed = line.trim_start();
        let indent_level = (line.len() - trimmed.len()) / self.indent_size;
        let indent = " ".repeat(indent_level * self.indent_size);
        
        format!("{}{}\n", indent, trimmed)
    }

    /// Wrap text to specified line width
    fn wrap_text(&self, text: &str) -> String {
        if text.len() <= self.line_width {
            return text.to_string();
        }
        
        let mut wrapped = String::new();
        let mut current_line = String::new();
        
        for word in text.split_whitespace() {
            if current_line.is_empty() {
                current_line.push_str(word);
            } else if current_line.len() + 1 + word.len() <= self.line_width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                wrapped.push_str(&current_line);
                wrapped.push('\n');
                current_line = word.to_string();
            }
        }
        
        if !current_line.is_empty() {
            wrapped.push_str(&current_line);
        }
        
        wrapped
    }

    /// Clean up multiple consecutive empty lines
    fn clean_empty_lines(&self, content: &str) -> String {
        let mut cleaned = String::new();
        let mut prev_empty = false;
        
        for line in content.lines() {
            if line.trim().is_empty() {
                if !prev_empty {
                    cleaned.push('\n');
                    prev_empty = true;
                }
            } else {
                cleaned.push_str(line);
                cleaned.push('\n');
                prev_empty = false;
            }
        }
        
        cleaned.trim().to_string()
    }

    /// Format a table in markdown
    pub fn format_table(&self, headers: &[String], rows: &[Vec<String>]) -> String {
        let mut table = String::new();
        
        // Calculate column widths
        let mut col_widths = vec![0; headers.len()];
        for (i, header) in headers.iter().enumerate() {
            col_widths[i] = header.len();
        }
        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                if i < col_widths.len() {
                    col_widths[i] = col_widths[i].max(cell.len());
                }
            }
        }
        
        // Format headers
        table.push('|');
        for (i, header) in headers.iter().enumerate() {
            table.push_str(&format!(" {:<width$} |", header, width = col_widths[i]));
        }
        table.push('\n');
        
        // Format separator
        table.push('|');
        for width in &col_widths {
            table.push_str(&format!(" {} |", "-".repeat(*width)));
        }
        table.push('\n');
        
        // Format rows
        for row in rows {
            table.push('|');
            for (i, cell) in row.iter().enumerate() {
                if i < col_widths.len() {
                    table.push_str(&format!(" {:<width$} |", cell, width = col_widths[i]));
                }
            }
            table.push('\n');
        }
        
        table
    }

    /// Format code with syntax highlighting hints
    pub fn format_code_block(&self, code: &str, language: &str) -> String {
        format!("```{}\n{}\n```", language, code.trim())
    }

    /// Create a table of contents from headers
    pub fn generate_toc(&self, content: &str) -> String {
        let mut toc = String::new();
        toc.push_str("## Table of Contents\n\n");
        
        for line in content.lines() {
            if line.starts_with("##") && !line.starts_with("## Table of Contents") {
                let level = line.chars().take_while(|&c| c == '#').count();
                let indent = "  ".repeat(level.saturating_sub(2));
                let title = line.trim_start_matches('#').trim();
                let anchor = self.title_to_anchor(title);
                
                toc.push_str(&format!("{}* [{}](#{})\n", indent, title, anchor));
            }
        }
        
        toc
    }

    /// Convert title to markdown anchor
    fn title_to_anchor(&self, title: &str) -> String {
        title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }
}

impl Default for DocFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_text() {
        let formatter = DocFormatter::with_settings(20, 2);
        let text = "This is a long line that needs to be wrapped";
        let wrapped = formatter.wrap_text(text);
        assert!(wrapped.lines().all(|line| line.len() <= 20));
    }

    #[test]
    fn test_format_table() {
        let formatter = DocFormatter::new();
        let headers = vec!["Name".to_string(), "Type".to_string()];
        let rows = vec![
            vec!["foo".to_string(), "String".to_string()],
            vec!["bar".to_string(), "Integer".to_string()],
        ];
        
        let table = formatter.format_table(&headers, &rows);
        assert!(table.contains("| Name | Type    |"));
        assert!(table.contains("| foo  | String  |"));
    }

    #[test]
    fn test_title_to_anchor() {
        let formatter = DocFormatter::new();
        assert_eq!(formatter.title_to_anchor("Hello World"), "hello-world");
        assert_eq!(formatter.title_to_anchor("API Reference"), "api-reference");
    }
}