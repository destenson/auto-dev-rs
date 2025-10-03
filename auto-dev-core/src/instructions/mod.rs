//! Instruction file parser and loader
//!
//! Reads project specifications from various formats and extracts structured requirements.

mod extractor;
mod formats;
mod parser;

pub use extractor::{MetadataExtractor, ProjectMetadata};
pub use formats::{Format, detect_format};
pub use parser::{InstructionParser, ParsedInstruction};

use anyhow::Result;
use std::path::Path;

/// Load instructions from a file or string
pub fn load_instructions(source: &str) -> Result<ParsedInstruction> {
    if Path::new(source).exists() {
        InstructionParser::from_file(source)
    } else {
        InstructionParser::from_string(source)
    }
}

/// Quick helper to extract metadata from instructions
pub fn extract_metadata(source: &str) -> Result<ProjectMetadata> {
    let instruction = load_instructions(source)?;
    MetadataExtractor::extract(&instruction)
}
