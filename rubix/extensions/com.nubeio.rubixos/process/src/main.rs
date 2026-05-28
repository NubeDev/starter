//! `com.nubeio.rubixos` — Rubix-OS / Nube-iO BMS data extension.
//!
//! Contributes:
//!
//!  - **Tools**: `echo` (smoke probe), `warehouse_query` (browser-
//!    facing proxy over `ctx.warehouse_read().query()` for the
//!    federated UI panel).
//!  - **Warehouse tables**: `points`, `histories`, plus six tag /
//!    meta-tag tables. The host creates them empty at boot
//!    (prefixed `com_nubeio_rubixos__`, with `tenant_id TEXT`
//!    prepended). The bundled `scripts/load-dump.sh` then
//!    pg_restores a Rubix-OS dump into a staging schema and bulk-
//!    INSERTs into the host tables.
//!  - **Warehouse templates**: ten named templates for the
//!    dashboard (`points_list`, `devices_overview`,
//!    `history_recent`, `history_bucketed`, …).
//!
//! Per SCOPE R8 the only dependency is `starter-ext-sdk`; the
//! handler bodies are pure functions of `(ctx, params)` and would
//! be byte-identical in an inproc/wasm twin.

use starter_ext_sdk::serde_json::{json, Value};
use starter_ext_sdk::Extension;

#[derive(Extension)]
#[extension(manifest = "../block.yaml")]
pub struct RubixOs;

starter_ext_sdk::requires! {
    name = RubixOsCtx,
    capabilities = [warehouse_read, tracing],
}

impl RubixOsToolHandlers for RubixOs {
    type Ctx = RubixOsCtx;

    /// `com.nubeio.rubixos.echo` — return the input verbatim.
    fn handle_com_nubeio_rubixos_echo(
        &self,
        _ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        Ok(params)
    }

    /// `com.nubeio.rubixos.warehouse_query` — thin proxy over
    /// `ctx.warehouse_read().query(template, params)` for the
    /// bundled UI panel. The browser cannot reach
    /// `WarehouseReadHandle` directly so the UI calls
    /// `/api/v1/tools/com.nubeio.rubixos.warehouse_query` and the
    /// host's tool dispatcher routes it here.
    ///
    /// Refuses anything outside this extension's own
    /// `com.nubeio.rubixos.*` template namespace — the host would
    /// also reject foreign templates via the grant gate, but the
    /// pre-check keeps the error surface friendly.
    fn handle_com_nubeio_rubixos_warehouse_query(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        let template = params
            .get("template")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                starter_ext_sdk::Error::Validation(
                    "warehouse_query: `template` (string) is required".into(),
                )
            })?
            .to_owned();

        if !template.starts_with("com.nubeio.rubixos.") {
            return Err(starter_ext_sdk::Error::Validation(format!(
                "warehouse_query: template `{template}` is outside this \
                 extension's namespace (`com.nubeio.rubixos.*`)"
            )));
        }

        let tpl_params = params
            .get("params")
            .cloned()
            .unwrap_or_else(|| json!({}));

        let rows = ctx.warehouse_read().query(&template, tpl_params)?;
        let rows_json: Vec<Value> = rows.into_iter().map(|r| Value::Object(r.0)).collect();
        let count = rows_json.len();

        Ok(json!({
            "template": template,
            "rows": rows_json,
            "count": count,
        }))
    }
}

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
