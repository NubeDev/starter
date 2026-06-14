//! `POST /api/v1/onboard` — self-service consumer onboarding.
//!
//! The "buy a device → sign up → add a device → get your own workspace" story
//! (NAV_USERS_TEAMS + SETUP_AUTOMATION_BUILDER). A freshly signed-up consumer is
//! a **non-admin**: they cannot call the admin-gated `/v1/tenants/*` or
//! `/v1/authz/*` endpoints that build a workspace. This one route does that
//! privileged work in-process, scoped so the new user sees ONLY their own
//! device.
//!
//! Pre-req: the caller has already created their account via `POST /auth/signup`
//! (which mints the user but binds it to no tenant). This route is then called
//! with that email + the device barcode; it:
//!
//!   1. resolves the signed-up user by email → their subject id,
//!   2. makes them a `reader` member of the `nexus` tenant,
//!   3. creates a **per-user team** (`dev-<slug>`) and adds them — the team is
//!      the scope boundary (`$caller_team_ids`),
//!   4. provisions their device into the extension-owned table, tagged with that
//!      team (idempotent upsert on the barcode-derived device id),
//!   5. creates a **dashboard** + a **nav node** that mounts it,
//!   6. grants the team `view` on BOTH the nav node and the dashboard (the
//!      two-grant rule from NAV_USERS_TEAMS), then reloads the policy engine.
//!
//! After this the user logs in and sees exactly one sidebar entry — their device
//! dashboard — and `devices_list` returns only their team's row.
//!
//! Idempotent by construction: re-onboarding the same email reuses the team
//! (slug conflict is tolerated) and upserts the device, so a retried signup does
//! not double-provision.

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use starter_authz::grants::{GrantStore, GrantSubject, NewGrant};
use starter_authz::instances::PermissionTier;
use starter_server::error::IntoResponse;
use starter_spi::Error;

use crate::authz::{KIND_DASHBOARD, KIND_NAV_NODE};
use crate::state::AppState;

/// `POST /api/v1/onboard` — the self-service consumer onboarding route.
pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/onboard", post(onboard))
}

/// The tenant a self-service consumer is onboarded into. The product runs a
/// single seeded tenant; per-user isolation is by team, not by tenant.
const ONBOARD_TENANT: &str = "nexus";

/// `Error::Invalid` from a message.
fn invalid(msg: impl Into<String>) -> Error {
    Error::Invalid {
        message: msg.into(),
    }
}

/// `Error::Internal` boxing a stringly-typed cause.
fn internal(msg: impl Into<String>) -> Error {
    Error::Internal {
        source: Box::<dyn std::error::Error + Send + Sync>::from(msg.into()),
    }
}

/// Request body: the just-signed-up user's email + the device they're adding.
#[derive(Debug, Deserialize)]
pub struct OnboardRequest {
    /// Email of the account created via `/auth/signup` immediately before.
    pub email: String,
    /// The device barcode the consumer is registering (their "new device").
    pub barcode: String,
    /// Optional human label for the install location.
    #[serde(default)]
    pub location: Option<String>,
}

/// What the new user's workspace looks like — returned so the UI can deep-link
/// straight into the freshly-created dashboard.
#[derive(Debug, Serialize)]
pub struct OnboardResponse {
    pub user_id: String,
    pub team_slug: String,
    pub device_id: String,
    pub dashboard_id: String,
    pub dashboard_slug: String,
    pub nav_node_id: String,
}

/// FNV-1a 64-bit — the SAME stable id the extension child derives, so the device
/// id is a pure function of the barcode (idempotent on re-onboard).
fn stable_id(prefix: &str, key: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in key.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{prefix}-{hash:016x}")
}

/// Short stable suffix for per-user object slugs (team, dashboard).
fn short(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect()
}

