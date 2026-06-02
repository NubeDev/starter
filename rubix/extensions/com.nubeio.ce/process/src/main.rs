//! `com.nubeio.ce` — Control-Engine device manager.
//!
//! Contributes:
//!
//!  - **Tools**: `echo` (smoke probe), `warehouse_query` (browser-
//!    facing proxy over `ctx.warehouse_read().query()` for the
//!    federated UI), the `device_*` CRUD surface over the engine
//!    catalog, and the `engine_*` REST proxy that forwards a
//!    wiresheet read/write to the remote control engine over HTTP.
//!  - **Warehouse table**: `ce_devices` — the catalog of registered
//!    control engines (IP / port / username / password / status).
//!    The host creates it empty at boot (prefixed `com_nubeio_ce__`,
//!    with `tenant_id TEXT` prepended).
//!  - **Warehouse templates**: `devices_list`, `device_get`.
//!
//! Per SCOPE the only dependency is `starter-ext-sdk`; the handler
//! bodies are pure functions of `(ctx, params)`. Device CRUD lives in
//! [`device`], the REST proxy in [`engine`].
//!
//! NOTE: this is a scaffold — handler bodies lay out the call surface
//! but contain no business logic (see the `TODO`s in each module).

mod device;
mod engine;
mod extension;

use extension::{ControlEngine, ControlEngineCtx};

starter_ext_sdk::register_process_main! {
    extension: ControlEngine,
    ctx: ControlEngineCtx,
    instance: ControlEngine,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("rubix-nubeio-ce-extension exiting with error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
