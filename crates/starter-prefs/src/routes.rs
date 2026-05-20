//! REST surface for preferences.
//!
//! Owns: SCOPE.md "API surface" — the four preference endpoints
//! (`GET/PATCH /v1/me/preferences`, `GET/PATCH /v1/orgs/{id}/preferences`)
//! plus the public unit registry endpoint (`GET /v1/units`).
//!
//! Gated behind the `routes` cargo feature (default off) so headless
//! consumers stay axum-free per workspace policy.
//!
//! # Auth contract
//!
//! - `/v1/me/preferences` requires an authenticated [`Principal`]
//!   (any role); the row is keyed on `principal.subject` and the
//!   active workspace.
//! - `/v1/orgs/{id}/preferences` requires `Role::Admin` per the SCOPE
//!   "API surface" block. The check is inline (not a tower layer) so
//!   this crate does not depend on `starter-server`.
//! - `/v1/units` is **public** — it reflects the closed `Quantity` /
//!   `Unit` registry and carries no per-principal data.
//!
//! Wrap the router with `starter_server::auth::with_principal` once
//! before mounting so the [`Principal`] extension is present.
//!
//! # Active workspace
//!
//! `GET /v1/me/preferences?org=<workspace_id>` lets the caller pick
//! the org explicitly. When `org` is absent the handler falls back to
//! `principal.extra["active_workspace"]` (string), and then to the
//! `"@starter/default"` sentinel per SCOPE.md R3 (single-tenant
//! deployments skip the org layer entirely).
//!
//! # PATCH semantics
//!
//! Bodies are parsed as `serde_json::Value` (not `PreferencesPatch`
//! directly) because [`PreferencesPatch`] uses `Option<T>` which
//! collapses `"missing key"` and `"explicit null"` into the same
//! `None`. The route layer must distinguish: missing means *leave
//! alone*, explicit `null` means *revert to inherit* (write SQL
//! `NULL`). Both interpretations are pinned by the Phase 1
//! integration tests below.

use std::sync::{Arc, OnceLock};

use axum::body::{to_bytes, Body};
use axum::extract::{Path, Query, Request};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};
use starter_spi::auth::{Principal, Role};
use starter_spi::dto::Problem;
use starter_spi::preferences::{
    DateFormat, NumberFormat, PreferencesPatch, ResolvedPreferences, Theme, TimeFormat, UnitSystem,
    WeekStart,
};
use starter_spi::units::{Quantity, StaticRegistry, Unit, UnitRegistry};
use utoipa::{OpenApi, ToSchema};

use crate::resolver::{resolve, OrgPrefsRow, StringPref, SystemDefaults, UnitPref, UserPrefsRow};
use crate::store::PrefsStore;

/// Reserved single-tenant workspace sentinel per SCOPE.md R3.
pub const DEFAULT_WORKSPACE: &str = "@starter/default";

/// Maximum PATCH body size. Preferences bodies are tiny (a dozen
/// scalar fields); 8 KiB is comfortably above the realistic ceiling
/// and stops a malicious client from streaming arbitrary JSON.
const MAX_BODY: usize = 8 * 1024;

// ---------------------------------------------------------------------
// Handler state.
// ---------------------------------------------------------------------

/// Shared state for the preference handlers. Cheap to clone — holds
/// an `Arc<dyn PrefsStore>` and an `Arc<SystemDefaults>`.
#[derive(Clone)]
pub struct PrefsRoutesState {
    store: Arc<dyn PrefsStore>,
    defaults: Arc<SystemDefaults>,
}

impl PrefsRoutesState {
    /// Build the state from a store implementation and the
    /// platform-wide [`SystemDefaults`].
    pub fn new(store: Arc<dyn PrefsStore>, defaults: SystemDefaults) -> Self {
        Self {
            store,
            defaults: Arc::new(defaults),
        }
    }
}

// ---------------------------------------------------------------------
// Router.
// ---------------------------------------------------------------------

