//! Wave 3 — capability host-methods: the handler nexus installs so a
//! process-flavour extension can call **back into** nexus (read a dashboard,
//! check authz, run a contributed warehouse query) under the **caller's**
//! tenant, never broader than its grants (WS-14 §4.3).
//!
//! The supervisor runs every inbound `method` through its `CapabilityGate`
//! first: a method whose leading segment (`warehouse`/`authz`/`dashboard`) is
//! not in the extension's declared `capabilities` is refused before it reaches
//! this handler (with a `capability_violations` bump). So by the time
//! [`NexusHostMethods::call`] runs, the *category* is allowed; this handler then
//! enforces the **tenant predicate** — the `caller.tenant_id` clamp — and any
//! finer allowlist, so an extension capability is never broader than the
//! caller's own grants.
//!
//! Caller extraction: the `caller: Option<&CallerIdentity>` argument is the
//! `_meta.caller` sidecar the supervisor threads from the inbound frame. nexus
//! refuses any tenant-scoped method whose caller has no `tenant_id` (the soft
//! trust boundary the kernel docs call out) — a missing tenant is a hard deny,
//! not an implicit "system" escalation.
//!
//! Methods implemented:
//! - `authz.check`  → nexus's `PolicyEngine` under the caller's tenant.
//! - `dashboard.read` → the dashboard store, tenant-clamped + authz-gated.
//! - `warehouse.query` → resolve a contributed/global query-kind and run it
//!   under the caller's tenant via the shared binder + guards.
//! - `ingest.write` → push JSON rows into a named flow's bounded source channel,
//!   tenant stamped from the caller; full channel returns `retry_after_secs`
//!   (see [`super::ingest`]).

use std::sync::Arc;

use async_trait::async_trait;
use starter_ext_spi::authz::{AuthzCheckRequest, AuthzCheckResponse};
use starter_ext_spi::dashboard::{DashboardReadRequest, DashboardReadResponse};
use starter_ext_spi::identity::CallerIdentity;
use starter_ext_spi::warehouse::{Row, WarehouseReadRequest, WarehouseReadResponse};
use starter_ext_spi::{Error as ExtError, ExtensionId, Result as ExtResult};
use starter_ext_supervisor::HostMethodHandler;
use starter_spi::auth::{Principal, Role};

use crate::state::AppState;

/// nexus's [`HostMethodHandler`]. Holds the [`AppState`] so each method can
/// reach the policy engine, the dashboard store, the metadata pool, and the
/// kind registries — exactly the same backends the HTTP routes use, so an
/// extension call is bound by the identical tenancy + authz rules.
pub struct NexusHostMethods {
    state: AppState,
}

impl NexusHostMethods {
    /// Build from the assembled app state. Installed into the supervisor
    /// factory via `WithHostMethodsFactory::new(Arc::new(self))`.
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Shared handle for the supervisor factory.
    pub fn shared(state: AppState) -> Arc<dyn HostMethodHandler> {
        Arc::new(Self::new(state))
    }
}

/// Map the kernel's `CallerIdentity` into a nexus `Principal`, refusing a
/// caller with no tenant. Tenant-scoped host methods cannot run without a
/// tenant clamp — that absence is a hard deny (it would otherwise read across
/// tenants). The role is the highest the caller's role list resolves to;
/// scopes are empty (host-method callers are gated by their extension's
/// capabilities, not fine-grained scopes).
fn principal_from_caller(caller: Option<&CallerIdentity>) -> ExtResult<(Principal, String)> {
    let caller = caller.ok_or_else(|| {
        ExtError::extension_internal("host method requires a caller identity (none supplied)")
    })?;
    let tenant = caller.tenant_id.clone().ok_or_else(|| {
        ExtError::extension_internal("host method requires a tenant-scoped caller (tenant_id is None)")
    })?;
    let role = highest_role(&caller.roles);
    let subject = caller.user_id.clone().unwrap_or_else(|| "system".to_string());
    let principal = Principal {
        subject,
        role,
        scopes: Vec::new(),
        tenant_id: Some(tenant.clone()),
        teams: Vec::new(),
        tenant_scope: Vec::new(),
        extra: serde_json::Value::Null,
    };
    Ok((principal, tenant))
}

