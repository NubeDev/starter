//! `TimescaleAnalyticsBridge` — implements
//! [`starter_sdui_routes::AnalyticsBridge`] against the Timescale
//! `samples` hypertable.
//!
//! The four named templates the bundled `data-flow-site-a`
//! dashboard uses are resolved by
//! [`super::template_resolver::resolve`]. That module is the
//! single source of truth shared with
//! [`crate::extensions::backends::RubixWarehouseReadBackend`] —
//! adding or removing a template is a one-place change.
//!
//! Templates outside the resolver's known set return an empty row
//! vector — the upstream resolver then renders the chart / KPI as
//! no-data, which is the same outcome as having no bridge at all.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};
use starter_ext_host::TemplateRegistry;
use starter_sdui_routes::AnalyticsBridge;
use starter_store_warehouse::WarehouseClient;
use tracing::warn;

use super::template_resolver;

/// Concrete bridge backed by a Timescale `samples` hypertable.
///
/// The bridge consults [`TemplateRegistry`] for name resolution — the
/// registry is the audit source of truth across host-builtin
/// templates and (once row 3 lands) extension-contributed ones.
/// Per-template SQL lives in [`super::template_resolver`].
#[derive(Clone)]
pub struct TimescaleAnalyticsBridge {
    client: WarehouseClient,
    registry: Arc<TemplateRegistry>,
}

impl TimescaleAnalyticsBridge {
    /// Construct with the host-builtin template registry. The four
    /// templates the bridge resolves are registered as builtins via
    /// [`TemplateRegistry::builtin`].
    pub fn new(client: WarehouseClient) -> Self {
        Self::with_registry(client, Arc::new(TemplateRegistry::builtin()))
    }

    /// Construct with an explicit registry. Used by tests and by host
    /// integrations that wire extension-contributed templates on top
    /// of the builtin set.
    pub fn with_registry(client: WarehouseClient, registry: Arc<TemplateRegistry>) -> Self {
        Self { client, registry }
    }

    /// Borrow the underlying warehouse client. Surfaced so the
    /// extension-substrate backend factory can share the same pool
    /// without re-plumbing config.
    pub fn client(&self) -> &WarehouseClient {
        &self.client
    }

    /// Borrow the registry. Surfaced for the same reason as
    /// [`Self::client`].
    pub fn registry(&self) -> &Arc<TemplateRegistry> {
        &self.registry
    }
}

#[async_trait]
impl AnalyticsBridge for TimescaleAnalyticsBridge {
    async fn invoke(
        &self,
        name: &str,
        params: &BTreeMap<String, JsonValue>,
    ) -> Result<Vec<JsonValue>, String> {
        // Catalog gate: an unknown template is refused regardless of
        // whether a resolver would match. This makes the
        // `TemplateRegistry` the single audit source of truth —
        // adding a host-builtin template or accepting a contributed
        // one is the only path to a new resolvable name.
        if self.registry.get(name).is_none() {
            warn!(
                target: "rubix.sdui.analytics_bridge",
                template = name,
                "unknown analytics template (not in TemplateRegistry); returning empty",
            );
            return Ok(vec![]);
        }

        let Some(tenant_id) = params.get("tenant_id").and_then(|v| v.as_str()) else {
            return Err(format!("{name}: tenant_id required"));
        };

        // Hand the rest off to the shared resolver. The BTreeMap
        // shape the SDUI layer uses is structurally compatible with
        // a JSON object; serialise once and pass the value through.
        let params_json: JsonValue = json!(params);
        template_resolver::resolve(&self.client, name, tenant_id, &params_json).await
    }
}
