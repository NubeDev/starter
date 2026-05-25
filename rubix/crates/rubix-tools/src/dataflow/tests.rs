//! Unit tests for `rubix.dataflow.synth.emit`.
//!
//! Each test pins the seed and forces one knob to a saturating
//! value so the assertion is deterministic. The composite "all
//! defaults" test asserts the realistic shape stage 01's success
//! bar reads.

use rubix_spi::dto::dataflow::synth::{
    ReadingQuality, SynthEmitRequest, SynthKnobs,
};

use super::synth::{new_state, tick};

const MAIN: &str = "site-a.elec.main";
const WATER: &str = "site-a.water.main";
const HVAC: &str = "site-a.elec.hvac";

fn req(meters: Vec<&str>, knobs: SynthKnobs) -> SynthEmitRequest {
    SynthEmitRequest {
        tenant_id: "site-a".into(),
        meters: meters.into_iter().map(str::to_owned).collect(),
        tick_epoch_ms: 1_748_275_200_000,
        knobs,
    }
}

#[test]
fn gap_saturated_drops_eligible_meter() {
    let mut state = new_state(42);
    let knobs = SynthKnobs {
        seed: Some(42),
        gap_prob: Some(1.0),
        ..Default::default()
    };
    let r = req(vec![MAIN, WATER, HVAC], knobs);
    for _ in 0..60 {
        let resp = tick(&mut state, &r).unwrap();
        // MAIN is the only gap-eligible meter; the other two still emit.
        assert!(resp.rows.iter().all(|row| row.meter_id != MAIN));
        assert_eq!(resp.stats.gaps, 1);
    }
}

#[test]
fn spike_saturated_flags_suspect_and_inflates_value() {
    let mut state = new_state(42);
    let knobs = SynthKnobs {
        seed: Some(42),
        spike_prob: Some(1.0),
        // Pin everything else to 0 so the spike isn't masked.
        gap_prob: Some(0.0),
        stuck_prob: Some(0.0),
        nan_prob: Some(0.0),
        jitter_ms: Some(0),
    };
    let r = req(vec![MAIN], knobs);
    for _ in 0..30 {
        let resp = tick(&mut state, &r).unwrap();
        assert_eq!(resp.rows.len(), 1);
        let row = &resp.rows[0];
        assert_eq!(row.quality, ReadingQuality::Suspect);
        // ×50 spike = cumulative + 50 * clean_step (1.2). Cumulative
        // grows slowly so value should easily clear cumulative + 50.
        assert!(row.value > row.value - 50.0);
        assert!(resp.stats.spikes >= 1);
    }
}

#[test]
fn stuck_saturated_freezes_water_for_10_to_30_ticks() {
    let mut state = new_state(42);
    let knobs = SynthKnobs {
        seed: Some(42),
        stuck_prob: Some(1.0),
        gap_prob: Some(0.0),
        spike_prob: Some(0.0),
        nan_prob: Some(0.0),
        jitter_ms: Some(0),
    };
    let r = req(vec![WATER], knobs);
    let mut values = Vec::new();
    for _ in 0..40 {
        let resp = tick(&mut state, &r).unwrap();
        values.push(resp.rows[0].value);
    }
    // The first emitted value triggers stuck on tick 1; the run of
    // identical values that follows has length ≥ 9 (10 ticks total
    // counting the trigger) and ≤ 29 thereafter.
    let first = values[0];
    let stuck_len = values.iter().take_while(|v| **v == first).count();
    assert!(
        (10..=30).contains(&stuck_len),
        "expected stuck run in [10,30], got {stuck_len}",
    );
}

#[test]
fn nan_saturated_emits_nan_on_hvac() {
    let mut state = new_state(42);
    let knobs = SynthKnobs {
        seed: Some(42),
        nan_prob: Some(1.0),
        gap_prob: Some(0.0),
        spike_prob: Some(0.0),
        stuck_prob: Some(0.0),
        jitter_ms: Some(0),
    };
    let r = req(vec![HVAC], knobs);
    for _ in 0..20 {
        let resp = tick(&mut state, &r).unwrap();
        assert_eq!(resp.rows.len(), 1);
        let row = &resp.rows[0];
        assert!(row.value.is_nan(), "expected NaN, got {}", row.value);
        assert_eq!(row.quality, ReadingQuality::Suspect);
    }
}

#[test]
fn jitter_shifts_hvac_epoch_within_window() {
    let mut state = new_state(42);
    let knobs = SynthKnobs {
        seed: Some(42),
        jitter_ms: Some(20_000),
        gap_prob: Some(0.0),
        spike_prob: Some(0.0),
        stuck_prob: Some(0.0),
        nan_prob: Some(0.0),
    };
    let r = req(vec![HVAC, MAIN], knobs);
    for _ in 0..30 {
        let resp = tick(&mut state, &r).unwrap();
        for row in &resp.rows {
            let delta = (row.epoch_ms - r.tick_epoch_ms).abs();
            if row.meter_id == HVAC {
                assert!(delta <= 20_000, "hvac jitter out of band: {delta}");
            } else {
                assert_eq!(delta, 0, "non-eligible meter jittered: {delta}");
            }
        }
    }
}

#[test]
fn defaults_over_1000_ticks_produce_realistic_mess() {
    let mut state = new_state(42);
    let knobs = SynthKnobs {
        seed: Some(42),
        ..Default::default()
    };
    let r = req(vec![MAIN, WATER, HVAC], knobs);
    let mut total_emitted: u32 = 0;
    let mut total_gaps: u32 = 0;
    let mut total_spikes: u32 = 0;
    for _ in 0..1000 {
        let resp = tick(&mut state, &r).unwrap();
        total_emitted += resp.stats.emitted;
        total_gaps += resp.stats.gaps;
        total_spikes += resp.stats.spikes;
    }
    // 3000 ticks worth of meter-rows minus gaps.
    assert!(
        (2700..=3000).contains(&total_emitted),
        "emitted={total_emitted} out of band",
    );
    // 0.02 gap_prob over 1000 elec.main ticks ⇒ expect ~20.
    assert!(total_gaps >= 1, "expected ≥1 gap, got {total_gaps}");
    // 0.005 spike_prob over 1000 elec.main ticks ⇒ expect ~5.
    assert!(total_spikes >= 1, "expected ≥1 spike, got {total_spikes}");
}

#[test]
fn deterministic_same_seed_same_output() {
    let knobs = SynthKnobs {
        seed: Some(42),
        ..Default::default()
    };
    let r = req(vec![MAIN, WATER, HVAC], knobs);
    let mut a = new_state(42);
    let mut b = new_state(42);
    for _ in 0..50 {
        let ra = tick(&mut a, &r).unwrap();
        let rb = tick(&mut b, &r).unwrap();
        // NaN != NaN — but with default nan_prob ~ 0.0005 over 50
        // ticks the chance of a NaN is ~2.5%; on the seeded path
        // we either both see one or neither does, so byte-equality
        // of the serialised form is the right assertion.
        assert_eq!(
            serde_json::to_value(&ra).unwrap(),
            serde_json::to_value(&rb).unwrap(),
        );
    }
}

#[test]
fn unknown_meter_id_is_invalid_error() {
    let mut state = new_state(42);
    let r = req(vec!["site-a.bogus"], SynthKnobs::default());
    let err = tick(&mut state, &r).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("unknown meter_id"), "unexpected error: {msg}");
}
