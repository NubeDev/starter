//! The native `simulator` source: emit synthetic device telemetry on an
//! interval — test data with no upstream.
//!
//! Built on the [`Source`] trait over the shared [`super::sim`] row builder, so
//! the three device profiles (`hvac`, `energy`, `door`) emit identical shapes. It
//! never ends on its own; the flow's cancellation token stops it. Output is
//! deterministic for a given `seed`.

use std::sync::atomic::AtomicU64;

use datafusion::arrow::array::RecordBatch;
use serde::Deserialize;
use serde_json::Value;

use crate::arrow_json::json_carrier_batch;
use crate::core::{EngineError, EngineResult, Source};
use crate::source::interval::parse_cadence;
use crate::source::sim::{self, Profile};
use crate::time::now_rfc3339;

#[derive(Debug, Clone, Deserialize)]
struct SimulatorConfig {
    /// Which device shape to emit: `hvac`, `energy`, or `door`.
    profile: Profile,
    /// Delay between emits, e.g. `"5s"`, `"1m"`.
    interval: String,
    /// Identifies the simulated device; copied onto every row.
    device_id: String,
    /// Seeds the deterministic value generator so a flow replays identically.
    #[serde(default)]
    seed: u64,
}

/// Emits one synthetic telemetry row per tick for the configured profile.
pub struct SimulatorSource {
    profile: Profile,
    interval: std::time::Duration,
    device_id: String,
    /// xorshift state, advanced once per emit; never zero.
    state: AtomicU64,
    /// Running `kwh_total` for the `energy` profile, in milli-kWh.
    kwh_milli: AtomicU64,
    first: bool,
}

impl SimulatorSource {
    /// Build from the node config, requiring `profile`, `interval`, and
    /// `device_id`.
    pub fn from_config(config: &Value) -> EngineResult<Self> {
        let config: SimulatorConfig = serde_json::from_value(config.clone())
            .map_err(|e| EngineError::Build(format!("invalid simulator config: {e}")))?;
        let interval = parse_cadence(&config.interval)
            .map_err(|e| EngineError::Build(format!("invalid simulator interval: {e}")))?;
        Ok(Self {
            profile: config.profile,
            interval,
            device_id: config.device_id,
            state: sim::seed_state(config.seed),
            kwh_milli: AtomicU64::new(0),
            first: true,
        })
    }
}

#[async_trait::async_trait]
impl Source for SimulatorSource {
    async fn read(&mut self) -> EngineResult<Option<RecordBatch>> {
        if self.first {
            self.first = false;
        } else {
            tokio::time::sleep(self.interval).await;
        }
        let ts = now_rfc3339();
        let row = sim::build_row(
            self.profile,
            &self.device_id,
            &ts,
            &self.state,
            &self.kwh_milli,
        );
        Ok(Some(json_carrier_batch(&[row.to_string()])))
    }
}
