//! The feature-gated `zenoh` source: an Eclipse Zenoh subscriber that turns
//! samples on a key-expression into pipeline batches.
//!
//! Zenoh (<https://zenoh.io>) is a pub/sub/query protocol for edge data. This
//! source opens a session, declares a subscriber on a key expression, and emits
//! each received sample's payload as one carrier document for `json_to_arrow` to
//! shape. A JSON payload is forwarded verbatim; a non-JSON payload is wrapped as a
//! single `binary` string column so a heterogeneous key space still flows without
//! a schema break (the schema-stability contract, roadmap §6, then applies to the
//! wrapper shape).
//!
//! Delivery is at-most-once: a plain subscriber has no upstream ack, so
//! [`Source::commit`] stays the default no-op and an in-flight sample dropped on
//! cancellation is not redelivered. Zenoh's reliability/queryable features could
//! make at-least-once cheap; that is a deliberate follow-up, not built here.
//!
//! The source is OFF by default (roadmap §8): it compiles only under the engine's
//! `zenoh` feature, so a default build pulls none of its transitive deps.

use std::time::Duration;

use datafusion::arrow::array::RecordBatch;
use serde::Deserialize;
use serde_json::{json, Value};
use zenoh::pubsub::Subscriber;
use zenoh::sample::Sample;
use zenoh::Session;

use crate::arrow_json::json_carrier_batch;
use crate::core::{EngineError, EngineResult, Source};

/// How long a connectivity probe waits to open a session + scout before giving
/// up, so the datasource "test" path cannot hang on an unreachable router.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Whether the session joins the Zenoh fabric as a `client` (connects to a
/// router) or a `peer` (meshes directly, including in-process for tests).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ZenohMode {
    /// Connect to a router at one of `endpoints`. The default deployment shape.
    #[default]
    Client,
    /// Mesh directly with peers; supports in-process peers for tests with no
    /// external router.
    Peer,
}

/// Config for a `zenoh` source.
#[derive(Debug, Clone, Deserialize)]
pub struct ZenohConfig {
    /// Endpoints to connect/listen on, e.g. `["tcp/127.0.0.1:7447"]`. May be empty
    /// in `peer` mode for an in-process mesh (tests).
    #[serde(default)]
    pub endpoints: Vec<String>,
    /// The key expression to subscribe to, e.g. `"site/**"`.
    pub key_expr: String,
    /// Session mode (`client` default, or `peer`).
    #[serde(default)]
    pub mode: ZenohMode,
}

impl ZenohConfig {
    /// Parse from a node config `Value`, requiring `key_expr`.
    pub fn from_value(config: &Value) -> EngineResult<Self> {
        serde_json::from_value(config.clone())
            .map_err(|e| EngineError::Build(format!("invalid zenoh config: {e}")))
    }

    /// Build the zenoh `Config` for this source: set the mode and, when given,
    /// the connect endpoints. An invalid endpoint string is a build error rather
    /// than a silent drop.
    pub fn to_zenoh_config(&self) -> EngineResult<zenoh::Config> {
        let mut cfg = zenoh::Config::default();
        let mode = match self.mode {
            ZenohMode::Client => "client",
            ZenohMode::Peer => "peer",
        };
        cfg.insert_json5("mode", &format!("\"{mode}\""))
            .map_err(|e| EngineError::Build(format!("zenoh mode rejected: {e}")))?;
        if !self.endpoints.is_empty() {
            let json = serde_json::to_string(&self.endpoints)
                .map_err(|e| EngineError::Build(format!("zenoh endpoints not serializable: {e}")))?;
            cfg.insert_json5("connect/endpoints", &json)
                .map_err(|e| EngineError::Build(format!("zenoh endpoints rejected: {e}")))?;
        }
        Ok(cfg)
    }
}

/// A Zenoh subscriber source. Opens the session and declares the subscriber on
/// first `read`, then yields one batch per received sample. The pipeline's source
/// task already races each `read` against the flow's cancellation token, so the
/// source needs no token of its own; on drop it closes the session, which
/// undeclares the subscriber and releases the transport.
pub struct ZenohSource {
    config: ZenohConfig,
    state: Option<SubscriberState>,
}

/// The live session + subscriber, established lazily so `build` does no I/O.
struct SubscriberState {
    /// Held for its lifetime, not read: the session owns the transport, so it must
    /// outlive the subscriber. Dropping it (on source drop) closes the session and
    /// undeclares the subscriber.
    #[allow(dead_code)]
    session: Session,
    subscriber: Subscriber<zenoh::handlers::FifoChannelHandler<Sample>>,
}

impl ZenohSource {
    /// Build from config. No network I/O here — the session opens on the first
    /// `read` so a build failure is purely config.
    pub fn build(config: &Value) -> EngineResult<Self> {
        Ok(Self {
            config: ZenohConfig::from_value(config)?,
            state: None,
        })
    }

    /// Open the session and declare the subscriber, memoised in `state`.
    async fn ensure_subscriber(&mut self) -> EngineResult<&SubscriberState> {
        if self.state.is_none() {
            let zcfg = self.config.to_zenoh_config()?;
            let session = zenoh::open(zcfg)
                .await
                .map_err(|e| EngineError::Source(format!("zenoh open failed: {e}")))?;
            let subscriber = session
                .declare_subscriber(self.config.key_expr.clone())
                .await
                .map_err(|e| EngineError::Source(format!("zenoh subscribe failed: {e}")))?;
            self.state = Some(SubscriberState {
                session,
                subscriber,
            });
        }
        Ok(self.state.as_ref().expect("state set above"))
    }
}

impl Drop for ZenohSource {
    fn drop(&mut self) {
        // Closing the session undeclares the subscriber and releases the transport.
        // The session's own drop does this; taking it here makes the intent explicit
        // and lets a future cleanup hook hang off the same point.
        self.state.take();
    }
}

#[async_trait::async_trait]
impl Source for ZenohSource {
    async fn read(&mut self) -> EngineResult<Option<RecordBatch>> {
        let state = self.ensure_subscriber().await?;
        match state.subscriber.recv_async().await {
            Ok(sample) => Ok(Some(json_carrier_batch(&[document(&sample)]))),
            // The subscriber's channel closed (session torn down) — clean end.
            Err(_) => Ok(None),
        }
    }
}

/// Render one sample as a JSON-document string: a UTF-8 JSON payload is forwarded
/// verbatim; anything else (non-UTF-8, or UTF-8 that is not valid JSON) is wrapped
/// as `{"binary": "<lossy-utf8>"}` so the stream schema stays stable.
fn document(sample: &Sample) -> String {
    let bytes = sample.payload().to_bytes();
    match std::str::from_utf8(&bytes) {
        Ok(text) if serde_json::from_str::<Value>(text).is_ok() => text.to_string(),
        Ok(text) => json!({ "binary": text }).to_string(),
        Err(_) => json!({ "binary": String::from_utf8_lossy(&bytes) }).to_string(),
    }
}
