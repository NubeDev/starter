//! `rubix.dashboard.page_set` — request/response DTOs and tool descriptor.
//!
//! Runtime **slot write** verb. Unlike
//! [`crate::dto::dashboard::update`] (which inserts a new
//! `dashboards_definitions` revision), `page_set` mutates a single
//! live slot on a dashboard target by funneling the write through
//! the same chokepoint the flow engine uses —
//! `starter_flow_spi::graph::GraphStore::write_slot` — so the R2
//! invariant ("slots are the only I/O surface; one write path")
//! holds for operator-driven mutations exactly as it does for
//! propagator-driven mutations.
//!
//! **Not** a revision write. **Not** `starter_undo`-reversible
//! (per `docs/scope/dashboards/08-open-questions.md` OQ-5: the
//! operator's revert path is to set the slot back). The verb
//! emits one structured [`Diagnostic`] keyed
//! `rubix.dashboard.page_set.applied`.
//!
//! See `rubix/docs/scope/dashboards/04-tools.md`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.dashboard.page_set`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PageSetRequest {
    /// Owning tenant — propagated to audit / change-log only;
    /// the slot key itself is `(node_id, slot)`.
    pub tenant_id: String,
    /// SDUI page id whose render-tree carries the action that
    /// produced this write. Carried for audit / diagnostics; the
    /// write target is `(node_id, slot)`, *not* the page id.
    pub page_id: String,
    /// Target node id — must satisfy
    /// `starter_flow_spi::node::NodeId` grammar (lower-case
    /// dotted segments). Typically a flow node id such as
    /// `com.acme.thermostat`.
    pub node_id: String,
    /// Slot name on the target node (e.g. `setpoint`, `enabled`).
    pub slot: String,
    /// New slot value. Coerced into
    /// `starter_flow_spi::node::SlotValue` by the verb body
    /// (booleans → `Bool`, integers → `Int`, floats → `Float`,
    /// strings → `String`, `null` → `Null`, anything else →
    /// `Json`).
    pub value: Value,
    /// Principal performing the write (for audit).
    pub written_by: String,
}

/// Tool reply for `rubix.dashboard.page_set`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PageSetResponse {
    /// Outcome (`rubix.dashboard.page_set.applied`).
    pub summary: Diagnostic,
    /// Echoed page id.
    pub page_id: String,
    /// Echoed target node id.
    pub node_id: String,
    /// Echoed slot name.
    pub slot: String,
    /// `true` once the write returns from
    /// `starter_flow_spi::graph::GraphStore::write_slot` without
    /// error (the chokepoint itself decides whether the value
    /// actually changed via the R3 idempotent short-circuit;
    /// callers should not infer "no-op" from this flag).
    pub written: bool,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "rubix.dashboard.edit";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Write one slot on a live dashboard target through the flow-engine chokepoint.",
    when_to_use: concat!(
        "Use when a user-driven action on a rendered dashboard ",
        "(toggle, slider, form submit) needs to mutate one slot on ",
        "a flow node — the same chokepoint the propagator uses, so ",
        "R2 audit / replay / observability apply uniformly."
    ),
    when_not_to_use: concat!(
        "Do not use to change the dashboard's body (component ",
        "tree) — that is rubix.dashboard.update / .create. Do not ",
        "use to batch many slots — call once per slot so the audit ",
        "row reflects exactly one operator intent."
    ),
    example: concat!(
        "Input:  { \"tenant_id\": \"tenant-a\", ",
        "\"page_id\": \"dashboard.ops\", ",
        "\"node_id\": \"com.acme.thermostat\", ",
        "\"slot\": \"setpoint\", \"value\": 21.5, ",
        "\"written_by\": \"alice\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.dashboard.page_set.applied\", ",
        "\"params\": { \"node_id\": \"com.acme.thermostat\", ",
        "\"slot\": \"setpoint\" } }, ",
        "\"page_id\": \"dashboard.ops\", ",
        "\"node_id\": \"com.acme.thermostat\", ",
        "\"slot\": \"setpoint\", \"written\": true }"
    ),
    siblings: &[SiblingTool {
        id: "rubix.dashboard.update",
        wins_when: "the caller wants to change the page's component tree, not a runtime slot.",
    }],
};
