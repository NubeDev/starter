//! `rubix.analytics.query` — request/response DTOs and tool descriptor.
//!
//! DTOs are `utoipa::ToSchema`-derived; the descriptor is a
//! `&'static` value (anti-prompt-injection parity with skill
//! bundles).
//!
//! The verb looks up a *named* SQL template (one of the six bundled
//! under `rubix-tools/src/analytics/templates/`) and runs it against
//! ClickHouse with the supplied params bound through CH's native
//! `{name:Type}` parameter syntax — never string concatenation. This
//! keeps callers (and LLMs) off the raw-SQL surface. See
//! [docs/design/analytics/](../../../../docs/design/analytics/README.md).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.analytics.query`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AnalyticsQueryRequest {
    /// Template name (matches a file under `analytics/templates/`
    /// without the `.sql` suffix). The catalogue of valid names is
    /// closed — unknown names yield `rubix.analytics.query.unknown_template`.
    pub name: String,
    /// Optional parameter map. Keys must match `{name:Type}` slots
    /// in the template; values are JSON scalars/arrays that the
    /// official ClickHouse Rust client serialises via its `param()`
    /// API (no string interpolation).
    #[serde(default)]
    #[schema(value_type = Object)]
    pub params: BTreeMap<String, Value>,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AnalyticsQueryResponse {
    /// Outcome (`rubix.analytics.query.ran`).
    pub summary: Diagnostic,
    /// Echoed template name.
    pub name: String,
    /// Result rows. Each row is a JSON object keyed by column name;
    /// shape depends on the template. Empty when the query returned
    /// no rows.
    #[schema(value_type = Vec<Object>)]
    pub rows: Vec<Value>,
    /// Number of rows in `rows`. Surfaced separately for callers
    /// that only need the count (e.g. the report agent's "did the
    /// dataset have any signal?" branch).
    pub row_count: u32,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "analytics.read";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Run a named, parameterised ClickHouse analytics query and return its rows.",
    when_to_use: concat!(
        "Use when the agent needs read-only warehouse data for a ",
        "dashboard, report, or alert decision — and a bundled named ",
        "template already covers the shape (disk_history_weekly, ",
        "alert_count_weekly, flow_run_summary_weekly, user_activity_weekly, ",
        "clickhouse_writes_weekly, undo_count_weekly)."
    ),
    when_not_to_use: concat!(
        "Do not use to write/alter ClickHouse state (call ",
        "rubix.warehouse.rule.write or rubix.warehouse.mart.create). ",
        "Do not use to render a multi-query report (call ",
        "rubix.analytics.report — it stitches several queries into a ",
        "single rendered artifact). Do not use for ad-hoc SQL — the ",
        "template catalogue is closed; add a new template instead."
    ),
    example: concat!(
        "Input:  { \"name\": \"disk_history_weekly\", \"params\": {} }\n",
        "Output: { \"summary\": { \"code\": \"rubix.analytics.query.ran\", ",
        "\"params\": { \"name\": \"disk_history_weekly\", \"rows\": 7 } }, ",
        "\"name\": \"disk_history_weekly\", \"rows\": [ ... ], \"row_count\": 7 }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.analytics.report",
            wins_when: "the caller wants a rendered HTML/CSV/JSON artifact that combines several named queries.",
        },
        SiblingTool {
            id: "rubix.warehouse.rule.write",
            wins_when: "the goal is to write or alter a derived-state object, not read from one.",
        },
    ],
};
