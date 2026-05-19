//! Namespaced migration runner. Mirrors the sqlite crate's module
//! exactly so consumers can swap stores without changing call sites.

mod runner;
mod source;

pub use runner::migrate;
pub use source::MigrationSource;
