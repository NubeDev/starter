//! `com.acme.geocode` — process flavour (WS-18 callee).
//!
//! Serves one tool, `com.acme.geocode.lookup`, which the manifest marks
//! peer-callable in `contributes.provides[]`. When another extension invokes it
//! through `extension.call`, the host dispatches it here as a `tools/<id>` call
//! carrying the *caller's* identity — this body sees `ctx.caller()` as the
//! original caller, not the geocode extension.
//!
//! `lookup` is a deterministic pseudo-geocode: a stable hash of the address
//! string spread across a valid lat/lon range. Same address → same coordinates,
//! side-effect-free — the point is to demonstrate a peer-callable building
//! block, not real geocoding.

use starter_ext_sdk::serde_json::{json, Value};
use starter_ext_sdk::Extension;

/// The extension's unit struct (SCOPE R5: no fields — state lives in Ctx).
#[derive(Extension)]
#[extension(manifest = "../block.yaml")]
pub struct Geocode;

starter_ext_sdk::requires! {
    name = GeocodeCtx,
    // `lookup` is a pure transform — no host capabilities needed.
    capabilities = [],
}

fn require_str<'a>(params: &'a Value, field: &str) -> starter_ext_sdk::Result<&'a str> {
    params
        .get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| starter_ext_sdk::Error::extension_internal(format!("missing `{field}`")))
}

/// FNV-1a 64-bit — deterministic, dependency-free.
fn hash64(key: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in key.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

impl GeocodeToolHandlers for Geocode {
    type Ctx = GeocodeCtx;

    /// `com.acme.geocode.lookup` — deterministic pseudo-geocode of `address`.
    fn handle_com_acme_geocode_lookup(
        &self,
        _ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        let address = require_str(&params, "address")?;
        let h = hash64(address);
        // Spread into valid ranges: lat [-90, 90], lon [-180, 180], 4 dp.
        let lat = ((h % 1_800_001) as f64) / 10_000.0 - 90.0;
        let lon = (((h >> 21) % 3_600_001) as f64) / 10_000.0 - 180.0;
        Ok(json!({
            "address": address,
            "lat": (lat * 10_000.0).round() / 10_000.0,
            "lon": (lon * 10_000.0).round() / 10_000.0,
        }))
    }
}

// Emits `pub async fn run() -> starter_ext_sdk::Result<()>` driving the stdio
// JSON-RPC loop the supervisor speaks to.
starter_ext_sdk::register_process_main! {
    extension: Geocode,
    ctx: GeocodeCtx,
    instance: Geocode,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("acme-geocode-extension exiting with error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
