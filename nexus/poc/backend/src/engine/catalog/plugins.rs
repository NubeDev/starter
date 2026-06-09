//! Plugin manager view: every registered component, with its source.
//!
//! ArkFlow registers components into global builder registries; the UI's plugin
//! manager shows what's installed and which are custom (registered by this POC).

use serde::Serialize;

use super::{buffers, inputs, outputs, processors};

#[derive(Debug, Clone, Serialize)]
pub struct PluginEntry {
    pub r#type: String,
    pub category: &'static str,
    /// "builtin" (shipped by ArkFlow) or "custom" (registered here).
    pub source: &'static str,
}

/// Custom component types registered by this POC (see `engine::register`).
const CUSTOM: &[&str] = &["collector"];

/// Flatten every catalog into one plugin list, tagging custom entries.
pub fn list() -> Vec<PluginEntry> {
    let mut out = Vec::new();
    collect(&mut out, "input", inputs::list().iter().map(kind_type));
    collect(&mut out, "output", outputs::list().iter().map(kind_type));
    collect(&mut out, "processor", processors::list().iter().map(kind_type));
    collect(&mut out, "buffer", buffers::list().iter().map(kind_type));
    out
}

fn kind_type(k: &crate::dto::catalog::ComponentKind) -> String {
    k.r#type.clone()
}

fn collect(out: &mut Vec<PluginEntry>, category: &'static str, types: impl Iterator<Item = String>) {
    for r#type in types {
        let source = if CUSTOM.contains(&r#type.as_str()) {
            "custom"
        } else {
            "builtin"
        };
        out.push(PluginEntry {
            r#type,
            category,
            source,
        });
    }
}
