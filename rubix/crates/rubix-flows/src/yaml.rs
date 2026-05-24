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

/// Key under `node.config` that carries the per-node tool allowlist
/// (a YAML array of reverse-DNS tool ids). The list is read in full
/// and threaded into the AiAgentNode config so the agent loop's
/// `ToolSet` filter scopes tool visibility per flow — every entry
/// in the list is allowed, not just `[0]`.
pub const ALLOWED_TOOLS_KEY: &str = "allowed_tools";

impl RubixNodeYaml {
    /// Return the full `config.allowed_tools` list, in declaration
    /// order. An empty `Vec` means the YAML did not declare the key;
    /// callers downstream interpret that as "no per-node scoping"
    /// (the AiAgentNode falls back to the host registry intersected
    /// with the active skill's allowlist per D-F4.5).
    ///
    /// Returns an error if `allowed_tools` is present but is not a
    /// YAML sequence of strings — surfacing typos at load time
    /// rather than at first invocation.
    pub fn allowed_tools(&self) -> Result<Vec<String>, LoadError> {
        let Some(node) = self.config.as_mapping() else {
            return Ok(Vec::new());
        };
        let Some(raw) = node.get(serde_yaml::Value::String(ALLOWED_TOOLS_KEY.to_owned())) else {
            return Ok(Vec::new());
        };
        let seq = raw.as_sequence().ok_or_else(|| LoadError::AllowedTools {
            node: self.id.clone(),
            message: format!("`{ALLOWED_TOOLS_KEY}` must be a sequence of strings"),
        })?;
        let mut out = Vec::with_capacity(seq.len());
        for entry in seq {
            let s = entry.as_str().ok_or_else(|| LoadError::AllowedTools {
                node: self.id.clone(),
                message: format!("`{ALLOWED_TOOLS_KEY}` entries must be strings"),
            })?;
            out.push(s.to_owned());
        }
        Ok(out)
    }
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
