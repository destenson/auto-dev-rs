//! Claude configuration support for auto-dev-rs
//!
//! This module provides discovery and management of Claude configuration files
//! including CLAUDE.md instructions and custom commands from .claude directories.

pub mod claude_md;
pub mod command_parser;
pub mod command_registry;
pub mod command_types;
pub mod context_integration;
pub mod discovery;

pub use claude_md::{ClaudeMdContent, ClaudeMdLoader};
pub use command_parser::CommandParser;
pub use command_registry::{CommandRegistrySystem, CommandSource, RegisteredCommand, CommandStats};
pub use command_types::{ClaudeCommand, CommandArgument, CommandRegistry};
pub use context_integration::{ClaudeContextProvider, ClaudeContextExtension};
pub use discovery::{ClaudeConfigDiscovery, ClaudeConfigLocation, ClaudeConfigPaths};
