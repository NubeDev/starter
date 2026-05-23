//! Catalog helpers — thin wrappers around the typed CRUD that
//! `starter_store_postgres::dimensions::*` already ships, plus the
//! [`ext`] module that enforces the W12 manifest-hash re-quarantine
//! transaction.

pub mod ext;
pub mod mart_spec;

use thiserror::Error;

/// Catalog-level error surface. Mostly a re-export of `sqlx::Error`
/// with the warehouse-specific structural failures (re-quarantine,
/// quota, frozen sandbox) carried as typed variants.
#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("catalog sql error: {0}")]
    Sql(#[from] sqlx::Error),
    /// The mart's `created_by` prefix is not one of the three
    /// W12-author types (`user:`, `agent:`, `ext:`).
    #[error("invalid created_by prefix: {0:?}")]
    BadCreatedBy(String),
    /// W12 — extension manifest hash is not in
    /// `ext_manifest_approvals`. Caller must run `mart.promote`
    /// after the operator reviews the new hash.
    #[error("extension {ext_id:?} manifest hash {hash:?} is not approved")]
    ExtManifestNotApproved { ext_id: String, hash: String },
    /// W12 — live-mart quota exceeded.
    #[error("live-mart quota of {quota} reached")]
    LiveMartQuotaExceeded { quota: i32 },
    /// RF-4 — sandbox is frozen (a cleaner was promoted from it);
    /// `sandbox.redefine` is refused.
    #[error("sandbox {name:?} is frozen at revision {revision}")]
    SandboxFrozen { name: String, revision: i64 },
}
