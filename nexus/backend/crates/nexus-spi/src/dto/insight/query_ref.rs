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
/// When both `script` and `insight_id` are set, `insight_id` wins (a stored
/// insight is the authored source of truth; an inline script is the override only
/// when no id is given). Caps still apply *after* the insight runs: it can
/// aggregate the result down but the surface guarantees it never grows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct InsightRef {
    /// A stored insight id to run. Resolved and tenant-authorised server-side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insight_id: Option<Uuid>,
    /// An inline Rhai script to run instead, when no `insight_id` is given. The
    /// script orchestrates the curated vectorized surface over the result frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    /// Parameters bound as the script's `params` object. Arbitrary JSON; the
    /// script reads `params.<field>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}
