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
    /// Cron expression for `trigger: schedule` flows. Surfaces at
    /// the YAML root (not on a node) so the rubix-agent boot
    /// seeder can register the schedule against `starter-flow-surfaces`'s
    /// durable cron table without descending into node configs. See
    /// `rubix-agent/src/boot/scheduler.rs`.
    #[serde(default)]
    pub cron_expr: Option<String>,
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

/// Key under `node.config` that carries the per-node CLI built-in
/// tool restriction. Distinct from [`ALLOWED_TOOLS_KEY`]:
/// `allowed_tools` is the MCP-bridged surface the model may dispatch
/// (forwarded as `--allowedTools` to the wrapped Claude CLI);
/// `tools` controls the CLI's own *built-in* catalogue (`Bash`,
/// `Read`, `Edit`, …) and is forwarded as `--tools`. `tools: []`
/// is the "MCP only, no built-ins" lockdown — the documented
/// default for every `ai-agent` flow node per stage 07.
pub const TOOLS_KEY: &str = "tools";

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

    /// Return the `config.tools` list as `Some(Vec<String>)` if the
    /// key is present (even if the list is empty), or `None` if the
    /// key is absent. The distinction matters: `tools: []` means
    /// "lock down to MCP only — no built-ins", while an absent key
    /// means "no per-node restriction, keep CLI default". Compare
    /// with [`Self::allowed_tools`] which collapses absent and
    /// empty into the same `Vec::new()`.
    ///
    /// Returns an error if `tools` is present but is not a YAML
    /// sequence of strings.
    pub fn tools(&self) -> Result<Option<Vec<String>>, LoadError> {
        let Some(node) = self.config.as_mapping() else {
            return Ok(None);
        };
        let Some(raw) = node.get(serde_yaml::Value::String(TOOLS_KEY.to_owned())) else {
            return Ok(None);
        };
        let seq = raw.as_sequence().ok_or_else(|| LoadError::Tools {
            node: self.id.clone(),
            message: format!("`{TOOLS_KEY}` must be a sequence of strings"),
        })?;
        let mut out = Vec::with_capacity(seq.len());
        for entry in seq {
            let s = entry.as_str().ok_or_else(|| LoadError::Tools {
                node: self.id.clone(),
                message: format!("`{TOOLS_KEY}` entries must be strings"),
            })?;
            out.push(s.to_owned());
        }
        Ok(Some(out))
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
