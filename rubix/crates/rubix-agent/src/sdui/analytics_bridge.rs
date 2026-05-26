//! Adapter from the in-process `rubix.analytics.query` tool to the
//! upstream [`starter_sdui_routes::AnalyticsBridge`] trait the
//! chart-source resolver consumes.
//!
//! Lives here rather than in `starter-sdui-routes` so the upstream
//! crate stays free of the `starter-spi::Tool` dep — the SDUI crate
//! defines the minimal bridge trait; this module bridges it to a
//! concrete `Arc<dyn Tool>` looked up by canonical id.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};
use starter_sdui_routes::AnalyticsBridge;
use starter_spi::tool::Tool;

/// Wraps an `Arc<dyn Tool>` whose `invoke` matches the
/// `rubix.analytics.query` request shape (`{ name, params }`) and
/// returns an `AnalyticsQueryResponse`-shaped value (`{ rows: [...] }`).
pub struct ToolAnalyticsBridge {
    tool: Arc<dyn Tool>,
}

impl ToolAnalyticsBridge {
    /// Wrap the tool registered under `rubix.analytics.query`.
    pub fn new(tool: Arc<dyn Tool>) -> Self {
        Self { tool }
    }
}

#[async_trait]
impl AnalyticsBridge for ToolAnalyticsBridge {
    async fn invoke(
        &self,
        name: &str,
        params: &BTreeMap<String, JsonValue>,
    ) -> Result<Vec<JsonValue>, String> {
        let input = json!({ "name": name, "params": params });
        let resp = self.tool.invoke(input).await.map_err(|e| format!("{e}"))?;
        let rows = resp
            .get("rows")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(rows)
    }
}
