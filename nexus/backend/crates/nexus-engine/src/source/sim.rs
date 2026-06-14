//! Shared bits for the `simulator` input and the `seed` bin: the device
//! [`Profile`] enum, a tiny deterministic generator, and the per-profile row
//! builder. Kept here so row shapes are defined in one place.

use std::sync::atomic::{AtomicU64, Ordering};

use serde::Deserialize;
use serde_json::{json, Value};

/// The three synthetic device shapes the simulator can emit. See [`build_row`]
/// for the columns each one produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    /// Numeric float telemetry (`temp_c`, `setpoint`, `fan_speed`).
    Hvac,
    /// A monotonic `kwh_total` counter plus instantaneous `power_w`.
    Energy,
    /// A discrete `open: bool` plus a `zone: str`.
    Door,
}

impl Profile {
    /// The datasource table the seed bin lands this profile's rows in.
    pub fn table(self) -> &'static str {
        match self {
            Profile::Hvac => "sim_hvac",
            Profile::Energy => "sim_energy",
            Profile::Door => "sim_door",
        }
    }
}

/// Advance an xorshift64 generator and return the new value. Deterministic for a
/// given seed; shared by the live input and the seed bin so both replay the same
/// stream. Caller guarantees the state is never zero.
pub fn next_rand(state: &AtomicU64) -> u64 {
    let mut x = state.load(Ordering::SeqCst);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    state.store(x, Ordering::SeqCst);
    x
}

/// Fold a seed into a nonzero xorshift state word.
pub fn seed_state(seed: u64) -> AtomicU64 {
    AtomicU64::new(seed ^ 0x9e37_79b9_7f4a_7c15)
}

/// Build one row for `profile` from the next pseudo-random draw. `kwh_milli`
/// carries the running energy counter across calls (in milli-kWh, integer-exact)
/// so `kwh_total` only ever climbs; it is unused by the other profiles. `ts` is
/// the RFC3339 timestamp to stamp on the row.
pub fn build_row(
    profile: Profile,
    device_id: &str,
    ts: &str,
    state: &AtomicU64,
    kwh_milli: &AtomicU64,
) -> Value {
    let r = next_rand(state);
    match profile {
        Profile::Hvac => {
            let temp = 18.0 + (r % 600) as f64 / 100.0; // 18.00..=24.00
            let fan = (r >> 10) % 4;
            json!({
                "device_id": device_id,
                "ts": ts,
                "temp_c": temp,
                "setpoint": 21.0,
                "fan_speed": fan as f64,
            })
        }
        Profile::Energy => {
            let step = 1 + (r % 50); // 1..=50 Wh per tick
            let total = kwh_milli.fetch_add(step, Ordering::SeqCst) + step;
            let power = 200.0 + (r % 1800) as f64; // 200..=1999 W
            json!({
                "device_id": device_id,
                "ts": ts,
                "kwh_total": total as f64 / 1000.0,
                "power_w": power,
            })
        }
        Profile::Door => {
            let open = r & 1 == 1;
            let zones = ["lobby", "server_room", "warehouse", "office"];
            let zone = zones[(r >> 1) as usize % zones.len()];
            json!({
                "device_id": device_id,
                "ts": ts,
                "open": open,
                "zone": zone,
            })
        }
    }
}
