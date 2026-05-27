//! Rubix-side [`HostMethodHandler`] — routes capability-gated
//! host calls from process-flavour extensions into the existing
//! Row-5 / warehouse / event-bus backends.
//!
//! Wired into every `Supervisor::start_with` site at boot. Today
//! it handles five methods:
//!
//! - `dashboard.read`   ⇒ [`RubixDashboardBackend::read`]
//! - `dashboard.write`  ⇒ [`RubixDashboardBackend::write`]
//! - `authz.check`      ⇒ [`RubixAuthzBackend::check`]
//! - `warehouse.query`  ⇒ [`RubixWarehouseReadBackend::query`]
//! - `event_bus.publish`⇒ [`RubixEventBusBackend::publish`]
//!
//! Any other method name returns `Error::ExtensionInternal("host
//! method <m> not implemented")`.
//!
//! ## Caller binding + manifest grants
//!
//! Per call we lift the inbound `_meta.caller` into a per-call
//! backend. When an [`ExtensionRegistry`] is attached (via
//! [`Self::with_extension_registry`]), the per-resource manifest
//! gate Slice 12 introduced fires here too — the resolvers
//! ([`super::backends::dashboard_read_grant`] etc.) are shared
//! with the builtin-flavour [`super::RubixCapabilityFactory`] so
//! both flavours enforce the same allowlist shape.
//!
//! The supervisor's `CapabilityGate` enforces the **category**
//! gate (an extension without `requires: [dashboard_read]` cannot
//! call `dashboard.read` at all). The resolvers here add the
//! per-resource layer (an extension that declared
//! `dashboard_read: { pages: [a] }` cannot read page `b`).

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dashboard::DashboardStore;
use starter_ext_host::{ExtensionRegistry, TemplateRegistry};
use starter_ext_sdk::ctx::{AuthzBackend, DashboardBackend, EventBusBackend, WarehouseReadBackend};
use starter_ext_spi::authz::{AuthzCheckRequest, AuthzCheckResponse};
use starter_ext_spi::dashboard::{
    DashboardReadRequest, DashboardReadResponse, DashboardWriteRequest, DashboardWriteResponse,
};
use starter_ext_spi::event_bus::{EventBusPublishRequest, EventBusPublishResponse};
use starter_ext_spi::fs_ext::{FsReadRequest, FsReadResponse};
use starter_ext_spi::http_out::{HttpRequest, HttpResponse};
use starter_ext_spi::identity::CallerIdentity;
use starter_ext_spi::secrets::{SecretsGetRequest, SecretsGetResponse};
use starter_ext_spi::tracing_ext::{TracingEventRequest, TracingEventResponse};
use starter_ext_spi::wall_clock::{WallClockNowResponse};
use starter_ext_spi::warehouse::{WarehouseReadRequest, WarehouseReadResponse};
use starter_ext_spi::{Error, ExtensionId, Result};
use starter_ext_supervisor::HostMethodHandler;
use starter_spi::authz::PolicyEngine;
use starter_spi::secrets::SecretStore;
use starter_store_warehouse::WarehouseClient;

use super::backends::{
    authz_kinds_grant, dashboard_read_grant, dashboard_write_grant, event_bus_publish_grant,
    fs_paths_grant, http_authorities_grant, secrets_prefixes_grant, warehouse_grant,
    RubixWarehouseReadBackend,
};
use super::dashboard_authz::{RubixAuthzBackend, RubixDashboardBackend};
use super::event_bus::{RubixEventBus, RubixEventBusBackend};

/// Concrete [`HostMethodHandler`] for the rubix host. Cheap to
/// clone — every field is `Arc`-backed.
#[derive(Clone)]
pub struct RubixHostMethods {
    dashboard_store: Arc<dyn DashboardStore>,
    authz_engine: Arc<dyn PolicyEngine>,
    /// Optional warehouse plumbing. Set-once via
    /// [`Self::install_warehouse`] (same set-once semantics as
    /// the extension registry). When unset, `warehouse.query`
    /// returns "not implemented" — same posture as a host that
    /// did not configure a warehouse URL.
    warehouse: Arc<std::sync::OnceLock<WarehouseDeps>>,
    /// Optional event-bus plumbing. Set-once via
    /// [`Self::install_event_bus`]. When unset,
    /// `event_bus.publish` returns "not implemented".
    event_bus: Arc<std::sync::OnceLock<Arc<RubixEventBus>>>,
    /// Sealed [`ExtensionRegistry`] used by the manifest-grant
    /// resolvers. Set-once via [`Self::install_extension_registry`]
    /// — boot orders the install *before* the autostart loop spawns
    /// supervisors, so by the time any child issues its first host
    /// call the cell is filled. When unset, the per-resource gate
    /// is skipped (host-internal posture); the supervisor's
    /// category gate still fires.
    extension_registry: Arc<std::sync::OnceLock<Arc<ExtensionRegistry>>>,
    /// Optional secret store. Set-once via
    /// [`Self::install_secret_store`]. When unset, `secrets.get`
    /// refuses with `Error::ExtensionInternal("…not wired")` —
    /// same shape as the warehouse / event-bus posture.
    secrets: Arc<std::sync::OnceLock<Arc<dyn SecretStore>>>,
    /// Reqwest client backing the `http.request` host method.
    /// Built lazily on first call so the cost of TLS setup
    /// doesn't land on every boot for hosts whose extensions
    /// never declare `http_out`.
    http_client: Arc<std::sync::OnceLock<reqwest::Client>>,
}

