pub mod manager;
pub mod analyzer;
pub mod storage;
pub mod embeddings;
pub mod query;

#[cfg(test)]
mod tests;

pub use manager::ContextManager;
pub use storage::ProjectContext;
pub use query::ContextQuery;