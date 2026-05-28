use starter_ext_sdk::serde_json::{json, Value};
use starter_ext_sdk::Extension;

use crate::crud::{crud_delete, crud_insert, crud_update, take_str};

#[derive(Extension)]
#[extension(manifest = "../block.yaml")]
pub struct Geo;

starter_ext_sdk::requires! {
    name = GeoCtx,
    capabilities = [warehouse_read, warehouse_write, tracing],
}

impl GeoToolHandlers for Geo {
    type Ctx = GeoCtx;

    fn handle_com_rubix_geo_warehouse_query(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        let template = take_str(&params, "template", "warehouse_query")?;
        if !template.starts_with("com.rubix.geo.") {
            return Err(starter_ext_sdk::Error::Validation(format!(
                "warehouse_query: template `{template}` is outside this \
                 extension's namespace (`com.rubix.geo.*`)"
            )));
        }
        let tpl_params = params.get("params").cloned().unwrap_or_else(|| json!({}));
        let rows = ctx.warehouse_read().query(&template, tpl_params)?;
        let rows_json: Vec<Value> = rows.into_iter().map(|r| Value::Object(r.0)).collect();
        let count = rows_json.len();
        Ok(json!({ "template": template, "rows": rows_json, "count": count }))
    }

    fn handle_com_rubix_geo_pin_create(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        crud_insert(ctx, &params, "pins", "pin_create")
    }

    fn handle_com_rubix_geo_pin_update(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        crud_update(ctx, &params, "pins", "pin_id", "pin_update")
    }

    fn handle_com_rubix_geo_pin_delete(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        crud_delete(ctx, &params, "pins", "pin_id", "pin_ids", "pin_delete")
    }

    fn handle_com_rubix_geo_layer_create(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        crud_insert(ctx, &params, "map_layers", "layer_create")
    }

    fn handle_com_rubix_geo_layer_update(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        crud_update(ctx, &params, "map_layers", "layer_id", "layer_update")
    }

    fn handle_com_rubix_geo_layer_delete(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        crud_delete(ctx, &params, "map_layers", "layer_id", "layer_ids", "layer_delete")
    }
}
