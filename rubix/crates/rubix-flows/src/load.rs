//! YAML → [`FlowBody`] loader for every bundled rubix flow.
//!
//! [`load_all`] walks the embedded [`crate::BUNDLED`] directory,
//! deserialises each `*.yaml` file into the rubix-surface
//! [`RubixFlowYaml`] shape, and converts the result into the typed
//! [`starter_flow::definition::body::FlowBody`] the host's
//! `FlowRegistry` accepts. One YAML in, one
//! `(FlowId, FlowRevisionId, FlowBody)` out — the agent binary loops
//! the result into `FlowRegistry::register` and `FlowAsTool` does the
//! rest.
//!
//! ## YAML shape
//!
//! The on-disk shape is the human-authoring surface, not the typed
//! starter-flow body. The two differ on three fields:
//!
//! | YAML key | starter-flow body key | Notes                              |
//! |----------|-----------------------|------------------------------------|
//! | `id`     | `flow_id`             | reverse-DNS (e.g. `com.rubix.foo`) |
//! | `nodes[].config` | `nodes[].settings` | per-node bag, opaque here |
//! | `nodes[].id` (short) | `NodeId` (reverse-DNS) | prefixed with `com.rubix.` during conversion |
//! | `nodes[].kind: ai-agent` | `KindId` ([`AI_AGENT_KIND_ID`]) | mapped to the registered reverse-DNS kind id |
//! | `trigger`, `description` | (carried as flow-level metadata) | description becomes the MCP tool description upstream |
//!
//! ## Block-A scope
//!
//! Stage 1 of the agent-runtime job only owns the YAML → body
//! conversion; the actual `ai-agent` [`NodeBehavior`] is bound by
//! Block C (rubix wiring of `starter-flow-node-loop`'s
//! `AiAgentNode`). Until then the host registers a stub behaviour
//! under [`AI_AGENT_KIND_ID`] so registration succeeds and
//! `mcp_tools=6`; invoking any flow surfaces a "not wired yet"
//! `NodeError` — that is expected and is what Block C fixes.

use std::sync::Arc;

use serde::Deserialize;
use thiserror::Error;

use starter_flow::definition::body::{FlowBody, LinkDecl, NodeDecl};
use starter_flow_spi::flow::{FlowId, FlowRevisionId};
use starter_flow_spi::node::{IdError, KindId, NodeId};

use crate::BUNDLED;

/// YAML-surface string for the agent kind every bundled rubix flow
/// is rooted at. This is the literal contributors type in their
/// `*.yaml` files (`kind: ai-agent`).
pub const AI_AGENT_KIND_YAML: &str = "ai-agent";

/// Registered reverse-DNS [`KindId`] the YAML-surface `ai-agent`
/// string maps to. The starter-flow `NodeKindRegistry` requires a
/// reverse-DNS id; `ai-agent` alone is not (no dot), so the loader
/// rewrites the kind during conversion. Block B replaces this with
/// `starter-flow-node-loop`'s `KIND_ID` constant; until then rubix
/// owns the id under its own namespace.
pub const AI_AGENT_KIND_ID: &str = "com.rubix.ai-agent";

/// Prefix prepended to short YAML node ids (`agent`, `check`, …) to
/// produce reverse-DNS [`NodeId`]s. Per-flow uniqueness is enforced
/// by the [`FlowRegistry`] at register time; the prefix only exists
/// to satisfy the reverse-DNS shape required by the id type.
pub const NODE_ID_PREFIX: &str = "com.rubix";

/// Default seed slot name written by the
/// [`FlowRegistration::with_default_adapters`] adapter pair every
/// bundled flow uses. Also added to the root node's `triggers` list
/// so the engine fires the node when the slot is seeded.
pub const DEFAULT_SEED_SLOT: &str = "payload";

/// Default terminal slot name read back at end-of-run by the
/// default output adapter. Pair with [`DEFAULT_SEED_SLOT`].
pub const DEFAULT_OUTPUT_SLOT: &str = "out";

