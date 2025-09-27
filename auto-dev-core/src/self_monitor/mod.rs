
//! Self-monitoring module for auto-dev-rs
//!
//! This module enables auto-dev-rs to monitor its own source directory,
//! providing recursive monitoring capabilities with safety features to
//! prevent infinite loops and dangerous modifications.
//!
//! Key features:
//! - Safe modification boundaries with whitelisted paths
//! - Loop detection to prevent infinite modification cycles
//! - Comprehensive audit trail of all self-modifications
//! - Modification validation and safety checks
//! - Cooldown periods and rate limiting

pub mod audit_trail;
pub mod loop_detector;
pub mod modification_guard;
pub mod self_monitor;

pub use audit_trail::{AuditTrail, AuditEntry, AuditConfig, AuditAction, ModificationInitiator, ModificationResult};
pub use loop_detector::{LoopDetector, LoopDetectorConfig, LoopDetectionResult};
pub use modification_guard::{ModificationGuard, ValidationResult};
pub use self_monitor::{SelfMonitor, SelfMonitorConfig, ModificationRecord, ModificationSource};
