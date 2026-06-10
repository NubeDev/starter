//! Row and input shapes for the nav-tree store.

use serde_json::Value;
use uuid::Uuid;

/// A stored navigation node (WS-13). Nestable via `parent_id` (NULL = root).
/// `target` is the JSONB tagged union (`group` | `dashboard` | `route`) and
/// `context` the dashboard-mount payload (`{ values?, tags? }`, NULL otherwise);
/// both are opaque here — the API DTO owns and validates their shapes, the store
/// only persists them, like `query_kind::params_schema`.
#[derive(Debug, Clone)]
pub struct NavNodeRecord {
    pub id: Uuid,
    pub tenant_id: String,
    pub parent_id: Option<Uuid>,
    pub title: String,
    pub sort_order: i32,
    pub target: Value,
    pub context: Option<Value>,
    pub icon: Option<String>,
    pub accent: Option<String>,
}

/// A new nav node to insert.
#[derive(Debug, Clone)]
pub struct NewNavNode {
    pub parent_id: Option<Uuid>,
    pub title: String,
    pub sort_order: i32,
    pub target: Value,
    pub context: Option<Value>,
    pub icon: Option<String>,
    pub accent: Option<String>,
}

/// A partial update; `None` leaves a field unchanged (COALESCE in the store).
/// `parent_id` is three-valued (leave / move-under / re-root) so it is wrapped:
/// `Some(Some(p))` moves under `p`, `Some(None)` re-roots, `None` leaves it.
/// `context`/`icon`/`accent` are likewise nested so a node can clear them (e.g.
/// retargeting a dashboard node to a group drops its context).
#[derive(Debug, Clone, Default)]
pub struct NavNodePatch {
    pub parent_id: Option<Option<Uuid>>,
    pub title: Option<String>,
    pub sort_order: Option<i32>,
    pub target: Option<Value>,
    pub context: Option<Option<Value>>,
    pub icon: Option<Option<String>>,
    pub accent: Option<Option<String>>,
}
