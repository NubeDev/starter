//! Project the flow node-kind registry into [`RegistryItem`]s.
//!
//! For M1 we project only the built-in node behaviours; the runtime
//! [`starter_flow::registry::NodeKindRegistry`] holds the same set
//! behind an async lock that requires a runtime context to walk.
//! Extension-contributed node kinds land here when the slot
//! registry / dynamic node-kind projection wires up (M3).

use std::sync::Arc;

use rubix_spi::dto::admin::{ItemSource, RegistryItem};
use serde_json::{json, to_value};
use starter_ext_host::ExtensionRegistry;
use starter_flow_spi::node::NodeBehavior;

use super::source::item_source;

/// Project every built-in node behaviour into the wire envelope.
pub fn node_items(
    behaviors: &[Arc<dyn NodeBehavior>],
    extensions: Option<&Arc<ExtensionRegistry>>,
) -> Vec<RegistryItem> {
    behaviors
        .iter()
        .map(|b| node_to_item(&**b, extensions))
        .collect()
}

/// Project a single node behaviour.
pub fn node_to_item(
    behavior: &dyn NodeBehavior,
    extensions: Option<&Arc<ExtensionRegistry>>,
) -> RegistryItem {
    let id = behavior.kind_id().as_str().to_owned();
    let source: ItemSource = item_source(&id, extensions);
    let schema = to_value(behavior.config_schema()).unwrap_or(serde_json::Value::Null);
    let metadata = json!({
        "facets": serde_json::Value::Array(vec![]),
        "streaming": false,
    });
    let item = RegistryItem::new(id, source).with_metadata(metadata);
    if schema.is_null() {
        item
    } else {
        item.with_input_schema(schema)
    }
}
