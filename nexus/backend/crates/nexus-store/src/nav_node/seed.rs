//! Seed a tenant's default navigation tree (WS-13 §6).
//!
//! A fresh tenant should not face an empty sidebar, so its first nav read seeds
//! one `route` node per built-in static page. Seeding is **idempotent**: it only
//! runs when the tenant has no nodes yet, so it is safe to call on every list.
//!
//! Access note: these nodes are seeded structurally; granting them `tenant`
//! scope (so non-admins see them) is an authz-layer concern handled where the
//! policy store is in scope — admins see every node via the built-in admin rule
//! regardless, so the default tree is immediately navigable for an admin.

use sqlx::PgPool;
use starter_spi::Error;

use super::insert::insert;
use super::record::{NavNodeRecord, NewNavNode};

/// The built-in static routes a default tree mounts, in sidebar order. These are
/// exactly the closed `route` allow-list (the router's static pages); the title
/// is the human label the sidebar shows.
const DEFAULT_ROUTES: &[(&str, &str)] = &[
    ("Dashboards", "dashboards"),
    ("Explore", "explore"),
    ("Datasources", "datasources"),
    ("Flows", "flows"),
    ("Insights", "insights"),
    ("Alerts", "alerts"),
    ("Findings", "findings"),
    ("Agents", "agents"),
    ("Access", "access"),
    ("Audit", "audit"),
];

/// Seed the tenant's default tree if (and only if) it currently has no nodes.
/// Returns the nodes created (empty when the tenant already had a tree, so a
/// caller can grant-seed exactly the new rows). Idempotent and safe to call on
/// every nav read.
pub async fn seed_default_tree_if_empty(
    pool: &PgPool,
    tenant_id: &str,
) -> Result<Vec<NavNodeRecord>, Error> {
    // Cheap guard: if any node exists, the tree was already built (seeded or
    // hand-authored) — never re-seed over a tenant's own structure.
    if !super::fetch::list(pool, tenant_id).await?.is_empty() {
        return Ok(Vec::new());
    }
    let mut created = Vec::with_capacity(DEFAULT_ROUTES.len());
    for (i, (title, route)) in DEFAULT_ROUTES.iter().enumerate() {
        let node = insert(
            pool,
            tenant_id,
            &NewNavNode {
                parent_id: None,
                title: (*title).to_string(),
                sort_order: i as i32,
                target: serde_json::json!({ "kind": "route", "route": route }),
                context: None,
                icon: None,
                accent: None,
            },
        )
        .await?;
        created.push(node);
    }
    Ok(created)
}

/// Backfill any built-in `route` node the tenant is missing, without disturbing
/// its existing tree. Unlike [`seed_default_tree_if_empty`] (which only runs on
/// a brand-new, empty tenant), this reconciles a tenant that was seeded before a
/// new built-in page existed — e.g. an established tenant that predates the
/// `insights` route. Returns the nodes created (empty when nothing was missing)
/// so the caller can grant `view` on exactly the new rows.
///
/// A route is considered present if any node targets it; a deliberately-deleted
/// built-in route will therefore be re-created. That is the intended trade-off:
/// built-in pages are meant to be reachable, and there is no tombstone to tell
/// "deleted" from "never seeded". New nodes append after the highest existing
/// root `sort_order` so they slot at the end rather than reshuffling the tree.
pub async fn reconcile_default_routes(
    pool: &PgPool,
    tenant_id: &str,
) -> Result<Vec<NavNodeRecord>, Error> {
    let existing = super::fetch::list(pool, tenant_id).await?;
    // A fresh tenant gets the full ordered seed instead, so order is canonical.
    if existing.is_empty() {
        return seed_default_tree_if_empty(pool, tenant_id).await;
    }
    let has_route = |route: &str| {
        existing.iter().any(|n| {
            n.target.get("kind").and_then(|k| k.as_str()) == Some("route")
                && n.target.get("route").and_then(|r| r.as_str()) == Some(route)
        })
    };
    // Append after the last root node so the backfill never reorders siblings.
    let mut next_order = existing
        .iter()
        .filter(|n| n.parent_id.is_none())
        .map(|n| n.sort_order)
        .max()
        .unwrap_or(-1)
        + 1;

    let mut created = Vec::new();
    for (title, route) in DEFAULT_ROUTES {
        if has_route(route) {
            continue;
        }
        let node = insert(
            pool,
            tenant_id,
            &NewNavNode {
                parent_id: None,
                title: (*title).to_string(),
                sort_order: next_order,
                target: serde_json::json!({ "kind": "route", "route": route }),
                context: None,
                icon: None,
                accent: None,
            },
        )
        .await?;
        created.push(node);
        next_order += 1;
    }
    Ok(created)
}
