//! SQLite-backed storage implementation.

mod comments;
mod config;
mod dependencies;
mod issues;
mod labels;
mod queries;
pub mod schema;
mod store;
mod transaction;

pub use store::SqliteStore;