#[utoipa::path(
    post,
    path = "/api/v1/onboard",
    tag = "onboard",
    operation_id = "onboard_consumer",
    request_body = inline(OnboardRequestSchema),
    responses(
        (status = 200, description = "Workspace provisioned", body = inline(OnboardResponseSchema)),
        (status = 400, description = "Unknown user / bad input", body = nexus_spi::Problem),
    ),
)]
pub async fn onboard(
    State(state): State<AppState>,
    Json(req): Json<OnboardRequest>,
) -> Result<Json<OnboardResponse>, IntoResponse> {
    let email = req.email.trim().to_lowercase();
    if email.is_empty() || req.barcode.trim().is_empty() {
        return Err(IntoResponse(invalid("email and barcode are required")));
    }

    // 1. Resolve the just-signed-up user → subject id.
    let user = state
        .users
        .find_by_email(&email)
        .await
        .map_err(|e| IntoResponse(internal(format!("user lookup: {e}"))))?
        .ok_or_else(|| {
            IntoResponse(invalid(
                "no such user — sign up first (POST /auth/signup) then onboard",
            ))
        })?;
    let user_id = user.id;

    // 2. Make them a tenant member (reader). Idempotent: re-adding is tolerated.
    let _ = state
        .tenants
        .add_member(&starter_auth_users::store::MembershipRecord {
            tenant_id: ONBOARD_TENANT.to_string(),
            user_id: user_id.clone(),
            role: "reader".to_string(),
            email: Some(email.clone()),
        })
        .await; // best-effort; an existing membership is fine.

    // 3. Per-user team — the scope boundary. Slug is stable per user so a
    //    re-onboard reuses it (slug conflict tolerated).
    let team_slug = format!("dev-{}", short(&user_id));
    let team_id = uuid::Uuid::new_v4().to_string();
    let _ = state
        .tenants
        .create_team(&starter_auth_users::store::TeamRecord {
            id: team_id.clone(),
            tenant_id: ONBOARD_TENANT.to_string(),
            slug: team_slug.clone(),
            display_name: format!("{}'s devices", email),
        })
        .await; // conflict ⇒ team already exists from a prior onboard.

    // Resolve the real team id (the create above may have been a no-op on retry).
    let team_id = resolve_team_id(&state, &team_slug)
        .await
        .map_err(IntoResponse)?
        .unwrap_or(team_id);
    let _ = state.tenants.add_team_member(&team_id, &user_id).await;

    // 4. Provision the device into the extension-owned table, tagged with the
    //    per-user team. Upsert on the barcode-derived id so re-onboard is a no-op.
    let device_id = stable_id("dev", req.barcode.trim());
    let location = req.location.clone().unwrap_or_default();
    upsert_device(
        &state,
        &device_id,
        req.barcode.trim(),
        &location,
        &user_id,
        &team_slug,
    )
    .await
    .map_err(IntoResponse)?;

    // 5. Dashboard + nav node mounting it.
    let dash_slug = format!("devices-{}", short(&user_id));
    let dashboard = nexus_store::dashboard::insert(
        &state.metadata,
        ONBOARD_TENANT,
        &nexus_store::dashboard::NewDashboard {
            slug: dash_slug.clone(),
            name: "My devices".to_string(),
            icon: "cpu".to_string(),
            accent: "152 76% 44%".to_string(),
            folder_id: None,
        },
    )
    .await
    // A re-onboard hits the existing dashboard (slug conflict) → resolve it
    // below instead of failing the whole onboarding.
    .ok();
    let dashboard_id = match dashboard {
        Some(d) => d.id,
        None => resolve_dashboard_id(&state, &dash_slug)
            .await
            .map_err(IntoResponse)?
            .ok_or_else(|| IntoResponse(internal("dashboard create+resolve failed")))?,
    };

    // 5b. A panel on the dashboard so it isn't empty — a table pinned to THIS
    // user's ONE device. It runs in KIND-MODE (`com.acme.devices.device_detail`)
    // with the device id in `kindParams`, NOT the team-scoped list: this
    // dashboard belongs to one device, so it shows exactly that device to anyone
    // authorised to open the page (the owner, and an admin). The kind reads the
    // extension-owned `com_acme_devices__devices` table tenant-scoped via the
    // host tokens. The kind + params are stashed in the panel's opaque `layout`
    // blob (panels have no `kind` column); the UI's `useWidgetQuery` lifts them
    // back out and posts `{ kind, params }` to `POST /api/v1/query`. Idempotent:
    // only insert when the dashboard has no panel yet.
    if !dashboard_has_panel(&state, &dashboard_id)
        .await
        .map_err(IntoResponse)?
    {
        // `fields.series` is REQUIRED for a table to render columns — an empty
        // series list shows "No data" even when the query returns rows (the
        // column→role mapping rides in the opaque `layout` blob, not a DB column;
        // see testing/docs/features/DASHBOARDS.md "GOTCHA"). List every column
        // the kind returns, with `kind` driving cell formatting.
        let layout = serde_json::json!({
            "x": 0, "y": 0, "w": 12, "h": 8,
            "kind": "com.acme.devices.device_detail",
            "kindParams": { "device_id": device_id },
            "fields": {
                "series": [
                    { "value": "barcode",    "label": "Barcode",    "kind": "text" },
                    { "value": "location",   "label": "Location",   "kind": "text" },
                    { "value": "team",       "label": "Team",       "kind": "text" },
                    { "value": "device_id",  "label": "Device ID",  "kind": "text" },
                    { "value": "created_at", "label": "Registered", "kind": "time" },
                ],
            },
        });
        nexus_store::dashboard::panel::insert(
            &state.metadata,
            ONBOARD_TENANT,
            &nexus_store::dashboard::NewPanel {
                dashboard_id,
                datasource_id: None, // kind-mode names its own table
                title: "My devices".to_string(),
                sql: String::new(), // ignored in kind-mode
                viz: "table".to_string(),
                layout,
                insight_id: None,
                insight_params: None,
            },
        )
        .await
        .map_err(IntoResponse)?;
    }

    // Nav nodes have no natural uniqueness, so a blind insert would duplicate on
    // re-onboard. Reuse an existing node that already targets this dashboard;
    // only insert when there is none.
    let nav_node_id = match resolve_nav_for_dashboard(&state, &dashboard_id)
        .await
        .map_err(IntoResponse)?
    {
        Some(existing) => existing,
        None => {
            let target =
                serde_json::to_value(nexus_spi::dto::nav::NavTarget::Dashboard { dashboard_id })
                    .map_err(|e| IntoResponse(internal(format!("nav target: {e}"))))?;
            nexus_store::nav_node::insert(
                &state.metadata,
                ONBOARD_TENANT,
                &nexus_store::nav_node::NewNavNode {
                    parent_id: None,
                    title: "My devices".to_string(),
                    sort_order: 0,
                    target,
                    context: None,
                    icon: Some("cpu".to_string()),
                    accent: None,
                },
            )
            .await
            .map_err(IntoResponse)?
            .id
        }
    };

    // 6. The two grants (nav node + dashboard) to the per-user team, then reload.
    let grants = GrantStore::new(state.policy.store().clone());
    let subject = GrantSubject::Team {
        slug: team_slug.clone(),
    };
    for (kind, id) in [
        (KIND_NAV_NODE, nav_node_id.to_string()),
        (KIND_DASHBOARD, dashboard_id.to_string()),
    ] {
        grants
            .create(
                NewGrant {
                    subject: subject.clone(),
                    resource_kind: kind.to_string(),
                    resource_id: Some(id),
                    tier: PermissionTier::View,
                    tenant_id: ONBOARD_TENANT.to_string(),
                },
                "onboarding",
            )
            .await
            .map_err(|e| IntoResponse(internal(format!("grant {kind}: {e}"))))?;
    }
    state
        .policy
        .reload()
        .await
        .map_err(|e| IntoResponse(internal(format!("engine reload: {e}"))))?;

    Ok(Json(OnboardResponse {
        user_id,
        team_slug,
        device_id,
        dashboard_id: dashboard_id.to_string(),
        dashboard_slug: dash_slug,
        nav_node_id: nav_node_id.to_string(),
    }))
}

