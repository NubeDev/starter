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
use starter_ext_spi::identity::CallerIdentity;
use starter_ext_spi::warehouse::{WarehouseReadRequest, WarehouseReadResponse};
use starter_ext_spi::{Error, ExtensionId, Result};
use starter_ext_supervisor::HostMethodHandler;
use starter_spi::authz::PolicyEngine;
use starter_store_warehouse::WarehouseClient;

use super::backends::{
    authz_kinds_grant, dashboard_read_grant, dashboard_write_grant, event_bus_publish_grant,
    warehouse_grant, RubixWarehouseReadBackend,
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
        }
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
            other => Err(Error::extension_internal(format!(
                "host method {other:?} not implemented"
            ))),
        }
    }
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
}
