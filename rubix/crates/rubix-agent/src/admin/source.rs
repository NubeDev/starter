//! Decide an item's [`ItemSource`].
//!
//! The rubix-agent does not annotate registry rows with a
//! `source` tag at registration time — every tool, node, rule,
//! template and table is just an opaque id once the registry
//! settles. This module reconstructs the provenance from the id
//! by checking the live extension registry first, then falling
//! back to namespace prefix rules.

use std::sync::Arc;

use rubix_spi::dto::admin::ItemSource;
use starter_ext_host::ExtensionRegistry;

/// Reverse-DNS prefix for items the upstream `starter-*` crates own.
const STARTER_PREFIX: &str = "starter.";

/// Decide the [`ItemSource`] for an item id.
///
/// Resolution order:
/// 1. If `extensions` lists an extension whose reverse-DNS id is a
///    prefix of `item_id` (with a trailing `.` or exact match),
///    the item is attributed to that extension.
/// 2. Items starting with `starter.` are [`ItemSource::Starter`].
/// 3. Anything else (`rubix.`, `builtin.`, or un-namespaced ids such
///    as the cleaner's `cleaner.tick` shim) is [`ItemSource::Builtin`].
pub fn item_source(item_id: &str, extensions: Option<&Arc<ExtensionRegistry>>) -> ItemSource {
    if let Some(registry) = extensions {
        for record in registry.iter_validated() {
            let Some(id) = record.id.as_ref() else {
                continue;
            };
            let ext_id = id.as_str();
            if item_id == ext_id || item_id.starts_with(&format!("{ext_id}.")) {
                return ItemSource::Extension {
                    id: ext_id.to_owned(),
                };
            }
        }
    }
    if item_id.starts_with(STARTER_PREFIX) {
        ItemSource::Starter
    } else {
        ItemSource::Builtin
    }
}
