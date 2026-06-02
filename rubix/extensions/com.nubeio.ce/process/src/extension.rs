//! `com.nubeio.ce` — extension definition + tool dispatch.
//!
//! Holds the `#[derive(Extension)]` struct, the capability bundle, and
//! the `ControlEngineToolHandlers` impl. Each handler is thin: extract,
//! call one domain function (in [`crate::device`] for catalog CRUD,
//! [`crate::engine`] for the REST proxy), shape the result, return. The
//! `tenant_id` stamp, capability gating and column validation all
//! happen host-side before these run.

use starter_ext_sdk::serde_json::{json, Value};
use starter_ext_sdk::Extension;

use crate::{device, engine};

#[derive(Extension)]
#[extension(manifest = "../block.yaml")]
pub struct ControlEngine;

starter_ext_sdk::requires! {
    name = ControlEngineCtx,
    capabilities = [warehouse_read, warehouse_write, tracing],
}

impl ControlEngineToolHandlers for ControlEngine {
    type Ctx = ControlEngineCtx;

    /// `com.nubeio.ce.echo` — return the input verbatim. Smoke probe.
    fn handle_com_nubeio_ce_echo(
        &self,
        _ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        Ok(params)
    }

    /// `com.nubeio.ce.warehouse_query` — thin proxy over
    /// `ctx.warehouse_read().query(template, params)` for the bundled
    /// UI. Refuses anything outside this extension's own
    /// `com.nubeio.ce.*` template namespace.
    fn handle_com_nubeio_ce_warehouse_query(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        let template = device::take_str(&params, "template", "warehouse_query")?;
        if !template.starts_with("com.nubeio.ce.") {
            return Err(starter_ext_sdk::Error::Validation(format!(
                "warehouse_query: template `{template}` is outside this \
                 extension's namespace (`com.nubeio.ce.*`)"
            )));
        }
        let tpl_params = params.get("params").cloned().unwrap_or_else(|| json!({}));
        let rows = ctx.warehouse_read().query(&template, tpl_params)?;
        let rows_json: Vec<Value> = rows.into_iter().map(|r| Value::Object(r.0)).collect();
        let count = rows_json.len();
        Ok(json!({ "template": template, "rows": rows_json, "count": count }))
    }

    // -- Device catalog CRUD (`ce_devices`) ---------------------------

    fn handle_com_nubeio_ce_device_create(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        device::handle_create(ctx, &params)
    }

    fn handle_com_nubeio_ce_device_update(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        device::handle_update(ctx, &params)
    }

    fn handle_com_nubeio_ce_device_delete(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        device::handle_delete(ctx, &params)
    }

    // -- Control-engine REST proxy ------------------------------------

    fn handle_com_nubeio_ce_engine_status(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        engine::handle_status(ctx, &params)
    }

    fn handle_com_nubeio_ce_engine_wiresheet_get(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        engine::handle_wiresheet_get(ctx, &params)
    }

    fn handle_com_nubeio_ce_engine_wiresheet_put(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        engine::handle_wiresheet_put(ctx, &params)
    }
}
