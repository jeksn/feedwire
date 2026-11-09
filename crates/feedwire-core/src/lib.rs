//! FeedWire core: the backend shared by every frontend (Tauri GUI, terminal UI).
//!
//! This crate owns the SQLite database (schema, migrations, queries), RSS/Atom
//! fetching and parsing, article filtering, OPML import/export, and the feed
//! refresh engine. It is deliberately free of any UI framework code.

pub mod database;
pub mod feed;
pub mod filters;
pub mod models;
pub mod opml;
pub mod refresh;

#[cfg(test)]
mod tests;

pub use database::*;
pub use feed::FeedParser;
pub use models::*;

/// App identifier used by the macOS GUI build to resolve its data directory.
pub const APP_IDENTIFIER: &str = "com.jeksn.feedwire";
/// SQLite database file name living inside the app data directory.
pub const DB_FILE_NAME: &str = "feedwire.db";
