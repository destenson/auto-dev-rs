//! Claude Code CLI integration module
//!
//! Provides detection, execution, and integration with the Claude Code CLI tool.

pub mod detector;

pub use detector::{ClaudeDetector, ClaudeLocation, DetectionError};
