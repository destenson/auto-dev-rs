pub mod analyzer;
pub mod embeddings;
pub mod manager;
pub mod query;
pub mod storage;

#[cfg(test)]
mod tests;

pub use manager::ContextManager;
pub use query::ContextQuery;
pub use storage::ProjectContext;
