//! A custom ArkFlow input (`type: simulator`) that emits synthetic device
//! telemetry on a fixed interval — test data for exercising the ingestion path
//! without a real upstream.
//!
//! Three device *profiles* cover distinct data shapes so a `simulator →
//! postgres` flow stresses different column types end-to-end:
//!
//! - `hvac`   — numeric floats (`temp_c`, `setpoint`, `fan_speed`)
//! - `energy` — a monotonic counter (`kwh_total`) plus instantaneous `power_w`
//! - `door`   — a discrete `open: bool` plus a `zone: str` state
//!
//! Like [`super::http_poll`] it never returns `EOF`: `read()` waits the
//! interval, builds one row for the configured profile, and returns it as a
//! single-element batch for the pipeline to shape. Output is deterministic for a
//! given `seed`, so tests get repeatable data.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use arkflow_core::codec::Codec;
use arkflow_core::input::{register_input_builder, Ack, Input, InputBuilder, NoopAck};
use arkflow_core::{Error, MessageBatch, MessageBatchRef, Resource};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;

use crate::source::sim::{self, Profile};
use crate::time::parse_interval;

#[derive(Debug, Clone, Deserialize)]
struct SimulatorConfig {
    /// Which device shape to emit: `hvac`, `energy`, or `door`.
    profile: Profile,
    /// How long to wait between emits, e.g. "5s", "1m".
    interval: String,
    /// Identifies the simulated device; copied onto every row.
    device_id: String,
    /// Seeds the deterministic value generator so a flow replays identically.
    #[serde(default)]
    seed: u64,
}

struct SimulatorInput {
    profile: Profile,
    interval: Duration,
    device_id: String,
    /// xorshift state, advanced once per emit; never zero.
    state: AtomicU64,
    /// Running `kwh_total` for the `energy` profile, in milli-kWh to stay
    /// integer-exact; only ever increases.
    kwh_milli: AtomicU64,
    /// The first emit fires immediately; later emits wait the interval.
    first: AtomicBool,
}

impl SimulatorInput {
    /// Build one row for the configured profile from the next pseudo-random
    /// draw. `ts` is the current wall-clock time as RFC3339, the same string a
    /// real device would stamp.
    fn row(&self) -> Value {
        let ts = crate::time::now_rfc3339();
        sim::build_row(self.profile, &self.device_id, &ts, &self.state, &self.kwh_milli)
    }
}

#[async_trait]
impl Input for SimulatorInput {
    async fn connect(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn read(&self) -> Result<(MessageBatchRef, Arc<dyn Ack>), Error> {
        if !self.first.swap(false, Ordering::SeqCst) {
            tokio::time::sleep(self.interval).await;
        }
        let row = self.row();
        let batch = MessageBatch::from_json(&row)
            .map_err(|e| Error::Process(format!("simulator batch build failed: {e}")))?;
        Ok((batch.into_arc(), Arc::new(NoopAck)))
    }

    async fn close(&self) -> Result<(), Error> {
        Ok(())
    }
}

struct SimulatorInputBuilder;

impl InputBuilder for SimulatorInputBuilder {
    fn build(
        &self,
        _name: Option<&String>,
        config: &Option<Value>,
        _codec: Option<Arc<dyn Codec>>,
        _resource: &Resource,
    ) -> Result<Arc<dyn Input>, Error> {
        let config: SimulatorConfig = config
            .clone()
            .ok_or_else(|| {
                Error::Config("simulator input requires a profile, interval and device_id".into())
            })
            .and_then(|v| {
                serde_json::from_value(v)
                    .map_err(|e| Error::Config(format!("invalid simulator config: {e}")))
            })?;
        let interval = parse_interval(&config.interval)
            .map_err(|e| Error::Config(format!("invalid simulator interval: {e}")))?;
        Ok(Arc::new(SimulatorInput {
            profile: config.profile,
            interval,
            device_id: config.device_id,
            state: sim::seed_state(config.seed),
            kwh_milli: AtomicU64::new(0),
            first: AtomicBool::new(true),
        }))
    }
}

/// Register the `simulator` input type. Called once at startup.
pub fn init() -> Result<(), Error> {
    register_input_builder("simulator", Arc::new(SimulatorInputBuilder))
}
