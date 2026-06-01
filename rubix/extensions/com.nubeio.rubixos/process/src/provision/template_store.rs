//! `bc_template_upsert` and template lookup against `bc_templates`.
//!
//! Templates live in Postgres so an operator can add or edit a device
//! type at runtime without a rebuild (BARCODE.md §3, §4.1). Upsert
//! validates the YAML (strict parse) *before* storing, so a broken
//! template can never land. The `templates/*.yaml` files in this repo
//! are the seed loaded at first boot.

use starter_ext_sdk::serde_json::{json, Map, Value};
use starter_ext_sdk::{Error, Row};

use crate::provision::crud::take_str;
use crate::provision::template::DeviceTemplate;
use crate::provision::RubixOsCtx;

/// `bc_template_upsert` — validate a YAML template then store it.
pub fn handle_upsert(ctx: &RubixOsCtx, params: &Value) -> starter_ext_sdk::Result<Value> {
    let yaml = take_str(params, "yaml", "bc_template_upsert")?;
    let tpl = DeviceTemplate::parse(&yaml)?;
    let row = template_row(&tpl, &yaml);

    // Upsert == update-then-insert-if-absent. The write backend's
    // update returns the affected count; 0 means the row is new and we
    // insert it. Keyed on `template`.
    let updated = ctx
        .warehouse_write()
        .update("bc_templates", "template", vec![Row::from_map(row.clone())])?;
    let operation = if updated > 0 {
        "update"
    } else {
        ctx.warehouse_write()
            .insert("bc_templates", vec![Row::from_map(row)])?;
        "create"
    };
    Ok(json!({ "operation": operation, "affected": 1, "template": tpl.template }))
}

/// Load and parse a template by key from `bc_templates`.
pub fn load(ctx: &RubixOsCtx, model: &str) -> starter_ext_sdk::Result<DeviceTemplate> {
    let rows = ctx.warehouse_read().query(
        "com.nubeio.rubixos.bc_templates_list",
        json!({ "limit": 500 }),
    )?;
    // The list template returns metadata, not the raw YAML, so re-read
    // is by `template` key. We stored the YAML in `yaml`; fetch it via
    // a direct row scan of the list plus a targeted re-read.
    let known: Vec<String> = rows
        .iter()
        .filter_map(|r| r.0.get("template").and_then(Value::as_str).map(str::to_owned))
        .collect();
    if !known.iter().any(|t| t == model) {
        return Err(Error::Validation(format!(
            "bc: unknown model `{model}` — known templates: [{}]",
            known.join(", ")
        )));
    }
    let yaml = load_yaml(ctx, model)?;
    DeviceTemplate::parse(&yaml)
}

/// List of known template keys, for friendly decode errors.
pub fn known_models(ctx: &RubixOsCtx) -> Vec<String> {
    ctx.warehouse_read()
        .query("com.nubeio.rubixos.bc_templates_list", json!({ "limit": 500 }))
        .map(|rows| {
            rows.iter()
                .filter_map(|r| {
                    r.0.get("template").and_then(Value::as_str).map(str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Re-read the raw YAML for one template. `bc_templates_list` omits
/// the YAML body to keep list reads light, so this uses the dedicated
/// single-row read template.
fn load_yaml(ctx: &RubixOsCtx, model: &str) -> starter_ext_sdk::Result<String> {
    let rows = ctx.warehouse_read().query(
        "com.nubeio.rubixos.bc_template_yaml",
        json!({ "template": model }),
    )?;
    rows.into_iter()
        .next()
        .and_then(|r| r.0.get("yaml").and_then(Value::as_str).map(str::to_owned))
        .ok_or_else(|| {
            Error::Validation(format!("bc: template `{model}` has no stored YAML body"))
        })
}

/// Build the `bc_templates` row for a parsed template.
fn template_row(tpl: &DeviceTemplate, yaml: &str) -> Map<String, Value> {
    let mut row = Map::new();
    row.insert("template".into(), json!(tpl.template));
    row.insert("version".into(), json!(tpl.version));
    row.insert("display_name".into(), json!(tpl.display_name));
    row.insert("network".into(), json!(tpl.network));
    row.insert("category".into(), json!(tpl.category));
    row.insert("icon".into(), json!(tpl.icon));
    row.insert("yaml".into(), json!(yaml));
    row.insert("points_json".into(), tpl.points_json());
    row.insert("widget_group_json".into(), tpl.widget_group_json());
    row
}
