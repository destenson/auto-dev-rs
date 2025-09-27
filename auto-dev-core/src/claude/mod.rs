//! Claude configuration support for auto-dev-rs
//!
//! This module provides discovery and management of Claude configuration files
//! including CLAUDE.md instructions and custom commands from .claude directories.

pub mod claude_md;
pub mod discovery;

pub use claude_md::{ClaudeMdContent, ClaudeMdLoader};
pub use discovery::{ClaudeConfigDiscovery, ClaudeConfigLocation, ClaudeConfigPaths};
