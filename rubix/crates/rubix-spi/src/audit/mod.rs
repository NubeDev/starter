//! Audit-policy SPI \u{2014} async store trait + value type for the
//! `changelog_kind_policy` table that drives audit-retention.
//!
//! Lives next to (but not under) [`crate::dto`] because the DTOs
//! are wire-shapes for the `rubix.audit.policy.*` tool surface
//! while this module is the host-side contract the verb bodies
//! and the production Pg impl share.
//!
//! Backed by [`rubix-store-postgres::audit::PgAuditPolicyStore`]
//! in production; the in-memory test fake + the [`Reversible`]
//! glue both live in `rubix-tools::audit::store` so this crate
//! retains zero deps on `sqlx`, `tokio`, or the verb dispatch
//! layer per SCOPE R6.
//!
//! [`Reversible`]: starter_spi::changelog::Reversible

pub mod store;

pub use store::{AuditPolicyRow, AuditPolicyStore, AUDIT_POLICY_KIND};
