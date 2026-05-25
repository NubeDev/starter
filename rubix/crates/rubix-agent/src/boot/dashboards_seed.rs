//! Seed the bundled rubix SDUI dashboard JSONs into the
//! `dashboards_definitions` Postgres table on first boot.
//!
//! Mirrors [`super::flows_seed`] in shape and intent, scaled to the
//! Goal-1 dashboard surface:
//!
//! - **Idempotent seed.** For each bundled `.json` file under
//!   [`rubix_flows::BUNDLED`] whose path starts with `dashboards/`,
//!   probe `(tenant_id, page_id) WHERE superseded_at IS NULL`
//!   first; insert only when no live row exists. A second boot is
//!   therefore a no-op.
//! - **Authz registration.** Every insert re-asserts the
//!   `rubix.dashboard.page` resource kind on the
//!   [`ResourceRegistry`] (idempotent via `try_register` —
//!   `DuplicateResource` is treated as success). Mirrors the
//!   goals-2/3/4 flow definitions wiring where each write path
//!   ensures the engine knows about the kind it just touched.
//! - **Laptop fallback.** When the pool is `None` (no Postgres
//!   DSN) the seeder no-ops so the binary still boots without a
//!   database.
//!
//! Phase A.1 leaves the load-back path empty — the page resolver
//! (Phase A.2, `03-host-glue.md`) reads from PG on demand and
//! doesn't need a boot-time eager load.

use anyhow::Result;
use rubix_spi::dashboard::{
    DashboardStore, NewRevision, BUNDLED_PRINCIPAL, BUNDLED_TENANT,
};
use rubix_store_postgres::PgDashboardStore;
use starter_authz::error::Error as AuthzError;
use starter_authz::StaticRegistry;
use starter_spi::authz::{Ownership, ResourceSpec};
use starter_store_postgres::pool::Pool;
use tracing::{debug, info, warn};

/// Re-asserted `ResourceSpec` for SDUI dashboard pages. The same
/// kind is registered eagerly at boot in [`super::authz`]; this
/// helper exists so the seeder (and, later, the `dashboard.create`
/// tool body) can re-register on every write without depending on
/// boot order.
pub fn dashboard_resource_spec() -> ResourceSpec {
    ResourceSpec::from_static_tenant_scoped(
        "rubix.dashboard.page",
        &["view", "edit", "delete"],
        Ownership::Subject,
        "Rubix dashboard page",
        "An SDUI page persisted in `dashboards_definitions` and resolved by the page provider.",
    )
}

/// Idempotently register the dashboard resource kind. Returns
/// `true` when this call inserted the spec, `false` when it was
/// already present (the steady-state path).
pub fn ensure_dashboard_resource(registry: &StaticRegistry) -> bool {
    match registry.try_register(dashboard_resource_spec()) {
        Ok(()) => true,
        Err(AuthzError::DuplicateResource { .. }) => false,
        // Any other variant would indicate registry corruption —
        // surface it via the boot-log warn so an operator notices,
        // but don't abort the seed.
        Err(other) => {
            warn!(target: "rubix.boot", error = %other, "dashboard resource re-register failed");
            false
        }
    }
}

/// Seed bundled dashboard pages into `dashboards_definitions`.
///
/// Returns the count of rows inserted by this call. `None` pool
/// short-circuits to `Ok(0)` for laptop boots.
pub async fn seed(
    pool: Option<&Pool>,
    registry: &StaticRegistry,
) -> Result<usize> {
    // Always ensure the kind is registered, even on the laptop
    // path — the authz gate consults the registry independently of
    // whether any row exists.
    let _ = ensure_dashboard_resource(registry);

    let Some(pool) = pool else {
        debug!(
            target: "rubix.boot",
            "dashboards_definitions seed: skipped (no Postgres pool)",
        );
        return Ok(0);
    };

    let store = PgDashboardStore::new(pool.clone());
    let mut inserted = 0usize;
    for page in bundled_pages() {
        // Idempotency probe — skip when a live row already exists
        // for `(tenant_id, page_id)`. Mirrors the flows seed shape.
        let existing = store
            .get_active(BUNDLED_TENANT, &page.page_id)
            .await
            .map_err(|e| anyhow::anyhow!("dashboards seed probe: {e}"))?;
        if existing.is_some() {
            debug!(page_id = %page.page_id, "dashboards seed: skipped (live row present)");
            continue;
        }

        store
            .insert_revision(page)
            .await
            .map_err(|e| anyhow::anyhow!("dashboards seed insert: {e}"))?;
        inserted += 1;
        // Re-assert the resource kind per insert; the second-plus
        // call returns `DuplicateResource` and is ignored.
        let _ = ensure_dashboard_resource(registry);
    }

    info!(
        target: "rubix.boot",
        inserted,
        "dashboards_definitions seed complete",
    );
    Ok(inserted)
}

/// Collect every bundled SDUI page body. Pages live under
/// `dashboards/<slug>.json` inside [`rubix_flows::BUNDLED`] so the
/// same `include_dir!` tree carries both flows and pages (Phase
/// A.5 will move them into `rubix-flows/dashboards/` proper). For
/// now the helper returns whatever the bundle holds and the
/// seeder copes with zero entries.
fn bundled_pages() -> Vec<NewRevision> {
    let mut out = Vec::new();
    // Phase D.2 moved bundled dashboards to a sibling `dashboards/`
    // dir served by `rubix_flows::BUNDLED_DASHBOARDS`. The old
    // `flows/dashboards/` subtree (kept for the brief window between
    // A.1 and D.2) is still consulted as a fallback so a stray
    // hand-authored test page picks up.
    collect(&rubix_flows::BUNDLED_DASHBOARDS, &mut out);
    if let Some(dir) = rubix_flows::BUNDLED.get_dir("dashboards") {
        collect(dir, &mut out);
    }
    out
}

fn collect(dir: &include_dir::Dir<'_>, out: &mut Vec<NewRevision>) {
    for entry in dir.entries() {
        match entry {
            include_dir::DirEntry::File(f) => {
                let path = f.path().to_string_lossy().into_owned();
                if !path.ends_with(".json") {
                    continue;
                }
                let Ok(body_json) = serde_json::from_slice::<serde_json::Value>(f.contents())
                else {
                    warn!(path = %path, "dashboards seed: skipping non-JSON bundled page");
                    continue;
                };
                // Derive page_id from filename: `dashboards/foo.json`
                // → `dashboard.foo`. Title falls back to the slug.
                let stem = std::path::Path::new(&path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                if stem.is_empty() {
                    continue;
                }
                let page_id = format!("dashboard.{stem}");
                let title = body_json
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&stem)
                    .to_string();
                out.push(NewRevision {
                    page_id,
                    tenant_id: BUNDLED_TENANT.to_string(),
                    owner_principal: BUNDLED_PRINCIPAL.to_string(),
                    title,
                    tags: Vec::new(),
                    body_json,
                    created_by: BUNDLED_PRINCIPAL.to_string(),
                });
            }
            include_dir::DirEntry::Dir(sub) => collect(sub, out),
        }
    }
}
