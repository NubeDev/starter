//! Project extension-contributed warehouse tables into
//! [`RegistryItem`]s.
//!
//! Tables are declared on each Validated extension's
//! `contributes.warehouse_tables[]` — the rubix-agent has no
//! built-in tables of its own today. Each row carries the declared
//! columns, sort order, engine, partition expression and TTL in
//! `metadata` so the console can render the schema page without a
//! second fetch.

use std::sync::Arc;

use rubix_spi::dto::admin::{ItemSource, RegistryItem};
use serde_json::{json, Value};
use starter_ext_host::ExtensionRegistry;

/// Project every contributed warehouse table.
pub fn table_items(extensions: Option<&Arc<ExtensionRegistry>>) -> Vec<RegistryItem> {
    let Some(registry) = extensions else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for record in registry.iter_validated() {
        let Some(manifest) = record.manifest.as_ref() else {
            continue;
        };
        let Some(ext_id) = record.id.as_ref() else {
            continue;
        };
        for table in &manifest.contributes.warehouse_tables {
            // The host namespaces a contributed table as
            // `<sanitized_extension_id>__<name>`; the canonical id
            // mirrors that convention so callers can map directly
            // onto the warehouse name.
            let sanitized = ext_id.as_str().replace(['.', '-'], "_");
            let id = format!("{sanitized}__{}", table.name);
            let source = ItemSource::Extension {
                id: ext_id.as_str().to_owned(),
            };
            let columns: Value = table
                .columns
                .iter()
                .map(|col| {
                    let mut obj = serde_json::Map::new();
                    obj.insert("name".into(), Value::String(col.name.clone()));
                    obj.insert("type".into(), Value::String(col.ty.clone()));
                    if let Some(default) = col.default.as_ref() {
                        obj.insert("default".into(), Value::String(default.clone()));
                    }
                    Value::Object(obj)
                })
                .collect();
            let metadata = json!({
                "columns": columns,
                "order_by": table.order_by.clone(),
                "engine": table.engine.clone(),
                "partition_by": table.partition_by.clone(),
                "ttl": table.ttl.clone(),
            });
            out.push(
                RegistryItem::new(id, source)
                    .with_label(table.name.clone())
                    .with_metadata(metadata),
            );
        }
    }
    out
}
