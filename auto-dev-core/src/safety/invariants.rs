#![allow(unused)]
//! System invariants that must always hold

use super::{Result, SafetyError};
use std::path::Path;

/// Checks system invariants
pub struct InvariantChecker {
    invariants: Vec<Box<dyn Invariant>>,
}

impl InvariantChecker {
    pub fn new() -> Self {
        Self {
            invariants: vec![
                Box::new(BuildSystemInvariant),
                Box::new(TestsPassInvariant),
                Box::new(DocumentationInvariant),
            ],
        }
    }

    pub async fn check_all(&self) -> Result<()> {
        for invariant in &self.invariants {
            invariant.check().await?;
        }
        Ok(())
    }
}

/// An invariant that must hold
#[async_trait::async_trait]
pub trait Invariant: Send + Sync {
    async fn check(&self) -> Result<()>;
    fn name(&self) -> &str;
}

struct BuildSystemInvariant;

#[async_trait::async_trait]
impl Invariant for BuildSystemInvariant {
    async fn check(&self) -> Result<()> {
        // TODO: Check that project still builds
        Ok(())
    }

    fn name(&self) -> &str {
        "BuildSystem"
    }
}

struct TestsPassInvariant;

#[async_trait::async_trait]
impl Invariant for TestsPassInvariant {
    async fn check(&self) -> Result<()> {
        // TODO: Check that all tests pass
        Ok(())
    }

    fn name(&self) -> &str {
        "TestsPass"
    }
}

struct DocumentationInvariant;

#[async_trait::async_trait]
impl Invariant for DocumentationInvariant {
    async fn check(&self) -> Result<()> {
        // TODO: Check that documentation is valid
        Ok(())
    }

    fn name(&self) -> &str {
        "Documentation"
    }
}
