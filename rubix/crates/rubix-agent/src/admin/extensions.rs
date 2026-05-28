//! Project the [`ExtensionRegistry`] into [`RegistryItem`]s.
//!
//! Each extension contributes one row carrying the manifest's
//! `display_name`, `version`, the lifecycle state, and a per-kind
//! count of every `contributes.*` slice so the console can render
//! the extension's footprint at a glance.

use std::sync::Arc;

use rubix_spi::dto::admin::{ItemSource, RegistryItem};
use serde_json::json;
use starter_ext_host::ExtensionRegistry;
use starter_ext_spi::LifecycleState;

/// Map a [`LifecycleState`] to the lowercase string carried in
/// `metadata.state` on the wire.
fn lifecycle_label(state: LifecycleState) -> &'static str {
    match state {
        LifecycleState::Discovered => "discovered",
        LifecycleState::Validated => "validated",
        LifecycleState::Starting => "starting",
        LifecycleState::Running => "running",
        LifecycleState::Stopping => "stopping",
        LifecycleState::Stopped => "stopped",
        LifecycleState::Failed => "failed",
        LifecycleState::Crashed => "crashed",
    }
}

/// Project every record in the registry — including those whose
/// manifest failed to validate (their `metadata.state` reports
/// `failed` so the console can render them differently).
pub fn extension_items(extensions: Option<&Arc<ExtensionRegistry>>) -> Vec<RegistryItem> {
    let Some(registry) = extensions else {
        return Vec::new();
    };
    registry
        .list()
        .iter()
        .map(|record| {
            let id = record
                .id
                .as_ref()
                .map(|i| i.as_str().to_owned())
                .unwrap_or_else(|| record.id_hint.clone());
            let label = record
                .manifest
                .as_ref()
                .map(|m| m.display_name.clone())
                .unwrap_or_else(|| id.clone());
            let version = record
                .manifest
                .as_ref()
                .map(|m| m.version.to_string())
                .unwrap_or_default();
            let state = lifecycle_label(record.state);
            let contributes = record
                .manifest
                .as_ref()
                .map(|m| {
                    json!({
                        "tools": m.contributes.tools.len(),
                        "nodes": m.contributes.nodes.len(),
                        "rules": m.contributes.anomaly_rules.len(),
                        "templates": m.contributes.warehouse_templates.len(),
                        "tables": m.contributes.warehouse_tables.len(),
                        "skills": m.contributes.skills.len(),
                    })
                })
                .unwrap_or_else(|| {
                    json!({
                        "tools": 0,
                        "nodes": 0,
                        "rules": 0,
                        "templates": 0,
                        "tables": 0,
                        "skills": 0,
                    })
                });
            let metadata = json!({
                "version": version,
                "state": state,
                "contributes": contributes,
            });
            // Extension records are their own source — the row
            // describes the extension itself, not an item the
            // extension contributes.
            let source = ItemSource::Extension { id: id.clone() };
            RegistryItem::new(id, source)
                .with_label(label)
                .with_metadata(metadata)
        })
        .collect()
}