/// Build the `/v1/{me,orgs,units}/…` preferences router.
///
/// Generic over the consumer's `AppState` so it merges cleanly into
/// the surrounding axum stack (same pattern as
/// `starter_ui_theme::theme_router`).
pub fn prefs_router<S: Clone + Send + Sync + 'static>(state: PrefsRoutesState) -> Router<S> {
    let s_me_get = state.clone();
    let s_me_patch = state.clone();
    let s_org_get = state.clone();
    let s_org_patch = state.clone();
    let s_units = state;

    Router::new()
        .route(
            "/v1/me/preferences",
            get(move |q: Query<OrgQuery>, req: Request<Body>| {
                let s = s_me_get.clone();
                async move { get_me_preferences(s, q, req).await }
            })
            .patch(move |q: Query<OrgQuery>, req: Request<Body>| {
                let s = s_me_patch.clone();
                async move { patch_me_preferences(s, q, req).await }
            }),
        )
        .route(
            "/v1/orgs/{id}/preferences",
            get(move |p: Path<String>, req: Request<Body>| {
                let s = s_org_get.clone();
                async move { get_org_preferences(s, p, req).await }
            })
            .patch(move |p: Path<String>, req: Request<Body>| {
                let s = s_org_patch.clone();
                async move { patch_org_preferences(s, p, req).await }
            }),
        )
        .route(
            "/v1/units",
            get(move || {
                let s = s_units.clone();
                async move { get_units(s).await }
            }),
        )
}

// ---------------------------------------------------------------------
// OpenAPI contribution.
// ---------------------------------------------------------------------

/// utoipa entry point. The consumer merges this into its own document
/// the same way it merges `starter_ui_theme::openapi::openapi()`.
#[derive(OpenApi)]
#[openapi(
    paths(
        get_me_preferences,
        patch_me_preferences,
        get_org_preferences,
        patch_org_preferences,
        get_units,
    ),
    components(schemas(
        ResolvedPreferences,
        PreferencesPatch,
        UnitsDocument,
        QuantityEntry,
        Problem,
    )),
    tags((name = "preferences", description = "User / org preferences + unit registry")),
)]
pub struct PrefsApi;

/// Build the canonical OpenAPI document for this crate's routes.
pub fn openapi() -> utoipa::openapi::OpenApi {
    PrefsApi::openapi()
}

// ---------------------------------------------------------------------
// Query / response DTOs.
// ---------------------------------------------------------------------

/// `?org=<workspace_id>` query for the `/v1/me/preferences` routes.
#[derive(Debug, Default, Deserialize)]
pub struct OrgQuery {
    /// Workspace id; falls back to `principal.extra["active_workspace"]`
    /// then `"@starter/default"`.
    #[serde(default)]
    pub org: Option<String>,
}

/// `/v1/units` payload — the closed registry serialised verbatim.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UnitsDocument {
    /// Per-quantity definition. Stable key order so the ETag is
    /// reproducible across calls.
    pub quantities: Vec<QuantityEntry>,
}

/// One row of [`UnitsDocument::quantities`].
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QuantityEntry {
    /// Wire identifier (e.g. `"temperature"`).
    pub quantity: String,
    /// Canonical SI unit identifier.
    pub canonical: String,
    /// Every unit accepted on the wire for this quantity.
    pub allowed: Vec<String>,
}

// ---------------------------------------------------------------------
// Handlers — `/v1/me/preferences`.
// ---------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/v1/me/preferences",
    tag = "preferences",
    params(
        ("org" = Option<String>, Query, description = "Workspace id; defaults to the active workspace on the Principal"),
    ),
    responses(
        (status = 200, description = "Resolved preferences", body = ResolvedPreferences),
        (status = 401, description = "Authentication required"),
    ),
)]
pub async fn get_me_preferences(
    state: PrefsRoutesState,
    Query(q): Query<OrgQuery>,
    req: Request<Body>,
) -> Response {
    let principal = match principal_or_401(&req) {
        (Some(p), _) => p,
        (_, Some(r)) => return r,
        _ => unreachable!(),
    };
    let workspace = workspace_for(&principal, q.org.as_deref());
    match resolve_for(&state, &principal.subject, &workspace).await {
        Ok(rp) => (StatusCode::OK, Json(rp)).into_response(),
        Err(r) => r,
    }
}

#[utoipa::path(
    patch,
    path = "/v1/me/preferences",
    tag = "preferences",
    params(
        ("org" = Option<String>, Query, description = "Workspace id; defaults to the active workspace on the Principal"),
    ),
    request_body = PreferencesPatch,
    responses(
        (status = 204, description = "Patch applied"),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Authentication required"),
    ),
)]
pub async fn patch_me_preferences(
    state: PrefsRoutesState,
    Query(q): Query<OrgQuery>,
    req: Request<Body>,
) -> Response {
    let principal = match principal_or_401(&req) {
        (Some(p), _) => p,
        (_, Some(r)) => return r,
        _ => unreachable!(),
    };
    let workspace = workspace_for(&principal, q.org.as_deref());

    let patch = match read_patch_object(req).await {
        Ok(p) => p,
        Err(r) => return r,
    };

    let mut row = state
        .store
        .get_user_prefs(&principal.subject, &workspace)
        .await
        .unwrap_or_default()
        .unwrap_or_default();
    if let Err(detail) = apply_user_patch(&mut row, &patch) {
        return bad_request(detail);
    }
    if let Err(e) = state
        .store
        .upsert_user_prefs(&principal.subject, &workspace, row)
        .await
    {
        return internal(e);
    }
    StatusCode::NO_CONTENT.into_response()
}

