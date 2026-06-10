//! The optional insight applied to a query result before serialization.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

/// How a query attaches a post-query insight transform. A request may either
/// inline a `script` (the ad-hoc / preview case) or name a stored insight by
/// `insight_id` (the panel case); `params` feeds either as the script's `params`
/// object. Both fields are optional and the whole `InsightRef` is optional on the
/// request, so a query without one behaves exactly as before — this is a purely
/// additive contract, like the RW-05 `sources` field.
///
/// A request may also name an extension-contributed insight by `insight_name`
/// (the global registry an installed extension contributes via
/// `contributes.insights[]`). Precedence when more than one is set: a stored
/// `insight_id` (tenant-authored) wins, then `insight_name` (global, admin-
/// curated), then an inline `script`. Caps still apply *after* the insight runs:
/// it can aggregate the result down but the surface guarantees it never grows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct InsightRef {
    /// A stored insight id to run. Resolved and tenant-authorised server-side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insight_id: Option<Uuid>,
    /// An extension-contributed insight name to run (e.g. `com.nexus.hello.zscore`).
    /// Resolved against the global extension-insight registry server-side; the
    /// script runs against the caller's own result rows, so a global definition
    /// only ever touches the caller's data. Additive and optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insight_name: Option<String>,
    /// An inline Rhai script to run instead, when no `insight_id` is given. The
    /// script orchestrates the curated vectorized surface over the result frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    /// Parameters bound as the script's `params` object. Arbitrary JSON; the
    /// script reads `params.<field>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}