/// Errors surfaced by the loader.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LoadError {
    /// A bundled file's bytes were not valid UTF-8.
    #[error("flow `{path}`: not valid UTF-8: {source}")]
    Utf8 {
        path: String,
        #[source]
        source: std::str::Utf8Error,
    },
    /// `serde_yaml` rejected the file shape.
    #[error("flow `{path}`: YAML shape: {source}")]
    Yaml {
        path: String,
        #[source]
        source: serde_yaml::Error,
    },
    /// The flow id, node id, or kind id failed reverse-DNS
    /// validation.
    #[error("flow `{path}`: id `{id}`: {source}")]
    Id {
        path: String,
        id: String,
        #[source]
        source: IdError,
    },
    /// The flow body had zero nodes — the loader has nothing to
    /// hook adapters onto.
    #[error("flow `{path}`: must declare at least one node")]
    EmptyBody { path: String },
}

/// The YAML-surface shape every bundled `flows/*.yaml` file
/// deserialises into. This is intentionally separate from
/// [`FlowBody`] so contributors can author the lighter shape (`id`,
/// `config`, short `id`s on nodes) while the agent binary still
/// hands `FlowRegistry` the typed body it expects.
#[derive(Debug, Clone, Deserialize)]
pub struct RubixFlowYaml {
    /// Flow id (reverse-DNS, e.g. `com.rubix.scheduled-system-check`).
    pub id: String,
    /// Human-readable description; carried up to the MCP tool
    /// catalogue by the agent binary.
    #[serde(default)]
    pub description: Option<String>,
    /// Trigger declaration (`explicit`, `schedule(cron = "...")`, …).
    /// Currently informational — Phase-4 cron wiring uses it.
    #[serde(default)]
    pub trigger: Option<String>,
    /// Declared nodes (the loader only supports single-node flows
    /// today; multi-node is accepted at parse time but the default
    /// adapters only wire the root node's slots).
    #[serde(default)]
    pub nodes: Vec<RubixNodeYaml>,
    /// Slot-to-slot links. Carried through to the typed body as
    /// [`LinkDecl`]s; today every bundled flow ships `links: []`.
    #[serde(default)]
    pub links: Vec<RubixLinkYaml>,
}

/// One node entry inside a [`RubixFlowYaml`].
#[derive(Debug, Clone, Deserialize)]
pub struct RubixNodeYaml {
    /// Short authoring id (`agent`, `check`, …). Rewritten to a
    /// reverse-DNS [`NodeId`] during conversion.
    pub id: String,
    /// Node kind — every bundled flow uses [`AI_AGENT_KIND_YAML`]
    /// today.
    pub kind: String,
    /// Per-node config bag. Forwarded verbatim into
    /// [`NodeDecl::settings`] as a JSON value.
    #[serde(default)]
    pub config: serde_yaml::Value,
}

/// One link entry inside a [`RubixFlowYaml`].
#[derive(Debug, Clone, Deserialize)]
pub struct RubixLinkYaml {
    /// Source slot, formatted as `<node_id>.<slot_name>` (uses the
    /// *converted* reverse-DNS node ids).
    pub from: String,
    /// Destination slot, formatted as `<node_id>.<slot_name>`.
    pub to: String,
}

/// Parse one YAML byte slice into the surface [`RubixFlowYaml`]
/// shape. Exposed primarily for the unit tests; callers that want
/// the typed body should use [`convert`] or [`load_all`] instead.
pub fn parse_yaml(path: &str, bytes: &[u8]) -> Result<RubixFlowYaml, LoadError> {
    // serde_yaml is happy with raw bytes; the explicit utf8 check
    // surfaces a nicer error message if a binary file slips into the
    // bundle directory.
    std::str::from_utf8(bytes).map_err(|e| LoadError::Utf8 {
        path: path.to_owned(),
        source: e,
    })?;
    serde_yaml::from_slice(bytes).map_err(|e| LoadError::Yaml {
        path: path.to_owned(),
        source: e,
    })
}

