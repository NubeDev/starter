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
use starter_ext_spi::{IssueCode, LifecycleState};

/// Map a starter [`IssueCode`] (a stable, non-localised wire token such
/// as `ext.issue.crashed`) onto the rubix `MessageKey` the admin console
/// renders. The starter API deliberately carries no English; rubix owns
/// the `rubix.extension.issue.*` catalog (see
/// `rubix-spi/catalogues/en.json`). The console projects an extension's
/// `/issues` feed through this map so each issue gets a localised title.
///
/// Returns a `&'static str` MessageKey for every variant — the mapping
/// is total, so a new `IssueCode` upstream forces a compile error here
/// rather than silently surfacing as an untranslated raw code.
pub fn issue_code_message_key(code: IssueCode) -> &'static str {
    match code {
        IssueCode::ManifestInvalid => "rubix.extension.issue.manifest_invalid",
        IssueCode::NamespaceViolation => "rubix.extension.issue.namespace_violation",
        IssueCode::CapabilityMismatch => "rubix.extension.issue.capability_mismatch",
        IssueCode::Crashed => "rubix.extension.issue.crashed",
        IssueCode::RestartCapExceeded => "rubix.extension.issue.restart_cap_exceeded",
        IssueCode::HealthTimeout => "rubix.extension.issue.health_timeout",
        IssueCode::CapabilityViolation => "rubix.extension.issue.capability_violation",
        IssueCode::WorkerFailed => "rubix.extension.issue.worker_failed",
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `IssueCode` maps to a `rubix.extension.issue.*` MessageKey,
    /// and the mapping is one-to-one (no two codes collide).
    #[test]
    fn issue_code_message_keys_are_unique_and_namespaced() {
        let codes = [
            IssueCode::ManifestInvalid,
            IssueCode::NamespaceViolation,
            IssueCode::CapabilityMismatch,
            IssueCode::Crashed,
            IssueCode::RestartCapExceeded,
            IssueCode::HealthTimeout,
            IssueCode::CapabilityViolation,
            IssueCode::WorkerFailed,
        ];
        let mut seen = std::collections::HashSet::new();
        for code in codes {
            let key = issue_code_message_key(code);
            assert!(
                key.starts_with("rubix.extension.issue."),
                "key {key} is not namespaced"
            );
            assert!(seen.insert(key), "duplicate key {key}");
        }
    }
}
