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
    ("Alerts", "alerts"),
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
