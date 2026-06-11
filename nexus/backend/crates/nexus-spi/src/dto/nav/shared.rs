//! Shared nav-tree DTO shapes: the target tagged union, the context payload, and
//! the node detail returned to clients (WS-13 §4).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;
use uuid::Uuid;

/// What a nav node points at — exactly one of group / dashboard / route, an
/// internally-tagged union on `kind`. This is the wire shape of the stored
/// `target` JSONB; the store persists it verbatim. A `group` is a non-clickable
/// header, a `dashboard` is a reusable page mount, a `route` is one of the app's
/// built-in static pages (a closed allow-list, not free-form).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NavTarget {
    /// A non-clickable organisational header.
    Group,
    /// A reusable dashboard page mounted at this node. The id is validated
    /// against a tenant-scoped lookup by the handler (a bare FK would not encode
    /// same-tenant), never trusted from the client.
    Dashboard {
        #[serde(rename = "dashboardId")]
        dashboard_id: Uuid,
    },
    /// A built-in static app page, from the closed [`StaticRoute`] allow-list.
    Route { route: StaticRoute },
}

/// The closed allow-list of built-in app pages a `route` node may point at —
/// the router table's static entries (`ui/src/app/router.tsx`). A node cannot
/// point at an arbitrary URL; this is what lets a static page be access-gated by
/// a nav node exactly like a dashboard mount (WS-13 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StaticRoute {
    Dashboards,
    Explore,
    Datasources,
    Flows,
    Insights,
    Detections,
    Findings,
    Agents,
    Access,
    Audit,
}

/// A nav node's context payload — applied only to `dashboard` targets. EXACTLY
/// `{ values?, tags? }` per the §1 merge contract: `values` become
/// `PageContext.values` (explicit overrides a `context`/`values` variable
/// reads), `tags` are merged *over* the dashboard's own tags. There is no
/// `varOverrides` channel — a node overrides a variable's current value through
/// `values` + a `context` variable on the normal WS-02 selection path.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, ToSchema)]
pub struct NavContext {
    /// Explicit variable-value overrides for this mount (e.g. `{ building: "b1" }`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, serde_json::Value>,
    /// Tag pins/overrides merged over the dashboard's own tags for this mount.
    /// A null value clears a tag for the mount without retagging the page.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tags: BTreeMap<String, Option<String>>,
}

/// A nav node as returned to clients. `target`/`context` are the typed shapes
/// above; `path` is not stored — the tree is built client-side from `parent_id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct NavNodeDetail {
    pub id: Uuid,
    /// The parent node, or `None` for a root node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Uuid>,
    /// Display label ("Buildings", "Building-1", "Agents").
    pub title: String,
    /// Position among siblings; lower sorts first.
    pub sort_order: i32,
    pub target: NavTarget,
    /// Present only for `dashboard` targets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<NavContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
}