/// Upsert one row into the extension-owned `com_acme_devices__devices` table,
/// tenant-stamped, on the same `(tenant_id, device_id)` PK the boot DDL creates.
/// This mirrors what the extension's `warehouse.write` host method does — done
/// here directly so onboarding is one atomic backend step (the consumer is not
/// yet authenticated enough to run the full automation).
async fn upsert_device(
    state: &AppState,
    device_id: &str,
    barcode: &str,
    location: &str,
    owner: &str,
    team: &str,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO com_acme_devices__devices \
            (tenant_id, device_id, barcode, location, owner, team) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (tenant_id, device_id) DO UPDATE SET \
            barcode = EXCLUDED.barcode, location = EXCLUDED.location, \
            owner = EXCLUDED.owner, team = EXCLUDED.team",
    )
    .bind(ONBOARD_TENANT)
    .bind(device_id)
    .bind(barcode)
    .bind(location)
    .bind(owner)
    .bind(team)
    .execute(&state.metadata)
    .await
    .map_err(|e| internal(format!("device upsert: {e}")))?;
    Ok(())
}

async fn resolve_team_id(state: &AppState, slug: &str) -> Result<Option<String>, Error> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM starter_auth_users_teams WHERE tenant_id = $1 AND slug = $2",
    )
    .bind(ONBOARD_TENANT)
    .bind(slug)
    .fetch_optional(&state.metadata)
    .await
    .map_err(|e| internal(format!("team resolve: {e}")))?;
    Ok(row.map(|r| r.0))
}

