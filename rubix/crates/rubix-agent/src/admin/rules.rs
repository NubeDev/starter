//! Project the cleaner [`RuleRegistry`] into [`RegistryItem`]s.
//!
//! The registry exposes rule ids only — schemas are not a concept
//! for anomaly rules (they consume `Reading` structs by-value, not
//! JSON). Source is decided per id: `builtin.*` are builtins,
//! anything else is attributed via the extension registry.

use std::sync::Arc;

use rubix_spi::dto::admin::RegistryItem;
use rubix_tools::cleaner::RuleRegistry;
use serde_json::json;
use starter_ext_host::ExtensionRegistry;

use super::source::item_source;

/// Project every rule the cleaner advertises.
pub fn rule_items(
    rules: Option<&Arc<RuleRegistry>>,
    extensions: Option<&Arc<ExtensionRegistry>>,
) -> Vec<RegistryItem> {
    let Some(rules) = rules else {
        return Vec::new();
    };
    rules
        .ids()
        .map(|id| {
            let source = item_source(id, extensions);
            let metadata = json!({
                "priority": serde_json::Value::Null,
                "quality": "",
            });
            RegistryItem::new(id, source).with_metadata(metadata)
        })
        .collect()
}
