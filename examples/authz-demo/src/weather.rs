//! `com.acme.weather` extension — builtin handler table.
//!
//! Phase 7d / SCOPE-EXT R15: the host-side hand-mounting that this
//! file used to do is **gone**. The extension's manifest declares the
//! per-route `auth.permission` field directly:
//!
//! ```yaml
//! contributes:
//!   rest:
//!     - id: com.acme.weather.forecast
//!       method: GET
//!       path: /weather/forecast
//!       auth: { permission: { resource: weather, action: read } }
//!     - id: com.acme.weather.refresh
//!       method: POST
//!       path: /weather/refresh
//!       auth: { permission: { resource: weather, action: refresh } }
//! ```
//!
//! `starter_ext_server::rest_router` reads those `permission` blocks,
//! validates each `resource` against the host's `ResourceRegistry` at
//! build time (typo → `RestBuildError::UnknownResource`, the extension
//! refuses to mount), and wraps each entry's handler in
//! `with_permission(resource, action)` automatically. The layer
//! order is `with_role → with_scope → with_permission → handler`
//! (R15); see `starter_ext_server::rest::auth::apply_gate` for the
//! audit-consequence note.
//!
//! What remains here is the **body** of the two endpoints. The
//! manifest's `runtime: builtin` tells the host this extension's
//! requests dispatch through a [`BuiltinTable`] registered at boot;
//! the table below is what the `BuiltinRestDispatcher` calls.
//!
//! Equivalent pre-Phase-7d code (kept as a docstring as a witness to
//! "this is what the adapter does for you now"):
//!
//! ```ignore
//! let read  = with_permission(
//!     Router::new().route("/weather/forecast", get(forecast)),
//!     "weather", "read",
//! );
//! let write = with_permission(
//!     Router::new().route("/weather/refresh",  post(refresh)),
//!     "weather", "refresh",
//! );
//! Router::new().merge(read).merge(write)
//! ```

use std::sync::Arc;

use chrono::Utc;
use serde_json::{json, Value};
use starter_ext_sdk::builtin::{BuiltinEntry, BuiltinTable};
use starter_ext_sdk::{Error, ExtensionId};

/// Construct the [`BuiltinTable`] entry for `com.acme.weather`. The
/// table is wrapped in an `Arc` and handed to
/// `BuiltinRestDispatcher::new`; the dispatcher calls into the
/// closure below for every request the REST adapter routes here.
pub fn builtin_table() -> Arc<BuiltinTable> {
    let mut table = BuiltinTable::new();
    let extension_id = ExtensionId::new("com.acme.weather").expect("weather id is valid");
    let entry = BuiltinEntry::new(
        &["com.acme.weather.forecast", "com.acme.weather.refresh"],
        |contribute_id, _ctx, _params| -> Result<Value, Error> {
            match contribute_id {
                "com.acme.weather.forecast" => Ok(json!({
                    "city": "Brisbane",
                    "temp_c": 24.5,
                    "condition": "sunny",
                })),
                "com.acme.weather.refresh" => Ok(json!({
                    "refreshed_at": Utc::now().to_rfc3339(),
                    "cleared": 0,
                })),
                other => Err(Error::validation(format!(
                    "unexpected contribute id {other:?}"
                ))),
            }
        },
    );
    table.insert(extension_id, entry);
    Arc::new(table)
}