async fn resolve_dashboard_id(state: &AppState, slug: &str) -> Result<Option<uuid::Uuid>, Error> {
    let row: Option<(uuid::Uuid,)> =
        sqlx::query_as("SELECT id FROM nexus_dashboards WHERE tenant_id = $1 AND slug = $2")
            .bind(ONBOARD_TENANT)
            .bind(slug)
            .fetch_optional(&state.metadata)
            .await
            .map_err(|e| internal(format!("dashboard resolve: {e}")))?;
    Ok(row.map(|r| r.0))
}

/// Whether the dashboard already has at least one panel (re-onboard guard).
async fn dashboard_has_panel(state: &AppState, dashboard_id: &uuid::Uuid) -> Result<bool, Error> {
    let row: Option<(uuid::Uuid,)> = sqlx::query_as(
        "SELECT id FROM nexus_panels WHERE tenant_id = $1 AND dashboard_id = $2 LIMIT 1",
    )
    .bind(ONBOARD_TENANT)
    .bind(dashboard_id)
    .fetch_optional(&state.metadata)
    .await
    .map_err(|e| internal(format!("panel check: {e}")))?;
    Ok(row.is_some())
}

async fn resolve_nav_for_dashboard(
    state: &AppState,
    dashboard_id: &uuid::Uuid,
) -> Result<Option<uuid::Uuid>, Error> {
    let row: Option<(uuid::Uuid,)> = sqlx::query_as(
        "SELECT id FROM nexus_nav_nodes \
         WHERE tenant_id = $1 AND target->>'dashboardId' = $2 LIMIT 1",
    )
    .bind(ONBOARD_TENANT)
    .bind(dashboard_id.to_string())
    .fetch_optional(&state.metadata)
    .await
    .map_err(|e| internal(format!("nav resolve: {e}")))?;
    Ok(row.map(|r| r.0))
}

// Minimal schema stand-ins so the `utoipa::path` macro compiles without
// deriving ToSchema on the real DTOs (they live in this crate already).
#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
pub struct OnboardRequestSchema {
    email: String,
    barcode: String,
    location: Option<String>,
}
#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
pub struct OnboardResponseSchema {
    user_id: String,
    team_slug: String,
    device_id: String,
    dashboard_id: String,
    dashboard_slug: String,
    nav_node_id: String,
}
