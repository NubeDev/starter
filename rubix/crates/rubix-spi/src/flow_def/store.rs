//! [`FlowDefStore`] — async persistence trait the rubix
//! `flow_ops` verbs target.
//!
//! Zero deps on `sqlx`; the PG impl lives in
//! [`rubix-store-postgres::flows`]. The in-memory reference impl
//! used by tests + the laptop no-DB path lives alongside the
//! verb bodies in [`rubix-tools::flow_ops::store`] so the verb
//! crate doesn't pull `sqlx` into the test surface.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use starter_spi::error::Result;

/// Resource-kind discriminator for flow-definition revisions.
/// Used by the `Reversible` impl in
/// [`rubix-tools::flow_ops::store`] and by the changelog query
/// surface so an operator can filter for flow-def writes.
pub const FLOW_DEFINITION_KIND: &str = "flow_definition";

/// One row in the `flows_definitions` table (PG) / the in-memory
/// map (tests).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlowRevisionRow {
    /// ULID/text primary key.
    pub id: String,
    /// Reverse-DNS flow id (rows for one flow share this value).
    pub flow_id: String,
    /// Per-row revision id (UUID text).
    pub revision_id: String,
    /// Raw YAML body, persisted verbatim.
    pub body_yaml: String,
    /// Insertion timestamp (epoch ms, UTC).
    pub created_at_ms: i64,
    /// When set, this revision has been replaced by a newer one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_at_ms: Option<i64>,
}

/// Snapshot payload the verbs stamp into `Change::after`. The
/// `Reversible` impl uses both fields to walk a deploy backwards
/// (mark the new revision superseded, clear the prior's
/// `superseded_at`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlowDefChange {
    /// Flow id this revision belongs to.
    pub flow_id: String,
    /// Revision id created by the verb.
    pub revision_id: String,
    /// Revision id that was marked superseded by this write, or
    /// `None` when this was the first revision for `flow_id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_revision_id: Option<String>,
}

/// Persistence surface the flow-programmer verbs target.
#[async_trait]
pub trait FlowDefStore: Send + Sync {
    /// Insert a new revision and mark the currently-live revision
    /// for `flow_id` superseded. Returns
    /// `(inserted_row, prior_revision_id)`.
    async fn insert_revision(
        &self,
        flow_id: &str,
        body_yaml: &str,
        now_ms: i64,
    ) -> Result<(FlowRevisionRow, Option<String>)>;

    /// Fetch the currently-live revision for `flow_id`.
    async fn fetch_latest_live(&self, flow_id: &str) -> Result<Option<FlowRevisionRow>>;

    /// Return every live revision (one row per `flow_id`).
    async fn list_live(&self) -> Result<Vec<FlowRevisionRow>>;

    /// Mark `revision_id` as superseded at `now_ms`. Used by the
    /// `Reversible` impl to walk a deploy forward, and during
    /// every insert to retire the previous head.
    async fn mark_superseded(&self, revision_id: &str, now_ms: i64) -> Result<()>;

    /// Clear `superseded_at` on `revision_id`. Used by the
    /// `Reversible` impl to walk an inverse — restoring a prior
    /// head after the deploy is undone.
    async fn clear_superseded(&self, revision_id: &str) -> Result<()>;
}
