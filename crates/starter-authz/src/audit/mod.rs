//! Phase 7c — decision audit log. SCOPE-EXT.md R14.
//!
//! Every `PolicyEngine::check()` produces a [`DecisionEntry`].
//! Where it goes is up to the wired-in [`DecisionSink`]:
//!
//! - [`NoopDecisionSink`] (the default) — silently drops. Zero-
//!   overhead opt-in matches Phase 1–6's "you pay nothing if you
//!   don't enable it."
//! - [`DbDecisionSink`] (feature `sqlite` / `postgres`) — appends to
//!   `starter_authz_decisions` via a bounded `mpsc::channel` and a
//!   dedicated writer task. On overflow the row is dropped with a
//!   `tracing::warn { dropped_count }`. **`record()` never blocks
//!   `check()`** — the dispatch is `try_send`, not an awaited
//!   blocking write.
//!
//! See [`spawn_retention`] for the retention story — if the binary
//! never spawns the retention task, the table grows without bound.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::Effect;

#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub mod db;

#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub use db::{DbDecisionSink, DecisionSinkConfig, RetentionConfig};

/// One audit row. The split between `rule_id` and `reason` is
/// load-bearing (SCOPE-EXT.md R14): `reason` is `Some` only when
/// the decision came from engine semantics
/// (`"cross_tenant"`, `"no_tenant_binding"`,
/// `"unknown_resource"`, `"no_matching_rule"`), and `rule_id` is
/// `Some` only when a rule actually matched. They are
/// **independent** so a rule whose id happens to be
/// `"cross_tenant"` is never confused with the engine code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionEntry {
    /// Wall-clock time the engine produced the decision.
    pub at: DateTime<Utc>,
    /// Principal's tenant binding (if any).
    pub tenant: Option<String>,
    /// `Principal.subject` — opaque user identifier.
    pub subject: String,
    /// Lowercased role name (`"reader" | "writer" | "admin"`).
    pub principal_role: String,
    /// Action requested.
    pub action: String,
    /// Resource kind.
    pub kind: String,
    /// Resource id (collection checks are `None`).
    pub id: Option<String>,
    /// Allow or Deny.
    pub effect: Effect,
    /// Identifier of the matched rule. `Some` only when a rule
    /// matched (allow-by-rule or explicit-deny rule). Independent
    /// of `reason` per R14.
    pub rule_id: Option<String>,
    /// Engine-supplied reason code when the decision came from
    /// engine semantics. `Some` only for `cross_tenant`,
    /// `no_tenant_binding`, `unknown_resource`, `no_matching_rule`,
    /// `not_owner`, `explicit_deny`, `condition_invalid`.
    /// Independent of `rule_id` so a rule whose id happens to be
    /// `"cross_tenant"` is never confused with the built-in code.
    pub reason: Option<String>,
}

/// Best-effort decision sink. SCOPE-EXT.md R14 specifies the
/// default shape:
///
/// - **non-blocking** — `record` MUST NOT block `check()`. The
///   shipped DB impl uses a bounded channel + writer task.
/// - **drop-on-overflow** — when the queue is full the sink drops
///   the row (with a `tracing::warn` carrying `dropped_count`) and
///   returns. The request still succeeds.
/// - **fail-open** — a sink that errors must not change the
///   request's Allow/Deny outcome. Errors go to `tracing::error`
///   and stop there.
///
/// A consumer that needs fail-closed durable audit wires a custom
/// sink whose `record` blocks the request path and a wrapping
/// engine that maps the error to `Deny { reason:
/// "audit_unavailable" }`. The shape is supported; it is not the
/// default.
#[async_trait]
pub trait DecisionSink: Send + Sync {
    /// Record one decision. Default behaviour is non-blocking;
    /// see the trait-level doc.
    async fn record(&self, entry: DecisionEntry);
}

/// Default sink — silently drops every row. Zero overhead matches
/// Phase 1–6's "you pay nothing if you don't enable it."
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopDecisionSink;

#[async_trait]
impl DecisionSink for NoopDecisionSink {
    async fn record(&self, _entry: DecisionEntry) {}
}
