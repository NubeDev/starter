//! `com.nubeio.rubixos` — extension definition + tool dispatch.
//!
//! Holds the `#[derive(Extension)]` struct, the capability bundle, and
//! the `RubixOsToolHandlers` impl. Each handler is thin: extract,
//! call one domain function (in `crate::provision` for the barcode
//! feature), shape the result, return. The `tenant_id` stamp,
//! capability gating and column validation all happen host-side
//! before these run.

use starter_ext_sdk::serde_json::{json, Value};
use starter_ext_sdk::Extension;

use crate::provision;

#[derive(Extension)]
#[extension(manifest = "../block.yaml")]
pub struct RubixOs;

starter_ext_sdk::requires! {
    name = RubixOsCtx,
    capabilities = [warehouse_read, warehouse_write, tracing],
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
    /// `ctx.warehouse_read().query(template, params)` for the bundled
    /// UI panel. Refuses anything outside this extension's own
    /// `com.nubeio.rubixos.*` template namespace; the host's grant
    /// gate would also reject foreign templates, but the pre-check
    /// keeps the error surface friendly.
    fn handle_com_nubeio_rubixos_warehouse_query(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        let template = provision::crud::take_str(&params, "template", "warehouse_query")?;
        if !template.starts_with("com.nubeio.rubixos.") {
            return Err(starter_ext_sdk::Error::Validation(format!(
                "warehouse_query: template `{template}` is outside this \
                 extension's namespace (`com.nubeio.rubixos.*`)"
            )));
        }
        let tpl_params = params.get("params").cloned().unwrap_or_else(|| json!({}));
        let started = std::time::Instant::now();
        let rows = ctx.warehouse_read().query(&template, tpl_params)?;
        let elapsed = started.elapsed();
        if elapsed > std::time::Duration::from_millis(500) {
            eprintln!(
                "warehouse_query SLOW: template={template} elapsed_ms={} rows={}",
                elapsed.as_millis(),
                rows.len(),
            );
        }
        let rows_json: Vec<Value> = rows.into_iter().map(|r| Value::Object(r.0)).collect();
        let count = rows_json.len();
        Ok(json!({ "template": template, "rows": rows_json, "count": count }))
    }

    // -- Barcode / scan-to-dashboard provisioning (BARCODE.md) --------

    fn handle_com_nubeio_rubixos_bc_decode(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        provision::handle_decode(ctx, &params)
    }

    fn handle_com_nubeio_rubixos_bc_provision(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        provision::handle_provision(ctx, &params)
    }

    fn handle_com_nubeio_rubixos_bc_device_update(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        provision::handle_device_update(ctx, &params)
    }

    fn handle_com_nubeio_rubixos_bc_device_decommission(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        provision::handle_device_decommission(ctx, &params)
    }

    fn handle_com_nubeio_rubixos_bc_device_assign_page(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        provision::handle_device_assign_page(ctx, &params)
    }

    fn handle_com_nubeio_rubixos_bc_site_create(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        provision::handle_site_create(ctx, &params)
    }

    fn handle_com_nubeio_rubixos_bc_location_create(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        provision::handle_location_create(ctx, &params)
    }

    fn handle_com_nubeio_rubixos_bc_page_create(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        provision::handle_page_create(ctx, &params)
    }

    fn handle_com_nubeio_rubixos_bc_template_upsert(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        provision::handle_template_upsert(ctx, &params)
    }

    fn handle_com_nubeio_rubixos_bc_label_render(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        provision::handle_label_render(ctx, &params)
    }
}