/// Reduce a caller's role-string list to the strongest nexus [`Role`]. Unknown
/// strings are ignored; an empty/all-unknown list is the least-privileged
/// `Reader` so a malformed role list can never silently grant write/admin.
fn highest_role(roles: &[String]) -> Role {
    let mut best = Role::Reader;
    for r in roles {
        let parsed = match r.to_ascii_lowercase().as_str() {
            "admin" => Some(Role::Admin),
            "writer" => Some(Role::Writer),
            "reader" => Some(Role::Reader),
            _ => None,
        };
        if let Some(p) = parsed {
            if role_rank(p) > role_rank(best) {
                best = p;
            }
        }
    }
    best
}

fn role_rank(r: Role) -> u8 {
    match r {
        Role::Reader => 0,
        Role::Writer => 1,
        Role::Admin => 2,
    }
}

#[async_trait]
impl HostMethodHandler for NexusHostMethods {
    async fn call(
        &self,
        extension: &ExtensionId,
        method: &str,
        params: serde_json::Value,
        caller: Option<&CallerIdentity>,
    ) -> ExtResult<serde_json::Value> {
        match method {
            "authz.check" => self.authz_check(params, caller).await,
            "dashboard.read" => self.dashboard_read(params, caller).await,
            // The kernel's gate maps the `warehouse` category to `warehouse_read`;
            // the conventional method name is `warehouse.query`.
            "warehouse.query" | "warehouse.read" => self.warehouse_query(extension, params, caller).await,
            // Data-plane: push rows into a named flow source. The host stamps the
            // caller's tenant; a full channel returns a `retry_after_secs`
            // back-pressure response. Gated by the supervisor's `ingest`
            // capability category exactly as `warehouse` is.
            "ingest.write" => super::ingest::write(self.state.flows.ingest(), params, caller),
            other => Err(ExtError::extension_internal(format!(
                "host method {other:?} is not implemented by nexus"
            ))),
        }
    }
}

impl NexusHostMethods {
    /// `authz.check` → run the nexus policy engine for the caller. The resource
    /// is parsed `kind` or `kind:id`; tenancy comes from the caller, never the
    /// extension, so an extension cannot probe another tenant's grants.
    async fn authz_check(
        &self,
        params: serde_json::Value,
        caller: Option<&CallerIdentity>,
    ) -> ExtResult<serde_json::Value> {
        let req: AuthzCheckRequest = serde_json::from_value(params)
            .map_err(|e| ExtError::extension_internal(format!("authz.check params: {e}")))?;
        let (principal, tenant) = principal_from_caller(caller)?;

        let (kind, id) = match req.resource.split_once(':') {
            Some((k, i)) => (k.to_string(), i.to_string()),
            // A kind-only resource (collection-level action) checks against a
            // wildcard id; the engine treats it as the resource class.
            None => (req.resource.clone(), "*".to_string()),
        };

        let allowed =
            crate::authz::can(&*self.state.engine, &principal, &req.action, &kind, &id, &tenant)
                .await;
        serde_json::to_value(AuthzCheckResponse { allowed })
            .map_err(|e| ExtError::extension_internal(format!("authz.check response: {e}")))
    }

    /// `dashboard.read` → read an SDUI page by id, clamped to the caller's
    /// tenant and gated by a `view` authz check (the same gate the HTTP route
    /// applies). A page outside the caller's tenant, or one they may not view,
    /// is an error — never a cross-tenant leak.
    async fn dashboard_read(
        &self,
        params: serde_json::Value,
        caller: Option<&CallerIdentity>,
    ) -> ExtResult<serde_json::Value> {
        let req: DashboardReadRequest = serde_json::from_value(params)
            .map_err(|e| ExtError::extension_internal(format!("dashboard.read params: {e}")))?;
        let (principal, tenant) = principal_from_caller(caller)?;

        // Page id may be a slug; resolve within the caller's tenant.
        let record = nexus_store::dashboard::by_slug(&self.state.metadata, &tenant, &req.page_id)
            .await
            .map_err(|e| ExtError::extension_internal(format!("dashboard.read store: {e}")))?
            .ok_or_else(|| {
                ExtError::extension_internal(format!(
                    "dashboard `{}` not found for tenant",
                    req.page_id
                ))
            })?;

        // Gate on `view` exactly like the route does, by the record's immutable
        // id. A caller who cannot view the page is denied even though it exists
        // in their tenant.
        let allowed = crate::authz::can(
            &*self.state.engine,
            &principal,
            "view",
            "dashboard",
            &record.id.to_string(),
            &tenant,
        )
        .await;
        if !allowed {
            return Err(ExtError::extension_internal(format!(
                "not authorised to view dashboard `{}`",
                req.page_id
            )));
        }

        // `DashboardRecord` is not `Serialize`; project the fields an extension
        // may read into an opaque JSON body. Tenant id is intentionally omitted
        // (the caller already knows their tenant; it is not data to hand back).
        let body = serde_json::json!({
            "id": record.id.to_string(),
            "slug": record.slug,
            "name": record.name,
            "icon": record.icon,
            "accent": record.accent,
            "folder_id": record.folder_id.map(|id| id.to_string()),
            "starred": record.starred,
        });
        serde_json::to_value(DashboardReadResponse { body })
            .map_err(|e| ExtError::extension_internal(format!("dashboard.read response: {e}")))
    }

