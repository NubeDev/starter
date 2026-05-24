//! YAML surface types + the `parse_yaml` verb.
//!
//! Contributors author the lighter shape (`id`, `config`, short `id`s
//! on nodes) and the [`crate::convert`] step lifts it into the typed
//! starter-flow body.

use serde::Deserialize;

use crate::error::LoadError;

/// Surface shape every bundled `flows/*.yaml` file deserialises into.
#[derive(Debug, Clone, Deserialize)]
pub struct RubixFlowYaml {
    /// Flow id (reverse-DNS).
    pub id: String,
    /// Human-readable description; surfaces as the MCP tool description.
    #[serde(default)]
    pub description: Option<String>,
    /// Trigger declaration (`explicit`, `schedule(cron = "...")`, …).
    #[serde(default)]
    pub trigger: Option<String>,
    /// Declared nodes (first is the root).
    #[serde(default)]
    pub nodes: Vec<RubixNodeYaml>,
    /// Slot-to-slot links; today every bundled flow ships `links: []`.
    #[serde(default)]
    pub links: Vec<RubixLinkYaml>,
}

/// One node entry inside a [`RubixFlowYaml`].
#[derive(Debug, Clone, Deserialize)]
pub struct RubixNodeYaml {
    /// Short authoring id (`agent`, `check`, …).
    pub id: String,
    /// Node kind — bundled flows use the literal `ai-agent`.
    pub kind: String,
    /// Per-node config bag, forwarded verbatim as JSON.
    #[serde(default)]
    pub config: serde_yaml::Value,
}

/// One link entry inside a [`RubixFlowYaml`].
#[derive(Debug, Clone, Deserialize)]
pub struct RubixLinkYaml {
    /// Source slot, formatted `<node_id>.<slot_name>`.
    pub from: String,
    /// Destination slot, formatted `<node_id>.<slot_name>`.
    pub to: String,
}

/// Parse one YAML byte slice into the surface [`RubixFlowYaml`] shape.
pub fn parse_yaml(path: &str, bytes: &[u8]) -> Result<RubixFlowYaml, LoadError> {
    std::str::from_utf8(bytes).map_err(|e| LoadError::Utf8 {
        path: path.to_owned(),
        source: e,
    })?;
    serde_yaml::from_slice(bytes).map_err(|e| LoadError::Yaml {
        path: path.to_owned(),
        source: e,
    })
}
