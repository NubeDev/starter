//! Namespaced migration runner. Multiple migration sources can
//! coexist in one database — each uses its own `_sqlx_migrations_<source>`
//! table so version numbers never collide.

mod runner;
mod source;

pub use runner::{migrate, Migrate};
pub use source::MigrationSource;
