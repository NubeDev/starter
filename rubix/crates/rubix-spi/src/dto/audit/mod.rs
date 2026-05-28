//! audit goal — REST DTOs + tool descriptors.
//!
//! Operator surface for the `changelog_kind_policy` table (the
//! audit-retention contract). Today the policy is configured
//! only via SQL seed migrations (`rubix-store-postgres`
//! `changelog_policy/0001_audit_floor_seed.sql` etc.); the
//! verbs in this goal give live operators a way to inspect and
//! adjust per-kind retention without touching SQL.
//!
//! The policy is intentionally a small surface: each row is
//! `(resource_kind, max_age_days)` where `NULL` pins the kind
//! to "keep forever" and a positive integer applies that
//! retention curve to the per-kind sweep in
//! `rubix-agent::boot::changelog_sweep`.

pub mod policy_list;
pub mod policy_set;
