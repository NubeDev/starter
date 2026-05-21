//! Postgres implementation of the `starter-skills` [`ApprovalStore`]
//! trait (Phase 5, R-skills-7). Twin of the SQLite module — same
//! shape, same migrator pattern, same `default-on skill-approvals`
//! feature gate.
//!
//! [`ApprovalStore`]: starter_skills::ApprovalStore

mod approval_store;

pub use approval_store::SkillApprovalStore;

/// `sqlx` migrator for the skill-approvals schema. Pair with the
/// crate's `migrate(pool).with_source(SKILL_APPROVALS_MIGRATION_SOURCE)`
/// chain.
pub static SKILL_APPROVALS_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/skills");

/// Convenience `MigrationSource` for the skill-approvals schema.
/// Add it to the `migrate(pool)` chain on engine boot if the host
/// uses `starter-skills`.
pub const SKILL_APPROVALS_MIGRATION_SOURCE: crate::migrate::MigrationSource =
    crate::migrate::MigrationSource {
        name: "skill_approvals",
        migrator: &SKILL_APPROVALS_MIGRATOR,
    };
