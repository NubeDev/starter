//! DTO ⇄ store conversion for nav nodes, plus dashboard-target validation.
//!
//! The store persists `target`/`context` as opaque JSONB; the typed DTO union
//! (`NavTarget`/`NavContext`) is (de)serialised here. A `dashboard` target's id
//! is validated against a tenant-scoped lookup — never a bare FK — because
//! `nexus_dashboards.id` is a global PK and a node must not point at another
//! tenant's page (WS-13 §4).

use nexus_spi::dto::nav::{NavContext, NavNodeDetail, NavTarget};
use nexus_store::nav_node::NavNodeRecord;
use sqlx::PgPool;
use starter_spi::Error;

/// Render a stored node as the wire DTO. `target` is decoded from JSONB; an
/// unrecognised/legacy shape falls back to `group` rather than failing the whole
/// list (a swept mount is already a group, so this only guards corruption).
pub fn to_detail(rec: &NavNodeRecord) -> NavNodeDetail {
    let target =
        serde_json::from_value::<NavTarget>(rec.target.clone()).unwrap_or(NavTarget::Group);
    let context = rec
        .context
        .as_ref()
        .and_then(|c| serde_json::from_value::<NavContext>(c.clone()).ok());
    NavNodeDetail {
        id: rec.id,
        parent_id: rec.parent_id,
        title: rec.title.clone(),
        sort_order: rec.sort_order,
        target,
        context,
        icon: rec.icon.clone(),
        accent: rec.accent.clone(),
    }
}

/// Serialise a typed target into the JSONB the store holds.
pub fn target_to_json(target: &NavTarget) -> serde_json::Value {
    serde_json::to_value(target).expect("NavTarget serialises")
}

/// Serialise an optional context payload. Context only travels with a
/// `dashboard` target; callers pass `None` for group/route nodes so the column
/// is cleared.
pub fn context_to_json(context: Option<&NavContext>) -> Option<serde_json::Value> {
    context.map(|c| serde_json::to_value(c).expect("NavContext serialises"))
}

/// Validate that, if `target` is a `dashboard` mount, the referenced page exists
/// **within the caller's tenant**. RLS hides other tenants' rows, so a
/// tenant-scoped `by_id` miss covers both "absent" and "foreign" without leaking
/// existence. Group/route targets need no validation. Returns `Invalid` on a
/// dangling/foreign id so the handler surfaces a 4xx, never persists a bad mount.
pub async fn validate_target(
    metadata: &PgPool,
    tenant: &str,
    target: &NavTarget,
) -> Result<(), Error> {
    if let NavTarget::Dashboard { dashboard_id } = target {
        let exists = nexus_store::dashboard::by_id(metadata, tenant, *dashboard_id)
            .await?
            .is_some();
        if !exists {
            return Err(Error::Invalid {
                message: "no such dashboard in this tenant".into(),
            });
        }
    }
    Ok(())
}
