//! `rubix.dataflow.synth.emit` — tool dispatch.
//!
//! Holds a seeded RNG and a per-meter state map across calls so
//! cumulative values rise monotonically between ticks and stuck
//! stretches can span many ticks. The RNG is reseeded only when the
//! incoming `knobs.seed` changes — repeat calls with the same seed
//! continue the same deterministic stream.
//!
//! Concurrency: synth is called once per ~60s for ~3 meters.
//! A plain `Mutex` is the right primitive at that rate.

use std::sync::Mutex;

use async_trait::async_trait;
use rand::{rngs::StdRng, SeedableRng};
use rubix_spi::dto::dataflow::synth::{
    MeterReading, ReadingQuality, SynthEmitRequest, SynthEmitResponse, SynthStats,
};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::tool::{Tool, ToolDefinition};
use std::collections::HashMap;

use crate::dataflow::mess;
use crate::dataflow::meters::{profile_for, MeterProfile, MeterState};

/// Resolved knobs (request → env → default). Constructed once per
/// invocation by [`mess::resolve`].
#[derive(Debug, Clone, Copy)]
pub struct Resolved {
    pub seed: u64,
    pub gap_prob: f64,
    pub spike_prob: f64,
    pub stuck_prob: f64,
    pub jitter_ms: i64,
    pub nan_prob: f64,
}

/// Per-process state the tool carries across ticks.
pub struct SynthState {
    rng: StdRng,
    rng_seed: u64,
    /// Global tick counter, incremented once per `invoke` call.
    /// Used as the "now" axis for stuck-zero deadlines so the unit
    /// tests don't depend on wall-clock.
    tick_index: u64,
    meters: HashMap<String, MeterState>,
}

impl SynthState {
    fn new(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
            rng_seed: seed,
            tick_index: 0,
            meters: HashMap::new(),
        }
    }

    fn ensure_seed(&mut self, seed: u64) {
        if seed != self.rng_seed {
            self.rng = StdRng::seed_from_u64(seed);
            self.rng_seed = seed;
            self.tick_index = 0;
            self.meters.clear();
        }
    }
}

/// Concrete `Tool` impl for `rubix.dataflow.synth.emit`.
pub struct SynthEmitTool {
    state: Mutex<SynthState>,
}

impl std::fmt::Debug for SynthEmitTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SynthEmitTool").finish()
    }
}

impl Default for SynthEmitTool {
    fn default() -> Self {
        Self {
            state: Mutex::new(SynthState::new(
                rubix_spi::dto::dataflow::synth::DEFAULT_SEED,
            )),
        }
    }
}

/// Direct entry point for unit tests — bypasses the JSON `invoke`
/// seam so assertions can read `MeterReading`/`SynthStats` directly.
pub fn tick(state: &mut SynthState, req: &SynthEmitRequest) -> Result<SynthEmitResponse> {
    let knobs = mess::resolve(&req.knobs);
    state.ensure_seed(knobs.seed);
    state.tick_index = state.tick_index.saturating_add(1);
    let now_tick = state.tick_index;

    let mut rows = Vec::with_capacity(req.meters.len());
    let mut stats = SynthStats::default();

    for meter_id in &req.meters {
        let profile = profile_for(meter_id).ok_or_else(|| Error::Invalid {
            message: format!("unknown meter_id: {meter_id}"),
        })?;
        let meter_state = state
            .meters
            .entry(meter_id.clone())
            .or_insert_with(|| MeterState::fresh(&profile));

        match tick_one_meter(
            &mut state.rng,
            &knobs,
            &profile,
            meter_state,
            &req.tenant_id,
            meter_id,
            req.tick_epoch_ms,
            now_tick,
            &mut stats,
        ) {
            Some(row) => rows.push(row),
            None => {} // gap
        }
    }

    stats.emitted = rows.len() as u32;
    Ok(SynthEmitResponse { rows, stats })
}

#[allow(clippy::too_many_arguments)]
fn tick_one_meter(
    rng: &mut StdRng,
    knobs: &Resolved,
    profile: &MeterProfile,
    state: &mut MeterState,
    tenant_id: &str,
    meter_id: &str,
    tick_epoch_ms: i64,
    now_tick: u64,
    stats: &mut SynthStats,
) -> Option<MeterReading> {
    // 1. gap — short-circuits the tick.
    if mess::gap_fires(rng, knobs, profile) {
        stats.gaps += 1;
        return None;
    }

    // 2. stuck — start a stretch if not already in one.
    if state.stuck_until_tick.is_none() {
        if let Some(end) = mess::maybe_start_stuck(rng, knobs, profile, now_tick) {
            state.stuck_until_tick = Some(end);
        }
    }
    let in_stuck = match state.stuck_until_tick {
        Some(end) if now_tick <= end => {
            stats.stuck_active += 1;
            true
        }
        Some(_) => {
            state.stuck_until_tick = None;
            false
        }
        None => false,
    };

    // 3. clean step (skipped while stuck — the meter is frozen).
    let mut value = if in_stuck {
        state.cumulative
    } else {
        mess::clean_step(rng, profile, state)
    };
    let mut quality = ReadingQuality::Ok;

    // 4. spike — only on non-stuck ticks (a frozen meter doesn't spike).
    if !in_stuck && mess::spike_fires(rng, knobs, profile) {
        value = mess::spike_value(profile, state);
        quality = ReadingQuality::Suspect;
        stats.spikes += 1;
    }

    // 5. nan
    if mess::nan_fires(rng, knobs, profile) {
        value = f64::NAN;
        quality = ReadingQuality::Suspect;
        stats.nans += 1;
    }

    // 6. jitter
    let epoch_ms = mess::jittered_epoch(rng, knobs, profile, tick_epoch_ms);

    Some(MeterReading {
        tenant_id: tenant_id.to_owned(),
        meter_id: meter_id.to_owned(),
        kind: profile.kind,
        unit: profile.unit,
        epoch_ms,
        value,
        quality,
    })
}

#[async_trait]
impl Tool for SynthEmitTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.dataflow.synth.emit".to_owned(),
            description: "Emit synthetic, deliberately-messy meter readings for one tick."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "required": ["tenant_id", "meters", "tick_epoch_ms"],
                "properties": {
                    "tenant_id":     { "type": "string" },
                    "meters":        { "type": "array",  "items": { "type": "string" } },
                    "tick_epoch_ms": { "type": "integer" },
                    "knobs":         { "type": "object" }
                },
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: SynthEmitRequest = serde_json::from_value(input).map_err(|e| Error::Invalid {
            message: format!("SynthEmitRequest: {e}"),
        })?;
        let mut guard = self.state.lock().map_err(|e| Error::Internal {
            source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
        })?;
        let resp = tick(&mut guard, &req)?;
        serde_json::to_value(resp).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

// Expose SynthState to the in-crate test module without leaking it
// from the public API.
#[cfg(test)]
pub(crate) fn new_state(seed: u64) -> SynthState {
    SynthState::new(seed)
}
