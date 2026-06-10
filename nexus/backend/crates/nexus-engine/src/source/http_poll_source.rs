//! The native `http_poll` source: GET a JSON endpoint on a fixed interval and
//! emit each response as a carrier batch.
//!
//! The native port of [`super::http_poll`] onto the RW-01 [`Source`] trait, same
//! behaviour: the first poll fires immediately, later polls wait the interval,
//! and each response body — a JSON object or array — is emitted as a carrier
//! batch for `json_to_arrow` to shape. An array body becomes one document per
//! element so a scalar and an array response both flow uniformly. It never ends
//! on its own; the flow's cancellation token stops it.

use datafusion::arrow::array::RecordBatch;
use serde::Deserialize;
use serde_json::Value;

use crate::arrow_json::json_carrier_batch;
use crate::core::{EngineError, EngineResult, Source};
use crate::source::interval::parse_cadence;

#[derive(Debug, Clone, Deserialize)]
struct HttpPollConfig {
    /// The endpoint to GET each tick.
    url: String,
    /// Delay between polls, e.g. `"15m"`, `"30s"`.
    interval: String,
    /// Optional bearer token sent as `Authorization: Bearer …`.
    #[serde(default)]
    bearer: Option<String>,
}

/// Polls a JSON endpoint on an interval, emitting each response as a batch.
pub struct HttpPollSource {
    url: String,
    interval: std::time::Duration,
    bearer: Option<String>,
    client: reqwest::Client,
    first: bool,
}

impl HttpPollSource {
    /// Build from the node config, requiring `url` and `interval`.
    pub fn from_config(config: &Value) -> EngineResult<Self> {
        let config: HttpPollConfig = serde_json::from_value(config.clone())
            .map_err(|e| EngineError::Build(format!("invalid http_poll config: {e}")))?;
        let interval = parse_cadence(&config.interval)
            .map_err(|e| EngineError::Build(format!("invalid http_poll interval: {e}")))?;
        Ok(Self {
            url: config.url,
            interval,
            bearer: config.bearer,
            client: reqwest::Client::new(),
            first: true,
        })
    }
}

#[async_trait::async_trait]
impl Source for HttpPollSource {
    async fn read(&mut self) -> EngineResult<Option<RecordBatch>> {
        if self.first {
            self.first = false;
        } else {
            tokio::time::sleep(self.interval).await;
        }
        let mut req = self.client.get(&self.url);
        if let Some(token) = &self.bearer {
            req = req.bearer_auth(token);
        }
        let body: Value = req
            .send()
            .await
            .map_err(|e| EngineError::Source(format!("http poll request failed: {e}")))?
            .json()
            .await
            .map_err(|e| EngineError::Source(format!("http poll body not JSON: {e}")))?;
        Ok(Some(json_carrier_batch(&documents(body))))
    }
}

/// Flatten a response body into JSON-document strings: an array yields one
/// document per element, any other value yields a single document.
fn documents(body: Value) -> Vec<String> {
    match body {
        Value::Array(items) => items.into_iter().map(|v| v.to_string()).collect(),
        other => vec![other.to_string()],
    }
}
