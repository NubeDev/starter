//! Mess injectors. Each is a pure function of `(rng, knobs, state)`.
//!
//! Composition order in `synth::tick_one_meter`:
//!   1. gap         — short-circuits the tick (no row emitted).
//!   2. stuck-start — may begin a stretch; while active, every tick
//!                    re-emits the same value with `quality=ok`.
//!   3. clean step  — advances `cumulative` by a jittered nominal step.
//!   4. spike       — replaces the row's value with ×50 of the clean
//!                    step and flips `quality=suspect`. Does not
//!                    update `cumulative`.
//!   5. nan         — replaces the row's value with NaN and flips
//!                    `quality=suspect`.
//!   6. jitter      — shifts `epoch_ms` by ±knob.

use rand::Rng;
use rubix_spi::dto::dataflow::synth::SynthKnobs;

use crate::dataflow::meters::{MeterProfile, MeterState};
use crate::dataflow::synth::Resolved;

/// Fires with probability `p` against `rng`. Clamped to [0.0, 1.0]
/// — out-of-range knobs are caller bugs but we don't panic, just
/// saturate.
pub fn fires(rng: &mut impl Rng, p: f64) -> bool {
    let clamped = p.clamp(0.0, 1.0);
    rng.gen::<f64>() < clamped
}

/// Should this tick drop the row entirely?
pub fn gap_fires(rng: &mut impl Rng, knobs: &Resolved, profile: &MeterProfile) -> bool {
    profile.mess.gap && fires(rng, knobs.gap_prob)
}

/// Start a stuck-zero stretch lasting 10..=30 minutes (= 10..=30
/// ticks at the 60-s nominal cadence). Returns the tick at which
/// the stretch ends.
pub fn maybe_start_stuck(
    rng: &mut impl Rng,
    knobs: &Resolved,
    profile: &MeterProfile,
    now_tick: u64,
) -> Option<u64> {
    if !profile.mess.stuck || !fires(rng, knobs.stuck_prob) {
        return None;
    }
    let len: u64 = rng.gen_range(10..=30);
    Some(now_tick + len)
}

/// Advance cumulative by a jittered nominal step. Mutates `state`.
/// Returns the new cumulative value.
pub fn clean_step(rng: &mut impl Rng, profile: &MeterProfile, state: &mut MeterState) -> f64 {
    let jitter: f64 = rng.gen_range(0.9_f64..=1.1_f64);
    state.cumulative += profile.clean_step * jitter;
    state.cumulative
}

/// Replace the emitted value with a ×50 spike of the last clean
/// step. Does not advance `cumulative`. Returns the spike value.
pub fn spike_value(profile: &MeterProfile, state: &MeterState) -> f64 {
    state.cumulative + profile.clean_step * 50.0
}

/// Returns true if a spike should fire this tick.
pub fn spike_fires(rng: &mut impl Rng, knobs: &Resolved, profile: &MeterProfile) -> bool {
    profile.mess.spike && fires(rng, knobs.spike_prob)
}

/// Returns true if a NaN should fire this tick.
pub fn nan_fires(rng: &mut impl Rng, knobs: &Resolved, profile: &MeterProfile) -> bool {
    profile.mess.nan && fires(rng, knobs.nan_prob)
}

/// Compute the jittered epoch for a tick. Symmetric uniform in
/// `[-jitter_ms, +jitter_ms]`.
pub fn jittered_epoch(
    rng: &mut impl Rng,
    knobs: &Resolved,
    profile: &MeterProfile,
    tick_epoch_ms: i64,
) -> i64 {
    if !profile.mess.jitter || knobs.jitter_ms <= 0 {
        return tick_epoch_ms;
    }
    let j = rng.gen_range(-knobs.jitter_ms..=knobs.jitter_ms);
    tick_epoch_ms.saturating_add(j)
}

/// Knob resolution: request value wins, then env var, then default.
pub fn resolve(knobs: &SynthKnobs) -> Resolved {
    use rubix_spi::dto::dataflow::synth::{
        DEFAULT_GAP_PROB, DEFAULT_JITTER_MS, DEFAULT_NAN_PROB, DEFAULT_SEED, DEFAULT_SPIKE_PROB,
        DEFAULT_STUCK_PROB,
    };
    Resolved {
        seed: knobs
            .seed
            .or_else(|| env_u64("DATA_FLOW_SEED"))
            .unwrap_or(DEFAULT_SEED),
        gap_prob: knobs
            .gap_prob
            .or_else(|| env_f64("DATA_FLOW_GAP_PROB"))
            .unwrap_or(DEFAULT_GAP_PROB),
        spike_prob: knobs
            .spike_prob
            .or_else(|| env_f64("DATA_FLOW_SPIKE_PROB"))
            .unwrap_or(DEFAULT_SPIKE_PROB),
        stuck_prob: knobs
            .stuck_prob
            .or_else(|| env_f64("DATA_FLOW_STUCK_PROB"))
            .unwrap_or(DEFAULT_STUCK_PROB),
        jitter_ms: knobs
            .jitter_ms
            .or_else(|| env_i64("DATA_FLOW_JITTER_MS"))
            .unwrap_or(DEFAULT_JITTER_MS),
        nan_prob: knobs
            .nan_prob
            .or_else(|| env_f64("DATA_FLOW_NAN_PROB"))
            .unwrap_or(DEFAULT_NAN_PROB),
    }
}

fn env_u64(key: &str) -> Option<u64> {
    std::env::var(key).ok().and_then(|s| s.parse().ok())
}

fn env_i64(key: &str) -> Option<i64> {
    std::env::var(key).ok().and_then(|s| s.parse().ok())
}

fn env_f64(key: &str) -> Option<f64> {
    std::env::var(key).ok().and_then(|s| s.parse().ok())
}