/// Convert one parsed [`RubixFlowYaml`] into the
/// `(FlowId, FlowRevisionId, FlowBody)` triple the agent binary
/// hands to `FlowRegistry::register`. A fresh
/// [`FlowRevisionId`] is minted on every call — the bundle is a
/// compile-time artefact, so each boot is a fresh registration.
pub fn convert(
    path: &str,
    yaml: RubixFlowYaml,
) -> Result<(FlowId, FlowRevisionId, FlowBody), LoadError> {
    let flow_id = FlowId::new(yaml.id.clone()).map_err(|e| LoadError::Id {
        path: path.to_owned(),
        id: yaml.id.clone(),
        source: e,
    })?;
    if yaml.nodes.is_empty() {
        return Err(LoadError::EmptyBody {
            path: path.to_owned(),
        });
    }

    let mut body = FlowBody::new(flow_id.clone());
    for node in yaml.nodes {
        let qualified_node_id = format!("{NODE_ID_PREFIX}.{}", node.id);
        let node_id = NodeId::new(qualified_node_id.clone()).map_err(|e| LoadError::Id {
            path: path.to_owned(),
            id: qualified_node_id,
            source: e,
        })?;
        // Map the YAML-surface kind string to its registered
        // reverse-DNS id. Today the only mapping is `ai-agent` →
        // [`AI_AGENT_KIND_ID`]; unknown strings pass through as-is
        // and let `KindId::new` validate (so a typo surfaces as an
        // [`IdError`] at load time, not at resolve time).
        let kind_str = if node.kind == AI_AGENT_KIND_YAML {
            AI_AGENT_KIND_ID.to_owned()
        } else {
            node.kind.clone()
        };
        let kind = KindId::new(kind_str.clone()).map_err(|e| LoadError::Id {
            path: path.to_owned(),
            id: kind_str,
            source: e,
        })?;

        let settings = serde_json::to_value(&node.config).unwrap_or_else(|_| {
            // YAML round-trip into JSON only fails on non-utf8 keys
            // or other shapes serde_json can't model; the bundled
            // flows are hand-authored, so swallow rather than fail
            // the whole boot.
            serde_json::Value::Object(serde_json::Map::new())
        });

        let mut decl = NodeDecl::new(node_id, kind);
        decl.settings = settings;
        decl.triggers = vec![DEFAULT_SEED_SLOT.to_owned()];
        body.nodes.push(decl);
    }

    body.links = yaml
        .links
        .into_iter()
        .map(|l| LinkDecl::new(l.from, l.to))
        .collect();

    Ok((flow_id, FlowRevisionId::new(), body))
}

/// Walk [`crate::BUNDLED`] and return one
/// `(FlowId, FlowRevisionId, FlowBody)` triple per `*.yaml` file
/// under `flows/`. Subdirectories are flattened. Order is
/// directory-order (deterministic per `include_dir` semantics).
pub fn load_all() -> Result<Vec<(FlowId, FlowRevisionId, FlowBody)>, LoadError> {
    let mut out = Vec::new();
    walk(&BUNDLED, &mut out)?;
    Ok(out)
}

fn walk(
    dir: &include_dir::Dir<'_>,
    out: &mut Vec<(FlowId, FlowRevisionId, FlowBody)>,
) -> Result<(), LoadError> {
    for entry in dir.entries() {
        match entry {
            include_dir::DirEntry::File(f) => {
                let path = f.path().to_string_lossy().into_owned();
                let is_yaml = path.ends_with(".yaml") || path.ends_with(".yml");
                if !is_yaml {
                    continue;
                }
                let yaml = parse_yaml(&path, f.contents())?;
                out.push(convert(&path, yaml)?);
            }
            include_dir::DirEntry::Dir(sub) => walk(sub, out)?,
        }
    }
    Ok(())
}