/// Warehouse plumbing the handler needs to mint a per-call
/// [`RubixWarehouseReadBackend`]. Bundled into one struct so the
/// optional `Option<WarehouseDeps>` field stays a single move.
#[derive(Clone)]
struct WarehouseDeps {
    client: WarehouseClient,
    template_registry: Arc<TemplateRegistry>,
}

impl std::fmt::Debug for RubixHostMethods {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RubixHostMethods")
            .field("has_warehouse", &self.warehouse.get().is_some())
            .field("has_event_bus", &self.event_bus.get().is_some())
            .field(
                "has_extension_registry",
                &self.extension_registry.get().is_some(),
            )
            .field("has_secrets", &self.secrets.get().is_some())
            .field("http_client_built", &self.http_client.get().is_some())
            .finish_non_exhaustive()
    }
}

impl RubixHostMethods {
    /// Build the handler from the host's already-wired Row-5
    /// primitives. Optional capabilities (warehouse, event bus,
    /// extension registry for manifest grants) attach via the
    /// builder methods below.
    pub fn new(
        dashboard_store: Arc<dyn DashboardStore>,
        authz_engine: Arc<dyn PolicyEngine>,
    ) -> Self {
        Self {
            dashboard_store,
            authz_engine,
            warehouse: Arc::new(std::sync::OnceLock::new()),
            event_bus: Arc::new(std::sync::OnceLock::new()),
            extension_registry: Arc::new(std::sync::OnceLock::new()),
            secrets: Arc::new(std::sync::OnceLock::new()),
            http_client: Arc::new(std::sync::OnceLock::new()),
        }
    }

    /// Install a secret store. Set-once. Once installed,
    /// `secrets.get` consults the store after gating on the
    /// `Capability::Secrets { prefixes }` manifest allowlist.
    pub fn install_secret_store(&self, store: Arc<dyn SecretStore>) {
        let _ = self.secrets.set(store);
    }

    /// Install warehouse plumbing. Set-once. Once installed,
    /// `warehouse.query` is dispatched into
    /// [`RubixWarehouseReadBackend`].
    pub fn install_warehouse(
        &self,
        client: WarehouseClient,
        template_registry: Arc<TemplateRegistry>,
    ) {
        let _ = self.warehouse.set(WarehouseDeps {
            client,
            template_registry,
        });
    }

    /// Install the event bus. Set-once. Once installed,
    /// `event_bus.publish` is dispatched into
    /// [`RubixEventBusBackend`].
    pub fn install_event_bus(&self, bus: Arc<RubixEventBus>) {
        let _ = self.event_bus.set(bus);
    }

    /// Install the sealed [`ExtensionRegistry`] for the
    /// manifest-grant resolvers. Set-once: subsequent calls are
    /// silently ignored (no overwrite — the registry is sealed
    /// upstream and only ever moves from "unset" to "set"). Boot
    /// calls this once, before the autostart loop spawns
    /// supervisors.
    ///
    /// Builder-style for ergonomic chaining at construction; the
    /// `&self` shape (rather than `mut self`) lets boot install
    /// after the handler `Arc` has been cloned for the factory.
    pub fn install_extension_registry(&self, registry: Arc<ExtensionRegistry>) {
        // Ignore the result: `set` returns `Err` only if the cell
        // was already filled, which means an earlier install won.
        // The registry is sealed upstream so any double-install
        // would be passing the same Arc; the result is the same.
        let _ = self.extension_registry.set(registry);
    }
}

