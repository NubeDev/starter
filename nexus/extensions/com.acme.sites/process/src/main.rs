//! `com.acme.sites` — process flavour (WS-18 caller).
//!
//! `register_site` exercises both extension-to-extension channels:
//!   1. **Synchronous peer call** — `ctx.extension_call().call(...)` into
//!      `com.acme.geocode.lookup` to resolve the site address. The call runs
//!      under THIS caller's tenant/identity; the geocode child sees the original
//!      caller, not the sites extension.
//!   2. **Async event-bus publish** — `ctx.event_bus().publish(...)` on
//!      `com.acme.sites.registered` (a topic this extension owns) so any
//!      same-tenant subscriber can react.
//!
//! Both edges are declared in `block.yaml` (`requires_extensions[]` +
//! `capabilities.extension` / `capabilities.event_bus`); the host triple-gates
//! the peer call and checks topic ownership + the publish allowlist.

use starter_ext_sdk::serde_json::{json, Value};
use starter_ext_sdk::Extension;

/// The extension's unit struct (SCOPE R5: no fields — state lives in Ctx).
#[derive(Extension)]
#[extension(manifest = "../block.yaml")]
pub struct Sites;

starter_ext_sdk::requires! {
    name = SitesCtx,
    // WS-18: `extension` grants `ctx.extension_call()`; `event_bus` grants
    // `ctx.event_bus()`.
    capabilities = [extension, event_bus],
}

fn require_str<'a>(params: &'a Value, field: &str) -> starter_ext_sdk::Result<&'a str> {
    params
        .get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| starter_ext_sdk::Error::extension_internal(format!("missing `{field}`")))
}

impl SitesToolHandlers for Sites {
    type Ctx = SitesCtx;

    /// `com.acme.sites.register_site` — geocode via the peer, then announce.
    fn handle_com_acme_sites_register_site(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        let site_name = require_str(&params, "site_name")?;
        let address = require_str(&params, "address")?;

        // (1) Synchronous peer call into the geocode extension. The host gates
        // the target against our grant + declaration + the callee's provides[],
        // and forwards our identity.
        let geo = ctx.extension_call().call(
            "com.acme.geocode",
            "com.acme.geocode.lookup",
            json!({ "address": address }),
        )?;
        let lat = geo.get("lat").and_then(|v| v.as_f64()).ok_or_else(|| {
            starter_ext_sdk::Error::extension_internal("geocode peer returned no `lat`")
        })?;
        let lon = geo.get("lon").and_then(|v| v.as_f64()).ok_or_else(|| {
            starter_ext_sdk::Error::extension_internal("geocode peer returned no `lon`")
        })?;

        // (2) Announce the registration on our owned topic. Publish is
        // fire-and-forget; a publish with no subscribers is a no-op, not an
        // error, so `register_site` succeeds regardless of who is listening.
        let published = ctx
            .event_bus()
            .publish(
                "com.acme.sites.registered",
                json!({ "site_name": site_name, "address": address, "lat": lat, "lon": lon }),
            )
            .is_ok();

        Ok(json!({
            "site_name": site_name,
            "address": address,
            "lat": lat,
            "lon": lon,
            "published": published,
        }))
    }
}

// Emits `pub async fn run() -> starter_ext_sdk::Result<()>` driving the stdio
// JSON-RPC loop the supervisor speaks to.
starter_ext_sdk::register_process_main! {
    extension: Sites,
    ctx: SitesCtx,
    instance: Sites,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("acme-sites-extension exiting with error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
