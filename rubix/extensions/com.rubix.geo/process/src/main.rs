//! `com.rubix.geo` — generic MapLibre / pins extension.
//!
//! Surfaces:
//!   - `warehouse_query` — thin proxy over `ctx.warehouse_read()`
//!     restricted to `com.rubix.geo.*` templates.
//!   - `pin_{create,update,delete}` — CRUD over `pins`.
//!   - `layer_{create,update,delete}` — CRUD over `map_layers`.
//!
//! Per SCOPE R8 this binary depends ONLY on `starter-ext-sdk`. All
//! tenant scoping, capability gating, and `tenant_id` stamping
//! happens in the host before these handlers run.

mod crud;
mod extension;

use extension::{Geo, GeoCtx};

starter_ext_sdk::register_process_main! {
    extension: Geo,
    ctx: GeoCtx,
    instance: Geo,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("rubix-geo-extension exiting with error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