#[async_trait]
impl HostMethodHandler for RubixHostMethods {
    async fn call(
        &self,
        extension: &ExtensionId,
        method: &str,
        params: serde_json::Value,
        caller: Option<&CallerIdentity>,
    ) -> Result<serde_json::Value> {
        // Cache the registry reference once — each arm hands it to
        // its grant resolver. `OnceLock::get()` returns
        // `Option<&Arc<_>>`; map through to `&ExtensionRegistry`.
        let registry: Option<&ExtensionRegistry> =
            self.extension_registry.get().map(Arc::as_ref);
        match method {
            "dashboard.read" => {
                let req: DashboardReadRequest = serde_json::from_value(params)
                    .map_err(|e| Error::validation(format!("dashboard.read params: {e}")))?;
                let tenant = caller.and_then(|c| c.tenant_id.clone());
                let user = caller.and_then(|c| c.user_id.clone());
                let granted_read = dashboard_read_grant(registry, extension);
                let backend = RubixDashboardBackend::new(
                    self.dashboard_store.clone(),
                    tenant,
                    user,
                    granted_read,
                    // Write grant not needed for read; pass `None` so
                    // the backend doesn't refuse a `write` we never
                    // attempt. The backend gates per-method.
                    None,
                );
                let body =
                    tokio::task::spawn_blocking(move || backend.read(&req.page_id))
                        .await
                        .map_err(|e| Error::extension_internal(format!("join error: {e}")))??;
                Ok(serde_json::to_value(DashboardReadResponse { body })
                    .expect("DashboardReadResponse serialises"))
            }
            "dashboard.write" => {
                let req: DashboardWriteRequest = serde_json::from_value(params)
                    .map_err(|e| Error::validation(format!("dashboard.write params: {e}")))?;
                let tenant = caller.and_then(|c| c.tenant_id.clone());
                let user = caller.and_then(|c| c.user_id.clone());
                let granted_write = dashboard_write_grant(registry, extension);
                let backend = RubixDashboardBackend::new(
                    self.dashboard_store.clone(),
                    tenant,
                    user,
                    None,
                    granted_write,
                );
                tokio::task::spawn_blocking(move || backend.write(&req.page_id, req.body))
                    .await
                    .map_err(|e| Error::extension_internal(format!("join error: {e}")))??;
                Ok(serde_json::to_value(DashboardWriteResponse::default())
                    .expect("DashboardWriteResponse serialises"))
            }
            "authz.check" => {
                let req: AuthzCheckRequest = serde_json::from_value(params)
                    .map_err(|e| Error::validation(format!("authz.check params: {e}")))?;
                let granted_kinds = authz_kinds_grant(registry, extension);
                let backend = RubixAuthzBackend::new(
                    self.authz_engine.clone(),
                    caller,
                    granted_kinds,
                );
                let allowed = tokio::task::spawn_blocking(move || {
                    backend.check(&req.action, &req.resource)
                })
                .await
                .map_err(|e| Error::extension_internal(format!("join error: {e}")))??;
                Ok(serde_json::to_value(AuthzCheckResponse { allowed })
                    .expect("AuthzCheckResponse serialises"))
            }
            "warehouse.query" => {
                let Some(wh) = self.warehouse.get().cloned() else {
                    return Err(Error::extension_internal(
                        "host method \"warehouse.query\" not wired: \
                         no warehouse client attached to RubixHostMethods",
                    ));
                };
                let req: WarehouseReadRequest = serde_json::from_value(params)
                    .map_err(|e| Error::validation(format!("warehouse.query params: {e}")))?;
                let tenant = caller.and_then(|c| c.tenant_id.clone());
                let granted_tables = warehouse_grant(registry, extension);
                let backend = RubixWarehouseReadBackend::new(
                    wh.client,
                    wh.template_registry,
                    tenant,
                    granted_tables,
                );
                let rows =
                    tokio::task::spawn_blocking(move || backend.query(&req.template, req.params))
                        .await
                        .map_err(|e| Error::extension_internal(format!("join error: {e}")))??;
                Ok(serde_json::to_value(WarehouseReadResponse { rows })
                    .expect("WarehouseReadResponse serialises"))
            }
            "event_bus.publish" => {
                let Some(bus) = self.event_bus.get().cloned() else {
                    return Err(Error::extension_internal(
                        "host method \"event_bus.publish\" not wired: \
                         no event bus attached to RubixHostMethods",
                    ));
                };
                let req: EventBusPublishRequest = serde_json::from_value(params)
                    .map_err(|e| Error::validation(format!("event_bus.publish params: {e}")))?;
                // The event-bus backend uses the calling extension's
                // reverse-DNS id as its namespace — same shape as
                // the builtin-flavour factory in
                // `RubixCapabilityFactory::event_bus`. The
                // per-topic manifest grant fires *before* the
                // namespace check (see `event_bus.rs`).
                let namespace = Some(extension.as_str().to_owned());
                let granted = event_bus_publish_grant(registry, extension);
                let backend = RubixEventBusBackend::with_grant(bus, namespace, granted);
                tokio::task::spawn_blocking(move || backend.publish(&req.topic, req.payload))
                    .await
                    .map_err(|e| Error::extension_internal(format!("join error: {e}")))??;
                Ok(serde_json::to_value(EventBusPublishResponse::default())
                    .expect("EventBusPublishResponse serialises"))
            }
            "secrets.get" => {
                let Some(store) = self.secrets.get().cloned() else {
                    return Err(Error::extension_internal(
                        "host method \"secrets.get\" not wired: \
                         no secret store attached to RubixHostMethods",
                    ));
                };
                let req: SecretsGetRequest = serde_json::from_value(params)
                    .map_err(|e| Error::validation(format!("secrets.get params: {e}")))?;
                // Manifest gate: refuse if the requested name has
                // no matching prefix in the extension's grant. An
                // unset registry (host-internal posture) skips the
                // gate; an extension with no `Capability::Secrets`
                // arm at all gets `Some(empty)` ⇒ every read
                // refused.
                if let Some(prefixes) = secrets_prefixes_grant(registry, extension) {
                    let name = req.name.as_str();
                    let allowed = prefixes.iter().any(|p| name.starts_with(p.as_str()));
                    if !allowed {
                        return Err(Error::capability(format!(
                            "secrets.get {name:?} refused: not under any prefix in the calling \
                             extension's `secrets.prefixes` grant"
                        )));
                    }
                }
                let name = req.name.clone();
                let outcome = tokio::task::spawn_blocking(move || store.get(&name))
                    .await
                    .map_err(|e| Error::extension_internal(format!("join error: {e}")))?;
                let secret = match outcome {
                    Ok(Some(s)) => s,
                    Ok(None) => {
                        return Err(Error::extension_internal(format!(
                            "secrets.get {:?}: no such secret",
                            req.name
                        )))
                    }
                    Err(e) => {
                        return Err(Error::extension_internal(format!(
                            "secrets.get {:?} backend error: {e}",
                            req.name
                        )))
                    }
                };
                Ok(serde_json::to_value(SecretsGetResponse {
                    value: secret.into_inner(),
                })
                .expect("SecretsGetResponse serialises"))
            }
            "wall_clock.now_ms" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                Ok(serde_json::to_value(WallClockNowResponse { now_unix_ms: now })
                    .expect("WallClockNowResponse serialises"))
            }
            "tracing.event" => {
                let req: TracingEventRequest = serde_json::from_value(params)
                    .map_err(|e| Error::validation(format!("tracing.event params: {e}")))?;
                // Map onto the host's `tracing` crate at the
                // appropriate level. Unknown levels default to
                // `info` — same shape `tracing-subscriber`'s
                // env-filter parse uses.
                let ext_id = extension.as_str();
                match req.level.as_str() {
                    "trace" => tracing::trace!(
                        target: "extension",
                        extension = ext_id,
                        msg = %req.msg,
                        fields = ?req.fields,
                    ),
                    "debug" => tracing::debug!(
                        target: "extension",
                        extension = ext_id,
                        msg = %req.msg,
                        fields = ?req.fields,
                    ),
                    "warn" => tracing::warn!(
                        target: "extension",
                        extension = ext_id,
                        msg = %req.msg,
                        fields = ?req.fields,
                    ),
                    "error" => tracing::error!(
                        target: "extension",
                        extension = ext_id,
                        msg = %req.msg,
                        fields = ?req.fields,
                    ),
                    _ => tracing::info!(
                        target: "extension",
                        extension = ext_id,
                        msg = %req.msg,
                        fields = ?req.fields,
                    ),
                }
                Ok(serde_json::to_value(TracingEventResponse::default())
                    .expect("TracingEventResponse serialises"))
            }
            "http.request" => {
                let req: HttpRequest = serde_json::from_value(params)
                    .map_err(|e| Error::validation(format!("http.request params: {e}")))?;
                // Manifest authority gate. Extract `host[:port]`
                // from the URL and match against the allowlist.
                // An unset registry skips the gate (host-internal
                // posture); an `Some(empty)` neutralised grant
                // refuses every call.
                let url = reqwest::Url::parse(&req.url).map_err(|e| {
                    Error::validation(format!("http.request url is not a valid URL: {e}"))
                })?;
                let req_authority = format_authority(&url).ok_or_else(|| {
                    Error::validation(format!(
                        "http.request url {:?} has no host component",
                        req.url
                    ))
                })?;
                if let Some(grant) = http_authorities_grant(registry, extension) {
                    if !grant.contains(&req_authority) {
                        return Err(Error::capability(format!(
                            "http.request {req_authority:?} refused: not in calling \
                             extension's `http_out.authorities` grant"
                        )));
                    }
                }
                // Lazy-build the reqwest client. `Client::new()`
                // panics only on egregious TLS misconfig (which
                // would manifest at boot in production); fall
                // back to the default on first call here.
                let client = self
                    .http_client
                    .get_or_init(reqwest::Client::new)
                    .clone();
                // Whitelist canonical HTTP verbs — `reqwest::Method`'s
                // own parser admits any valid HTTP token (e.g.
                // `"BOGUS"`), and we want stricter validation at the
                // wire boundary so extensions get a clear local
                // error instead of a server-side 405.
                let method = match req.method.to_ascii_uppercase().as_str() {
                    "GET" => reqwest::Method::GET,
                    "HEAD" => reqwest::Method::HEAD,
                    "POST" => reqwest::Method::POST,
                    "PUT" => reqwest::Method::PUT,
                    "DELETE" => reqwest::Method::DELETE,
                    "PATCH" => reqwest::Method::PATCH,
                    "OPTIONS" => reqwest::Method::OPTIONS,
                    other => {
                        return Err(Error::validation(format!(
                            "http.request method {other:?} is not a known HTTP verb"
                        )));
                    }
                };
                let mut builder = client.request(method, url);
                for (name, value) in &req.headers {
                    builder = builder.header(name, value);
                }
                if let Some(body) = req.body {
                    builder = builder.json(&body);
                }
                let response = builder.send().await.map_err(|e| {
                    Error::extension_internal(format!("http.request transport: {e}"))
                })?;
                let status = response.status().as_u16();
                let resp_headers: Vec<(String, String)> = response
                    .headers()
                    .iter()
                    .filter_map(|(k, v)| v.to_str().ok().map(|s| (k.as_str().to_owned(), s.to_owned())))
                    .collect();
                // Try JSON first; fall back to a UTF-8 string;
                // null for empty.
                let bytes = response.bytes().await.map_err(|e| {
                    Error::extension_internal(format!("http.request body read: {e}"))
                })?;
                let body = if bytes.is_empty() {
                    serde_json::Value::Null
                } else if let Ok(j) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    j
                } else {
                    serde_json::Value::String(String::from_utf8_lossy(&bytes).into_owned())
                };
                Ok(serde_json::to_value(HttpResponse {
                    status,
                    headers: resp_headers,
                    body,
                })
                .expect("HttpResponse serialises"))
            }
            "fs.read" => {
                let req: FsReadRequest = serde_json::from_value(params)
                    .map_err(|e| Error::validation(format!("fs.read params: {e}")))?;
                // Manifest path gate. v0.1 uses a simple
                // prefix match — a path is admitted if any
                // declared `paths[i]` is a prefix of the
                // requested path. A future slice can layer in
                // glob support and read-only / read-write
                // distinctions; the path-spec string already
                // carries the room for that contract
                // (`starter-ext-host` resolves the actual
                // disk path).
                if let Some(grant) = fs_paths_grant(registry, extension) {
                    let admitted = grant.iter().any(|p| req.path.starts_with(p.as_str()));
                    if !admitted {
                        return Err(Error::capability(format!(
                            "fs.read {:?} refused: not under any prefix in the calling \
                             extension's `fs.paths` grant",
                            req.path
                        )));
                    }
                }
                let bytes = tokio::fs::read(&req.path).await.map_err(|e| {
                    Error::extension_internal(format!("fs.read {:?}: {e}", req.path))
                })?;
                use base64::Engine;
                let bytes_b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                Ok(serde_json::to_value(FsReadResponse { bytes_b64 })
                    .expect("FsReadResponse serialises"))
            }
            other => Err(Error::extension_internal(format!(
                "host method {other:?} not implemented"
            ))),
        }
    }
}

