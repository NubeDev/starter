//! A named set of migrations. Mirrors the sqlite crate's `MigrationSource`.

/// One migration source: name + sqlx migrator borrow.
#[derive(Copy, Clone)]
pub struct MigrationSource {
    /// Source identifier. Must match `^[a-z][a-z0-9_]{0,30}$`.
    pub name: &'static str,
    /// The sqlx migrator over the consumer's migration files.
    pub migrator: &'static sqlx::migrate::Migrator,
}