/// Convenience: convert a slice of bundled triples into a Vec of
/// [`Arc<FlowBody>`] for callers that want shared ownership of each
/// body across hot-reload generations. Today only the boot path
/// consumes this; left in for symmetry with starter's flow registry
/// APIs.
pub fn into_arcs(
    triples: Vec<(FlowId, FlowRevisionId, FlowBody)>,
) -> Vec<(FlowId, FlowRevisionId, Arc<FlowBody>)> {
    triples
        .into_iter()
        .map(|(id, rev, body)| (id, rev, Arc::new(body)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The six goal flows the spec mandates are bundled. The unit
    /// test cross-checks `load_all()` returns exactly this set so a
    /// missing or renamed file shows up here, not at MCP boot.
    const EXPECTED_FLOW_IDS: &[&str] = &[
        "com.rubix.scheduled-system-check",
        "com.rubix.weekly-report",
        "com.rubix.dashboard-assistant",
        "com.rubix.flow-programmer",
        "com.rubix.clickhouse-ruler",
        "com.rubix.user-admin",
    ];

    #[test]
    fn every_bundled_yaml_parses_with_ai_agent_root() {
        // Parse each file directly so we can inspect the on-disk
        // surface (short node id, literal `ai-agent` kind string)
        // before conversion rewrites them.
        let mut seen: Vec<String> = Vec::new();
        for file in BUNDLED.files() {
            let path = file.path().to_string_lossy().into_owned();
            if !(path.ends_with(".yaml") || path.ends_with(".yml")) {
                continue;
            }
            let yaml = parse_yaml(&path, file.contents()).expect("yaml parses");
            assert!(
                !yaml.nodes.is_empty(),
                "flow `{path}` declares zero nodes",
            );
            let root = &yaml.nodes[0];
            assert_eq!(
                root.kind, AI_AGENT_KIND_YAML,
                "flow `{path}` root node kind must be `{AI_AGENT_KIND_YAML}` (got `{}`)",
                root.kind,
            );
            assert!(
                !root.id.is_empty(),
                "flow `{path}` root node must have an id",
            );
            seen.push(yaml.id);
        }
        seen.sort();
        let mut expected: Vec<String> =
            EXPECTED_FLOW_IDS.iter().map(|s| (*s).to_owned()).collect();
        expected.sort();
        assert_eq!(seen, expected, "bundled flow id set drifted from spec");
    }

    #[test]
    fn load_all_converts_every_bundled_flow() {
        let triples = load_all().expect("load_all succeeds");
        assert_eq!(
            triples.len(),
            EXPECTED_FLOW_IDS.len(),
            "load_all must surface every bundled flow",
        );
        for (flow_id, _rev, body) in &triples {
            assert_eq!(
                &body.flow_id, flow_id,
                "FlowBody.flow_id must match the returned FlowId",
            );
            assert!(
                !body.nodes.is_empty(),
                "converted body for `{flow_id}` has zero nodes",
            );
            let root = &body.nodes[0];
            assert_eq!(
                root.kind.as_str(),
                AI_AGENT_KIND_ID,
                "root node kind must be rewritten to {AI_AGENT_KIND_ID}",
            );
            assert!(
                root.id.as_str().starts_with(NODE_ID_PREFIX),
                "node id `{}` must carry the {NODE_ID_PREFIX} prefix",
                root.id,
            );
            assert!(
                root.triggers.iter().any(|t| t == DEFAULT_SEED_SLOT),
                "root node must list `{DEFAULT_SEED_SLOT}` as a trigger slot",
            );
        }
        let mut seen: Vec<String> = triples
            .iter()
            .map(|(id, _, _)| id.to_string())
            .collect();
        seen.sort();
        let mut expected: Vec<String> =
            EXPECTED_FLOW_IDS.iter().map(|s| (*s).to_owned()).collect();
        expected.sort();
        assert_eq!(seen, expected);
    }
}