// ---------------------------------------------------------------------
// Handlers — `/v1/orgs/{id}/preferences`.
// ---------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/v1/orgs/{id}/preferences",
    tag = "preferences",
    params(("id" = String, Path, description = "Workspace id")),
    responses(
        (status = 200, description = "Resolved org-layer preferences", body = ResolvedPreferences),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Admin role required"),
    ),
)]
pub async fn get_org_preferences(
    state: PrefsRoutesState,
    Path(id): Path<String>,
    req: Request<Body>,
) -> Response {
    if let Some(r) = require_admin(&req) {
        return r;
    }
    let org = match state.store.get_org_prefs(&id).await {
        Ok(o) => o,
        Err(e) => return internal(e),
    };
    let resolved = resolve(None, org, &state.defaults);
    (StatusCode::OK, Json(resolved)).into_response()
}

#[utoipa::path(
    patch,
    path = "/v1/orgs/{id}/preferences",
    tag = "preferences",
    params(("id" = String, Path, description = "Workspace id")),
    request_body = PreferencesPatch,
    responses(
        (status = 204, description = "Patch applied"),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Admin role required"),
    ),
)]
pub async fn patch_org_preferences(
    state: PrefsRoutesState,
    Path(id): Path<String>,
    req: Request<Body>,
) -> Response {
    if let Some(r) = require_admin(&req) {
        return r;
    }
    let patch = match read_patch_object(req).await {
        Ok(p) => p,
        Err(r) => return r,
    };
    let mut row = state
        .store
        .get_org_prefs(&id)
        .await
        .unwrap_or_default()
        .unwrap_or_default();
    if let Err(detail) = apply_org_patch(&mut row, &patch) {
        return bad_request(detail);
    }
    if let Err(e) = state.store.upsert_org_prefs(&id, row).await {
        return internal(e);
    }
    StatusCode::NO_CONTENT.into_response()
}

// ---------------------------------------------------------------------
// Handler — `/v1/units`.
// ---------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/v1/units",
    tag = "preferences",
    responses(
        (status = 200, description = "Closed unit registry", body = UnitsDocument),
    ),
)]
pub async fn get_units(_state: PrefsRoutesState) -> Response {
    let (etag, body) = units_payload();
    let mut headers = HeaderMap::new();
    headers.insert(header::ETAG, etag);
    headers.insert(
        "x-platform-version",
        HeaderValue::from_static(env!("CARGO_PKG_VERSION")),
    );
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    (StatusCode::OK, headers, body).into_response()
}

// Cached units payload — the registry is compile-time static so we
// can serialise once and reuse the body + ETag forever.
fn units_payload() -> (HeaderValue, String) {
    static CACHED: OnceLock<(HeaderValue, String)> = OnceLock::new();
    CACHED
        .get_or_init(|| {
            let registry = StaticRegistry::new();
            let mut quantities = Vec::with_capacity(Quantity::ALL.len());
            for q in Quantity::ALL {
                let def = registry
                    .get(*q)
                    .expect("StaticRegistry covers all Quantity");
                quantities.push(QuantityEntry {
                    quantity: q.as_str().to_owned(),
                    canonical: def.canonical.as_str().to_owned(),
                    allowed: def
                        .allowed_units
                        .iter()
                        .map(|u| u.as_str().to_owned())
                        .collect(),
                });
            }
            let doc = UnitsDocument { quantities };
            let body = serde_json::to_string(&doc).expect("UnitsDocument serializes");
            // ETag: stable FNV-1a over the body bytes, hex-encoded.
            // The registry is compile-time static, so the body —
            // and therefore the ETag — never change for a given
            // build; spec quote-wrapping makes it a strong ETag.
            let hash = fnv1a_64(body.as_bytes());
            let etag = format!("\"{hash:016x}\"");
            (
                HeaderValue::from_str(&etag).expect("hex ASCII is valid header"),
                body,
            )
        })
        .clone()
}

fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

// ---------------------------------------------------------------------
// Auth helpers.
// ---------------------------------------------------------------------

/// Pull the [`Principal`] out of the request extensions.
///
/// Returns `(Some(principal), None)` when present, `(None, Some(401))`
/// when not. The two-arm shape sidesteps the `result_large_err` lint
/// that fires when an `axum::Response` rides in `Result::Err`; same
/// pattern as `starter_ui_theme::routes::guards`.
fn principal_or_401<B>(req: &Request<B>) -> (Option<Principal>, Option<Response>) {
    match req.extensions().get::<Principal>().cloned() {
        Some(p) => (Some(p), None),
        None => (None, Some(unauthorized())),
    }
}

/// `None` → caller may proceed; `Some(resp)` → return immediately.
fn require_admin<B>(req: &Request<B>) -> Option<Response> {
    let (Some(p), _) = principal_or_401(req) else {
        return Some(unauthorized());
    };
    if matches!(p.role, Role::Admin) {
        None
    } else {
        Some(forbidden())
    }
}

fn workspace_for(principal: &Principal, query: Option<&str>) -> String {
    if let Some(q) = query {
        if !q.is_empty() {
            return q.to_owned();
        }
    }
    if let Some(active) = principal
        .extra
        .get("active_workspace")
        .and_then(|v| v.as_str())
    {
        if !active.is_empty() {
            return active.to_owned();
        }
    }
    DEFAULT_WORKSPACE.to_owned()
}

async fn resolve_for(
    state: &PrefsRoutesState,
    user_id: &str,
    workspace_id: &str,
) -> Result<ResolvedPreferences, Response> {
    let user = state
        .store
        .get_user_prefs(user_id, workspace_id)
        .await
        .map_err(internal)?;
    let org = state
        .store
        .get_org_prefs(workspace_id)
        .await
        .map_err(internal)?;
    Ok(resolve(user, org, &state.defaults))
}

// ---------------------------------------------------------------------
// PATCH plumbing.
// ---------------------------------------------------------------------

async fn read_patch_object(req: Request<Body>) -> Result<Map<String, JsonValue>, Response> {
    let (_, body) = req.into_parts();
    let bytes = to_bytes(body, MAX_BODY).await.map_err(|_| {
        problem(
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload_too_large",
            "request body exceeds 8 KiB",
            None,
        )
    })?;
    if bytes.is_empty() {
        return Ok(Map::new());
    }
    let value: JsonValue = serde_json::from_slice(&bytes).map_err(|e| {
        problem(
            StatusCode::BAD_REQUEST,
            "invalid_input",
            "could not parse JSON body",
            Some(e.to_string()),
        )
    })?;
    match value {
        JsonValue::Object(map) => Ok(map),
        _ => Err(bad_request("body must be a JSON object".into())),
    }
}

/// Apply a parsed PATCH map to a user-layer row in place.
///
/// Per the route docs, `null` = revert to inherit (set `None`),
/// absent key = leave existing field unchanged, value = set
/// `Some(parsed)`.
fn apply_user_patch(row: &mut UserPrefsRow, patch: &Map<String, JsonValue>) -> Result<(), String> {
    for (key, value) in patch {
        match key.as_str() {
            "timezone" => row.timezone = parse_opt_string_pref(value, key)?,
            "locale" => row.locale = parse_opt_string(value, key)?,
            "language" => row.language = parse_opt_string(value, key)?,
            "unit_system" => row.unit_system = parse_opt_enum::<UnitSystem>(value, key)?,
            "temperature_unit" => row.temperature_unit = parse_opt_unit(value, key)?,
            "pressure_unit" => row.pressure_unit = parse_opt_unit(value, key)?,
            "speed_unit" => row.speed_unit = parse_opt_unit(value, key)?,
            "length_unit" => row.length_unit = parse_opt_unit(value, key)?,
            "mass_unit" => row.mass_unit = parse_opt_unit(value, key)?,
            "date_format" => row.date_format = parse_opt_enum::<DateFormat>(value, key)?,
            "time_format" => row.time_format = parse_opt_enum::<TimeFormat>(value, key)?,
            "week_start" => row.week_start = parse_opt_enum::<WeekStart>(value, key)?,
            "number_format" => row.number_format = parse_opt_enum::<NumberFormat>(value, key)?,
            "currency" => row.currency = parse_opt_string_pref(value, key)?,
            "theme" => row.theme = parse_opt_enum::<Theme>(value, key)?,
            other => return Err(format!("unknown field {other:?}")),
        }
    }
    Ok(())
}

