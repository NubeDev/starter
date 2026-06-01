//! `com.nubeio.rubixos` — Rubix-OS / Nube-iO BMS data extension.
//!
//! Contributes:
//!
//!  - **Tools**: `echo` (smoke probe), `warehouse_query` (browser-
//!    facing proxy over `ctx.warehouse_read().query()` for the
//!    federated UI panel), and the `bc_*` barcode/provisioning tool
//!    surface (decode, provision, device/site/location/page CRUD,
//!    template upsert, label render — see BARCODE.md).
//!  - **Warehouse tables**: the read-only Rubix-OS dump tables
//!    (`points`, `histories`, six tag tables) plus the nine `bc_*`
//!    provisioning catalog tables. The host creates them empty at
//!    boot (prefixed `com_nubeio_rubixos__`, with `tenant_id TEXT`
//!    prepended).
//!  - **Warehouse templates**: the dashboard read templates plus the
//!    `bc_*` provisioning reads.
//!
//! Per SCOPE the only dependency is `starter-ext-sdk`; the handler
//! bodies are pure functions of `(ctx, params)`. The barcode feature
//! lives entirely under [`provision`] and the `bc_*` manifest entries,
//! so it is removable as a unit (BARCODE.md §11).

mod extension;
mod provision;

use extension::{RubixOs, RubixOsCtx};

starter_ext_sdk::register_process_main! {
    extension: RubixOs,
    ctx: RubixOsCtx,
    instance: RubixOs,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("rubix-nubeio-rubixos-extension exiting with error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