/// Format the `host[:port]` authority of a parsed URL. Returns
/// `None` when the URL has no host (e.g. `file:///...`). Matches
/// the manifest's [`starter_ext_spi::capability::Authority`]
/// string shape exactly so a manifest entry like
/// `"api.example.com:443"` lines up against `https://api.example.com/`.
///
/// Note the implicit-default-port behaviour: when the URL omits
/// the port (the common case for `https://`), the formatted
/// authority is just the host with no `:443` suffix — operators
/// should declare authorities the same way the URL is written.
fn format_authority(url: &reqwest::Url) -> Option<String> {
    let host = url.host_str()?;
    Some(match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait as async_trait_alias;
    use rubix_spi::dashboard::{
        DashboardRevision, DashboardStoreError, InsertOutcome, ListFilter, NewRevision,
    };
    use starter_spi::auth::Principal;
    use starter_spi::authz::{Decision, ResourceRef};

    #[derive(Default)]
    struct MemStore {
        inner: std::sync::Mutex<std::collections::HashMap<(String, String), serde_json::Value>>,
    }

    #[async_trait_alias]
    impl DashboardStore for MemStore {
        async fn insert_revision(
            &self,
            new: NewRevision,
        ) -> std::result::Result<DashboardRevision, DashboardStoreError> {
            self.inner.lock().unwrap().insert(
                (new.tenant_id.clone(), new.page_id.clone()),
                new.body_json.clone(),
            );
            Ok(DashboardRevision {
                page_id: new.page_id,
                revision_id: "r-1".into(),
                tenant_id: new.tenant_id,
                owner_principal: new.owner_principal,
                title: new.title,
                tags: new.tags,
                body_json: new.body_json,
                created_by: new.created_by,
                created_at: "1970-01-01T00:00:00Z".into(),
                superseded_at: None,
            })
        }
        async fn insert_revision_with_prior(
            &self,
            new: NewRevision,
        ) -> std::result::Result<InsertOutcome, DashboardStoreError> {
            let inserted = self.insert_revision(new).await?;
            Ok(InsertOutcome {
                inserted,
                prior: None,
            })
        }
        async fn get_active(
            &self,
            tenant_id: &str,
            page_id: &str,
        ) -> std::result::Result<Option<DashboardRevision>, DashboardStoreError> {
            let map = self.inner.lock().unwrap();
            Ok(map
                .get(&(tenant_id.to_owned(), page_id.to_owned()))
                .map(|body| DashboardRevision {
                    page_id: page_id.to_owned(),
                    revision_id: "r-1".into(),
                    tenant_id: tenant_id.to_owned(),
                    owner_principal: "u-1".into(),
                    title: String::new(),
                    tags: Vec::new(),
                    body_json: body.clone(),
                    created_by: "u-1".into(),
                    created_at: "1970-01-01T00:00:00Z".into(),
                    superseded_at: None,
                }))
        }
        async fn list_active(
            &self,
            _: &str,
            _: &ListFilter,
        ) -> std::result::Result<Vec<DashboardRevision>, DashboardStoreError> {
            Ok(Vec::new())
        }
        async fn mark_superseded(
            &self,
            _: &str,
            _: &str,
        ) -> std::result::Result<u64, DashboardStoreError> {
            Ok(0)
        }
        async fn history(
            &self,
            _: &str,
        ) -> std::result::Result<Vec<DashboardRevision>, DashboardStoreError> {
            Ok(Vec::new())
        }
    }

    struct AllowEngine;
    #[async_trait_alias]
    impl PolicyEngine for AllowEngine {
        async fn check(&self, _: &Principal, _: &str, _: &ResourceRef) -> Decision {
            Decision::allow()
        }
    }

    fn caller(tenant: &str, user: &str) -> CallerIdentity {
        CallerIdentity {
            tenant_id: Some(tenant.into()),
            user_id: Some(user.into()),
            roles: vec!["Reader".into()],
            request_id: String::new(),
        }
    }

    fn ext() -> ExtensionId {
        ExtensionId::new("com.acme.cap").unwrap()
    }

    fn handler() -> RubixHostMethods {
        let store: Arc<dyn DashboardStore> = Arc::new(MemStore::default());
        let engine: Arc<dyn PolicyEngine> = Arc::new(AllowEngine);
        RubixHostMethods::new(store, engine)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dashboard_write_then_read_round_trips_over_host_method() {
        let h = handler();
        let c = caller("t-1", "u-1");

        let write_params = serde_json::to_value(DashboardWriteRequest {
            page_id: "p1".into(),
            body: serde_json::json!({"k": 1}),
        })
        .unwrap();
        let _ = h
            .call(&ext(), "dashboard.write", write_params, Some(&c))
            .await
            .expect("write");
        let read_params = serde_json::to_value(DashboardReadRequest {
            page_id: "p1".into(),
        })
        .unwrap();
        let res = h
            .call(&ext(), "dashboard.read", read_params, Some(&c))
            .await
            .expect("read");
        let parsed: DashboardReadResponse = serde_json::from_value(res).unwrap();
        assert_eq!(parsed.body, serde_json::json!({"k": 1}));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dashboard_read_refuses_system_frame() {
        let h = handler();
        let read_params = serde_json::to_value(DashboardReadRequest {
            page_id: "p1".into(),
        })
        .unwrap();
        let err = h
            .call(&ext(), "dashboard.read", read_params, None)
            .await
            .expect_err("system frame must refuse");
        assert!(matches!(err, Error::Capability(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn authz_check_returns_allow() {
        let h = handler();
        let c = caller("t-1", "u-1");
        let params = serde_json::to_value(AuthzCheckRequest {
            action: "view".into(),
            resource: "rubix.dashboard.page:p1".into(),
        })
        .unwrap();
        let res = h
            .call(&ext(), "authz.check", params, Some(&c))
            .await
            .expect("check");
        let parsed: AuthzCheckResponse = serde_json::from_value(res).unwrap();
        assert!(parsed.allowed);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unknown_method_is_not_implemented() {
        let h = handler();
        let err = h
            .call(&ext(), "totally.bogus", serde_json::Value::Null, None)
            .await
            .expect_err("unknown method refuses");
        assert!(matches!(err, Error::ExtensionInternal(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn warehouse_query_not_wired_without_warehouse_dep() {
        let h = handler();
        let c = caller("t-1", "u-1");
        let params = serde_json::to_value(WarehouseReadRequest {
            template: "samples_window".into(),
            params: serde_json::Value::Null,
        })
        .unwrap();
        let err = h
            .call(&ext(), "warehouse.query", params, Some(&c))
            .await
            .expect_err("not wired without warehouse dep");
        assert!(matches!(err, Error::ExtensionInternal(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn event_bus_publish_not_wired_without_bus_dep() {
        let h = handler();
        let c = caller("t-1", "u-1");
        let params = serde_json::to_value(EventBusPublishRequest {
            topic: "com.acme.cap.tick".into(),
            payload: serde_json::Value::Null,
        })
        .unwrap();
        let err = h
            .call(&ext(), "event_bus.publish", params, Some(&c))
            .await
            .expect_err("not wired without event-bus dep");
        assert!(matches!(err, Error::ExtensionInternal(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn event_bus_publish_round_trips_when_wired() {
        let store: Arc<dyn DashboardStore> = Arc::new(MemStore::default());
        let engine: Arc<dyn PolicyEngine> = Arc::new(AllowEngine);
        let bus = Arc::new(RubixEventBus::new());
        let h = RubixHostMethods::new(store, engine);
        h.install_event_bus(bus);
        let c = caller("t-1", "u-1");
        let params = serde_json::to_value(EventBusPublishRequest {
            topic: "com.acme.cap.tick".into(),
            payload: serde_json::json!({"n": 1}),
        })
        .unwrap();
        let res = h
            .call(&ext(), "event_bus.publish", params, Some(&c))
            .await
            .expect("publish ok");
        // Response is the default-empty struct.
        let _parsed: EventBusPublishResponse = serde_json::from_value(res).unwrap();
    }

    // ---------- secrets / wall_clock / tracing ----------

    /// Trivial in-memory `SecretStore` for the new arm tests.
    #[derive(Default)]
    struct MemSecretStore {
        inner: std::sync::Mutex<std::collections::HashMap<String, String>>,
    }

    impl starter_spi::secrets::SecretStore for MemSecretStore {
        fn ready(&self) -> bool {
            true
        }
        fn get(
            &self,
            name: &str,
        ) -> std::result::Result<
            Option<starter_spi::secrets::Secret>,
            starter_spi::secrets::SecretError,
        > {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .get(name)
                .cloned()
                .map(starter_spi::secrets::Secret::new))
        }
        fn put(
            &self,
            name: &str,
            value: starter_spi::secrets::Secret,
        ) -> std::result::Result<(), starter_spi::secrets::SecretError> {
            self.inner
                .lock()
                .unwrap()
                .insert(name.to_owned(), value.into_inner());
            Ok(())
        }
        fn delete(
            &self,
            name: &str,
        ) -> std::result::Result<(), starter_spi::secrets::SecretError> {
            self.inner.lock().unwrap().remove(name);
            Ok(())
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn secrets_get_not_wired_without_store() {
        let h = handler();
        let params = serde_json::to_value(SecretsGetRequest {
            name: "ai:anthropic:api_key".into(),
        })
        .unwrap();
        let err = h
            .call(&ext(), "secrets.get", params, None)
            .await
            .expect_err("not wired without store");
        assert!(matches!(err, Error::ExtensionInternal(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn secrets_get_round_trips_when_wired_and_in_grant() {
        // The test uses no `ExtensionRegistry`, so the per-prefix
        // gate is skipped — the store lookup itself is what runs.
        let dash_store: Arc<dyn DashboardStore> = Arc::new(MemStore::default());
        let engine: Arc<dyn PolicyEngine> = Arc::new(AllowEngine);
        let h = RubixHostMethods::new(dash_store, engine);
        // Pre-seed a value via a fresh secret store.
        let secrets: Arc<MemSecretStore> = Arc::new(MemSecretStore::default());
        starter_spi::secrets::SecretStore::put(
            secrets.as_ref(),
            "ai:anthropic:api_key",
            starter_spi::secrets::Secret::new("hunter2"),
        )
        .expect("seed ok");
        h.install_secret_store(secrets);

        let params = serde_json::to_value(SecretsGetRequest {
            name: "ai:anthropic:api_key".into(),
        })
        .unwrap();
        let res = h
            .call(&ext(), "secrets.get", params, None)
            .await
            .expect("get ok");
        let parsed: SecretsGetResponse = serde_json::from_value(res).unwrap();
        assert_eq!(parsed.value, "hunter2");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn secrets_get_missing_key_is_not_found_shape() {
        let dash_store: Arc<dyn DashboardStore> = Arc::new(MemStore::default());
        let engine: Arc<dyn PolicyEngine> = Arc::new(AllowEngine);
        let h = RubixHostMethods::new(dash_store, engine);
        h.install_secret_store(Arc::new(MemSecretStore::default()));
        let params = serde_json::to_value(SecretsGetRequest {
            name: "no.such.secret".into(),
        })
        .unwrap();
        let err = h
            .call(&ext(), "secrets.get", params, None)
            .await
            .expect_err("missing key");
        assert!(matches!(err, Error::ExtensionInternal(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn wall_clock_now_returns_recent_ms() {
        let h = handler();
        let params = serde_json::to_value(
            starter_ext_spi::wall_clock::WallClockNowRequest::default(),
        )
        .unwrap();
        let res = h
            .call(&ext(), "wall_clock.now_ms", params, None)
            .await
            .expect("now ok");
        let parsed: WallClockNowResponse = serde_json::from_value(res).unwrap();
        // Sanity: timestamps from this decade.
        assert!(parsed.now_unix_ms > 1_500_000_000_000);
    }

    // ---------- http_out + fs ----------

    #[tokio::test(flavor = "multi_thread")]
    async fn http_request_refuses_invalid_url() {
        let h = handler();
        let params = serde_json::to_value(HttpRequest {
            method: "GET".into(),
            url: "not-a-url".into(),
            headers: vec![],
            body: None,
        })
        .unwrap();
        let err = h
            .call(&ext(), "http.request", params, None)
            .await
            .expect_err("invalid url");
        assert!(matches!(err, Error::Validation(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_request_refuses_unknown_method() {
        let h = handler();
        let params = serde_json::to_value(HttpRequest {
            method: "BOGUS".into(),
            url: "https://example.com".into(),
            headers: vec![],
            body: None,
        })
        .unwrap();
        let err = h
            .call(&ext(), "http.request", params, None)
            .await
            .expect_err("bad method");
        assert!(matches!(err, Error::Validation(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn fs_read_round_trips() {
        let h = handler();
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        std::fs::write(tmp.path(), b"hello-fs").unwrap();
        let params = serde_json::to_value(FsReadRequest {
            path: tmp.path().to_string_lossy().into_owned(),
        })
        .unwrap();
        let res = h
            .call(&ext(), "fs.read", params, None)
            .await
            .expect("fs.read ok");
        let parsed: FsReadResponse = serde_json::from_value(res).unwrap();
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(parsed.bytes_b64.as_bytes())
            .unwrap();
        assert_eq!(bytes, b"hello-fs");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn fs_read_missing_file_is_internal_error() {
        let h = handler();
        let params = serde_json::to_value(FsReadRequest {
            path: "/definitely/not/here-abc-xyz".into(),
        })
        .unwrap();
        let err = h
            .call(&ext(), "fs.read", params, None)
            .await
            .expect_err("missing file");
        assert!(matches!(err, Error::ExtensionInternal(_)), "got {err:?}");
    }

    #[test]
    fn format_authority_with_and_without_port() {
        use reqwest::Url;
        assert_eq!(
            super::format_authority(&Url::parse("https://api.example.com/x").unwrap())
                .as_deref(),
            Some("api.example.com")
        );
        assert_eq!(
            super::format_authority(&Url::parse("https://api.example.com:8443/x").unwrap())
                .as_deref(),
            Some("api.example.com:8443")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn tracing_event_returns_empty_response() {
        let h = handler();
        let params = serde_json::to_value(TracingEventRequest {
            level: "info".into(),
            msg: "hello".into(),
            fields: serde_json::json!({"k": "v"}),
        })
        .unwrap();
        let res = h
            .call(&ext(), "tracing.event", params, None)
            .await
            .expect("event ok");
        let _: TracingEventResponse = serde_json::from_value(res).unwrap();
    }
}
