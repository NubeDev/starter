use anyhow::{Context, Result};

use crate::config::ZenohConfig;
use crate::model::Telemetry;

pub struct ZenohPublisher {
    session: zenoh::Session,
}

impl ZenohPublisher {
    pub async fn connect(config: &ZenohConfig) -> Result<Self> {
        let mut zconfig = zenoh::Config::default();
        let endpoints = serde_json::to_string(&[config.endpoint.as_str()])
            .context("serialize zenoh endpoint")?;
        zconfig
            .insert_json5("mode", "\"client\"")
            .map_err(|error| anyhow::anyhow!("set zenoh mode: {error}"))?;
        zconfig
            .insert_json5("connect/endpoints", &endpoints)
            .map_err(|error| anyhow::anyhow!("set zenoh endpoint: {error}"))?;

        let session = zenoh::open(zconfig)
            .await
            .map_err(|error| anyhow::anyhow!("connect to zenoh: {error}"))?;
        tracing::info!(endpoint = %config.endpoint, "connected to Zenoh router");
        Ok(Self { session })
    }

    pub async fn publish(
        &self,
        path_prefix: &str,
        path_tenant: &str,
        telemetry: &Telemetry,
    ) -> Result<()> {
        let key_expr = telemetry.path(path_prefix, path_tenant);
        let payload = serde_json::to_vec(telemetry).context("serialize telemetry")?;
        self.session
            .put(key_expr, payload)
            .await
            .map_err(|error| anyhow::anyhow!("publish telemetry to zenoh: {error}"))
    }
}
