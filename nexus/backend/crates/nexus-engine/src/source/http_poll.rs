//! A custom ArkFlow input (`type: http_poll`) that fetches a JSON endpoint on a
//! fixed interval.
//!
//! ArkFlow's built-in `http` input is an *ingress server* — it listens for
//! incoming POSTs. Light ingestion needs the opposite: poll an upstream API
//! (e.g. weather every 15m) and emit each response as a batch. This input does
//! that — `read()` waits the interval, GETs the URL, and returns the body as one
//! JSON message for the pipeline to shape; it never returns `EOF`, so the flow
//! runs until its cancellation token fires.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use arkflow_core::codec::Codec;
use arkflow_core::input::{register_input_builder, Ack, Input, InputBuilder, NoopAck};
use arkflow_core::{Error, MessageBatch, MessageBatchRef, Resource};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;

use crate::time::parse_interval;

#[derive(Debug, Clone, Deserialize)]
struct HttpPollConfig {
    /// The endpoint to GET each tick.
    url: String,
    /// How long to wait between polls, e.g. "15m", "30s".
    interval: String,
    /// Optional bearer token sent as `Authorization: Bearer …`.
    #[serde(default)]
    bearer: Option<String>,
}

struct HttpPollInput {
    url: String,
    interval: Duration,
    bearer: Option<String>,
    client: reqwest::Client,
    /// The first poll fires immediately; subsequent polls wait the interval, so
    /// a flow produces its first batch without waiting a full cycle.
    first: AtomicBool,
}

#[async_trait]
impl Input for HttpPollInput {
    async fn connect(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn read(&self) -> Result<(MessageBatchRef, Arc<dyn Ack>), Error> {
        if !self.first.swap(false, Ordering::SeqCst) {
            tokio::time::sleep(self.interval).await;
        }
        let mut req = self.client.get(&self.url);
        if let Some(token) = &self.bearer {
            req = req.bearer_auth(token);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| Error::Process(format!("http poll request failed: {e}")))?;
        let body: Value = resp
            .json()
            .await
            .map_err(|e| Error::Process(format!("http poll body not JSON: {e}")))?;

        // The pipeline's json_to_arrow processor expects one JSON document per
        // row; wrap a bare object as a single-element batch so a scalar response
        // and an array response both shape uniformly.
        let batch = MessageBatch::from_json(&body)
            .map_err(|e| Error::Process(format!("http poll batch build failed: {e}")))?;
        Ok((batch.into_arc(), Arc::new(NoopAck)))
    }

    async fn close(&self) -> Result<(), Error> {
        Ok(())
    }
}

struct HttpPollInputBuilder;

impl InputBuilder for HttpPollInputBuilder {
    fn build(
        &self,
        _name: Option<&String>,
        config: &Option<Value>,
        _codec: Option<Arc<dyn Codec>>,
        _resource: &Resource,
    ) -> Result<Arc<dyn Input>, Error> {
        let config: HttpPollConfig = config
            .clone()
            .ok_or_else(|| Error::Config("http_poll input requires a url and interval".into()))
            .and_then(|v| {
                serde_json::from_value(v)
                    .map_err(|e| Error::Config(format!("invalid http_poll config: {e}")))
            })?;
        let interval = parse_interval(&config.interval)
            .map_err(|e| Error::Config(format!("invalid http_poll interval: {e}")))?;
        Ok(Arc::new(HttpPollInput {
            url: config.url,
            interval,
            bearer: config.bearer,
            client: reqwest::Client::new(),
            first: AtomicBool::new(true),
        }))
    }
}

/// Register the `http_poll` input type. Called once at startup.
pub fn init() -> Result<(), Error> {
    register_input_builder("http_poll", Arc::new(HttpPollInputBuilder))
}
