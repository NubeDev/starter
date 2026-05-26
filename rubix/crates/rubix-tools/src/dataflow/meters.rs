//! Per-meter state machine.
//!
//! Each meter id maps to a [`MeterProfile`] (kind, unit, nominal
//! tick increment, mess eligibility) and a [`MeterState`] (cumulative
//! value, stuck-zero deadline). Profiles are static; state is held
//! by the tool across calls so cumulative values monotonically rise
//! between ticks and stuck stretches can span many ticks.

use rubix_spi::dto::dataflow::synth::{MeterKind, MeterUnit, ELEC_START_KWH, WATER_START_L};

/// Mess shapes a given meter is eligible for. Encoded here (not in
/// the request) so callers can't accidentally inject a shape that
/// breaks the scenario's success criteria.
#[derive(Debug, Clone, Copy, Default)]
pub struct MessEligibility {
    pub gap: bool,
    pub spike: bool,
    pub stuck: bool,
    pub jitter: bool,
    pub nan: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct MeterProfile {
    pub kind: MeterKind,
    pub unit: MeterUnit,
    /// Nominal increment in `unit` per tick on a clean read. Real
    /// readings perturb this by ±10% jitter so flat lines aren't
    /// suspicious.
    pub clean_step: f64,
    /// Starting cumulative value for a fresh state.
    pub start_value: f64,
    pub mess: MessEligibility,
}

/// Resolve a meter id to its profile. Returns `None` for unknown
/// meters; the tool surfaces that as an `Error::Invalid`.
pub fn profile_for(meter_id: &str) -> Option<MeterProfile> {
    match meter_id {
        "site-a.elec.main" => Some(MeterProfile {
            kind: MeterKind::Electricity,
            unit: MeterUnit::KWh,
            clean_step: 1.2,
            start_value: ELEC_START_KWH,
            mess: MessEligibility {
                gap: true,
                spike: true,
                ..Default::default()
            },
        }),
        "site-a.water.main" => Some(MeterProfile {
            kind: MeterKind::Water,
            unit: MeterUnit::L,
            clean_step: 4.5,
            start_value: WATER_START_L,
            mess: MessEligibility {
                stuck: true,
                ..Default::default()
            },
        }),
        "site-a.elec.hvac" => Some(MeterProfile {
            kind: MeterKind::Electricity,
            unit: MeterUnit::KWh,
            clean_step: 0.8,
            start_value: ELEC_START_KWH,
            mess: MessEligibility {
                jitter: true,
                nan: true,
                ..Default::default()
            },
        }),
        _ => None,
    }
}

/// Mutable per-meter state held by the tool across ticks.
#[derive(Debug, Clone)]
pub struct MeterState {
    /// Last clean cumulative value emitted (spikes do not update this —
    /// a spike is a transient sensor glitch, not a real meter advance).
    pub cumulative: f64,
    /// Tick index at which the current stuck-zero stretch ends, if any.
    /// While `Some`, the meter emits its `cumulative` unchanged.
    pub stuck_until_tick: Option<u64>,
}

impl MeterState {
    pub fn fresh(profile: &MeterProfile) -> Self {
        Self {
            cumulative: profile.start_value,
            stuck_until_tick: None,
        }
    }
}