    /// `warehouse.query` → resolve a contributed/global query-kind by name and
    /// run it under the caller's tenant via the shared binder + guards. The
    /// kind must exist in the **extension-contributed** registry (the third
    /// source): an extension queries through the kinds it (or another global
    /// source) declared, not arbitrary SQL. Tenancy is the caller's; the kind's
    /// `$caller_tenant_id` predicate is bound to it, so the read is clamped.
    async fn warehouse_query(
        &self,
        extension: &ExtensionId,
        params: serde_json::Value,
        caller: Option<&CallerIdentity>,
    ) -> ExtResult<serde_json::Value> {
        let req: WarehouseReadRequest = serde_json::from_value(params)
            .map_err(|e| ExtError::extension_internal(format!("warehouse.query params: {e}")))?;
        let (_principal, tenant) = principal_from_caller(caller)?;

        // Resolve the named kind. Only the global sources (file pack +
        // extension-contributed) are consulted — a host-method caller is not a
        // tenant author, so the tenant overlay is intentionally excluded. The
        // extension's own contributed kind resolves here.
        let bound = match crate::kinds::resolve(&self.state.kinds, &req.template, &req.params) {
            Ok(b) => b,
            Err(crate::kinds::KindError::Unknown(_)) => crate::kinds::resolve(
                &self.state.extension_kinds,
                &req.template,
                &req.params,
            )
            .map_err(|e| {
                ExtError::extension_internal(format!(
                    "warehouse.query: extension `{}` template `{}`: {e}",
                    extension.as_str(),
                    req.template
                ))
            })?,
            Err(e) => {
                return Err(ExtError::extension_internal(format!(
                    "warehouse.query template `{}`: {e}",
                    req.template
                )))
            }
        };

        // Build a query identity clamped to the caller's tenant so the bound
        // `$caller_tenant_id` predicate filters to exactly their rows, and a
        // minimal kind-mode request (the kind's SQL/params are passed
        // explicitly; the request only carries macro/time context, which a
        // host-method warehouse read does not use).
        let identity = nexus_store::QueryIdentity {
            tenant_id: Some(tenant),
            user_id: caller.and_then(|c| c.user_id.clone()),
            // CallerIdentity (the extension host-method frame) carries no
            // team membership, so `$caller_team_ids` binds empty here (P3a).
            teams: Vec::new(),
        };
        let query_req = nexus_spi::dto::query::QueryRequest {
            sql: String::new(),
            time_range: None,
            interval_secs: None,
            variables: Vec::new(),
            kind: Some(req.template.clone()),
            params: Some(req.params.clone()),
            sources: Vec::new(),
            insight: None,
        };
        let response = nexus_store::run_kind_request(
            &self.state.metadata,
            &bound.sql,
            bound.params,
            &query_req,
            &identity,
            self.state.guards,
        )
        .await
        .map_err(|e| ExtError::extension_internal(format!("warehouse.query run: {e}")))?;

        // Each response row is a JSON object keyed by column name; map it into
        // the kernel's `Row` (a JSON map) verbatim. Non-object rows are skipped
        // (the kind contract yields objects, so this is defensive only).
        let rows: Vec<Row> = response
            .rows
            .into_iter()
            .filter_map(|v| match v {
                serde_json::Value::Object(map) => Some(Row::from_map(map)),
                _ => None,
            })
            .collect();
        serde_json::to_value(WarehouseReadResponse { rows })
            .map_err(|e| ExtError::extension_internal(format!("warehouse.query response: {e}")))
    }
}
