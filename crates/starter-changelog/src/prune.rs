//! Retention pruning surface.
//!
//! SCOPE §"Open questions" item 2: no automatic TTL. Backends
//! implement [`Prune`]; this crate exposes the trait + a parameter
//! struct the `prune` CLI subcommand will pass through. The CLI
//! itself wires into `starter-cli` later — kept out of this crate
//! to preserve R1.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use starter_spi::Result;

/// Inputs for a prune pass.
#[derive(Debug, Clone)]
pub struct PruneRequest {
    /// Delete rows with `at < before`.
    pub before: DateTime<Utc>,
    /// Optional resource-kind narrowing.
    pub resource_kind: Option<String>,
    /// If `true`, count rows that would be deleted but do not delete.
    pub dry_run: bool,
}

/// Outcome of a prune pass.
#[derive(Debug, Clone, Default)]
pub struct PruneReport {
    /// Number of rows deleted (or matched, when `dry_run`).
    pub rows: u64,
}

/// Implemented by changelog backends.
#[async_trait]
pub trait Prune: Send + Sync {
    /// Run a prune pass under `req`.
    async fn prune(&self, req: &PruneRequest) -> Result<PruneReport>;
}