/// Apply a parsed PATCH map to an org-layer row in place. Same shape
/// as [`apply_user_patch`] minus `theme` (org rows have no theme
/// column per the SCOPE Decisions block).
fn apply_org_patch(row: &mut OrgPrefsRow, patch: &Map<String, JsonValue>) -> Result<(), String> {
    for (key, value) in patch {
        match key.as_str() {
            "timezone" => row.timezone = parse_opt_string_pref(value, key)?,
            "locale" => row.locale = parse_opt_string(value, key)?,
            "language" => row.language = parse_opt_string(value, key)?,
            "unit_system" => row.unit_system = parse_opt_enum::<UnitSystem>(value, key)?,
            "temperature_unit" => row.temperature_unit = parse_opt_unit(value, key)?,
            "pressure_unit" => row.pressure_unit = parse_opt_unit(value, key)?,
            "speed_unit" => row.speed_unit = parse_opt_unit(value, key)?,
            "length_unit" => row.length_unit = parse_opt_unit(value, key)?,
            "mass_unit" => row.mass_unit = parse_opt_unit(value, key)?,
            "date_format" => row.date_format = parse_opt_enum::<DateFormat>(value, key)?,
            "time_format" => row.time_format = parse_opt_enum::<TimeFormat>(value, key)?,
            "week_start" => row.week_start = parse_opt_enum::<WeekStart>(value, key)?,
            "number_format" => row.number_format = parse_opt_enum::<NumberFormat>(value, key)?,
            "currency" => row.currency = parse_opt_string_pref(value, key)?,
            "theme" => return Err("org preferences do not carry `theme`".into()),
            other => return Err(format!("unknown field {other:?}")),
        }
    }
    Ok(())
}

fn parse_opt_string(value: &JsonValue, key: &str) -> Result<Option<String>, String> {
    match value {
        JsonValue::Null => Ok(None),
        JsonValue::String(s) => Ok(Some(s.clone())),
        _ => Err(format!("{key:?} must be a string or null")),
    }
}

fn parse_opt_string_pref(value: &JsonValue, key: &str) -> Result<Option<StringPref>, String> {
    match value {
        JsonValue::Null => Ok(None),
        JsonValue::String(s) => Ok(Some(StringPref::parse(s))),
        _ => Err(format!("{key:?} must be a string or null")),
    }
}

fn parse_opt_enum<T: for<'de> Deserialize<'de>>(
    value: &JsonValue,
    key: &str,
) -> Result<Option<T>, String> {
    match value {
        JsonValue::Null => Ok(None),
        other => serde_json::from_value::<T>(other.clone())
            .map(Some)
            .map_err(|e| format!("{key:?}: {e}")),
    }
}

fn parse_opt_unit(value: &JsonValue, key: &str) -> Result<Option<UnitPref>, String> {
    match value {
        JsonValue::Null => Ok(None),
        JsonValue::String(s) if s == "auto" => Ok(Some(UnitPref::Auto)),
        JsonValue::String(_) => serde_json::from_value::<Unit>(value.clone())
            .map(|u| Some(UnitPref::Explicit(u)))
            .map_err(|e| format!("{key:?}: {e}")),
        _ => Err(format!("{key:?} must be a string or null")),
    }
}

// ---------------------------------------------------------------------
// Problem helpers.
// ---------------------------------------------------------------------

fn problem(status: StatusCode, kind: &str, title: &str, detail: Option<String>) -> Response {
    let body = Problem {
        kind: kind.to_owned(),
        title: title.to_owned(),
        detail,
    };
    (status, Json(body)).into_response()
}

fn unauthorized() -> Response {
    problem(
        StatusCode::UNAUTHORIZED,
        "unauthenticated",
        "authentication required",
        None,
    )
}

fn forbidden() -> Response {
    problem(
        StatusCode::FORBIDDEN,
        "forbidden",
        "admin role required",
        None,
    )
}

fn bad_request(detail: String) -> Response {
    problem(
        StatusCode::BAD_REQUEST,
        "invalid_input",
        "invalid preferences patch",
        Some(detail),
    )
}

fn internal<E: std::fmt::Display>(err: E) -> Response {
    tracing::warn!(target: "starter_prefs", error = %err, "prefs store error");
    problem(
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal",
        "internal error",
        None,
    )
}
