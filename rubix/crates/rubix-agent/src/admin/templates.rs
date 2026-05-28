//! Project the warehouse-read [`TemplateRegistry`] into
//! [`RegistryItem`]s.
//!
//! Each [`TemplateSpec`] supplies a `params` JSON Schema (passed
//! through verbatim as `input_schema`), the `tables` allowlist
//! (surfaced in `metadata.tables`), and an optional `sql` body
//! (the first 200 bytes go into `metadata.sql_preview` so the
//! console can show a teaser without serving multi-kilobyte SQL).

use std::sync::Arc;

use rubix_spi::dto::admin::RegistryItem;
use serde_json::{json, Value};
use starter_ext_host::{ExtensionRegistry, TemplateRegistry};

use super::source::item_source;

const SQL_PREVIEW_BYTES: usize = 200;

/// Project every registered warehouse template.
pub fn template_items(
    templates: Option<&Arc<TemplateRegistry>>,
    extensions: Option<&Arc<ExtensionRegistry>>,
) -> Vec<RegistryItem> {
    let Some(templates) = templates else {
        return Vec::new();
    };
    templates
        .iter()
        .map(|spec| {
            let source = item_source(&spec.name, extensions);
            let sql_preview: Value = match spec.sql.as_deref() {
                Some(sql) if !sql.is_empty() => {
                    let take = sql.len().min(SQL_PREVIEW_BYTES);
                    Value::String(sql[..take].to_owned())
                }
                _ => Value::Null,
            };
            let metadata = json!({
                "tables": spec.tables.clone(),
                "sql_preview": sql_preview,
            });
            RegistryItem::new(spec.name.clone(), source)
                .with_input_schema(spec.params.clone())
                .with_metadata(metadata)
        })
        .collect()
}
