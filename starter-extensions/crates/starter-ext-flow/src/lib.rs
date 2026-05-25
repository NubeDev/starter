//! `starter-ext-flow` — the adapter that surfaces extension-contributed
//! flow artefacts into the host's flow runtime.
//!
//! Per [`DOCS/agent/SCOPE.md`] R-agent-4 the same adapter handles three
//! `contributes:` branches on a `block.yaml` manifest:
//!
//! - `contributes.flows`  — extension-shipped flow YAML files (later
//!   phase of the flow track; not wired here).
//! - `contributes.skills` — extension-shipped `SKILL.md` bundle
//!   directories (DOCS/agent/SKILLS.md R-agent-4 + Phase 6).
//! - `contributes.nodes`  — extension-supplied flow node kinds, wired
//!   in stage 1 (slice A) of the FLOW-NODES track. The descriptor
//!   surface is fully wired here; the proxy that routes `invoke()`
//!   onto a child process lands in slice B per
//!   `DOCS/extensions/scope/FLOW-NODES.md`.
//!
//! ## Trust matrix
//!
//! Skills surfaced through this adapter feed
//! [`starter_skills::SkillRegistry::extend`], which classifies
//! everything it receives as
//! [`Trust::Quarantined`][starter_skills::Trust::Quarantined]
//! regardless of the bundle's frontmatter `trust:` field
//! (DOCS/agent/SKILLS.md R-skills-3 row 3). An extension cannot
//! ship pre-approved skills; the operator must approve each
//! `(skill_id, bundle_hash)` explicitly through
//! [`SkillRegistry::approve`][starter_skills::SkillRegistry::approve].
//!
//! ## What this crate does **not** do
//!
//! - It does not load the extension manifest itself. That is
//!   `starter-ext-host`'s job; this adapter consumes the parsed
//!   [`Manifest`][starter_ext_spi::manifest::Manifest].
//! - It does not parse `SKILL.md`. That is `starter-skills`' job;
//!   this adapter only enumerates bundle directories and hands
//!   their paths to [`SkillRegistry::extend`].
//! - It does not build the [`SkillRegistry`] for the host. The
//!   host owns the registry lifecycle and reload cadence
//!   (R-skills-8); this adapter contributes a batch into a
//!   pre-existing builder or onto an already-built registry's
//!   next reload.
//! - It does not wire the runtime body of an extension-contributed
//!   node kind. Stage 1 hands every descriptor through with a
//!   placeholder [`UnboundNodeBehavior`] that returns the typed
//!   `no_behaviour_bound` error on invoke; slice B's
//!   `ProcessNodeProxy` replaces the placeholder over the
//!   `flow.node.invoke` wire method (R-flow-node-1, R-flow-node-5).

#![deny(missing_docs)]

pub mod process_proxy;

pub use process_proxy::ProcessNodeProxy;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use starter_ext_spi::manifest::Manifest;
use starter_flow_spi::node::{
    DynamicNodeKindEntry, KindId, NodeBehavior, NodeCtx, NodeDescriptor, NodeError, SlotMap,
};
use starter_skills::ContributedSkill;

/// Errors the [`contributed_skills`] walker can return.
///
/// Every variant carries the offending path so an operator can find
/// and fix the bundle without grepping logs.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ContributeSkillsError {
    /// A `contributes.skills[].dir` resolved to a path that is not a
    /// readable directory. Distinct from "directory exists but
    /// contains no bundles": the latter is **not** an error (an
    /// extension may legitimately ship zero skills under a declared
    /// directory while it iterates on them).
    #[error("contributes.skills[].dir {dir} is not a readable directory: {source}")]
    InvalidSkillsDir {
        /// The resolved (`extension_root + dir`) path that failed.
        dir: PathBuf,
        /// Underlying I/O error from `read_dir`.
        #[source]
        source: std::io::Error,
    },
}

/// Errors the [`contributed_node_kinds`] walker can return.
///
/// Stage 1 (slice A) only surfaces parse-level failures from the kind
/// id — namespace/reserved-prefix checks live in
/// `starter-ext-host::validate` and run *before* the host hands the
/// manifest to this adapter (R-flow-node-3). The variant set is open
/// (`#[non_exhaustive]`) so slice B can add proxy-construction
/// failures without a breaking change.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ContributeNodesError {
    /// A `contributes.nodes[].kind` string did not parse as a valid
    /// reverse-DNS [`KindId`]. The host's pre-validator (`R-flow-node-3`)
    /// catches namespace-ownership violations; this one catches
    /// syntactic shape (uppercase, missing dots, illegal chars).
    #[error("contributes.nodes[].kind {kind:?} is not a valid reverse-DNS KindId: {source}")]
    InvalidKindId {
        /// The offending kind id.
        kind: String,
        /// Underlying validation failure from [`KindId::new`].
        #[source]
        source: starter_flow_spi::node::IdError,
    },
}

/// Walk every `contributes.skills[].dir` declared on `manifest` and
/// return the [`ContributedSkill`] values the host should feed into
/// [`SkillRegistry::extend`][starter_skills::SkillRegistry::extend].
///
/// Each `dir` is resolved relative to `extension_root` (the directory
/// the extension's `block.yaml` lives in). Inside each resolved
/// directory the adapter looks for **one level** of sub-directories
/// that contain a `SKILL.md` file — the same shape
/// [`SkillRegistry::load_dir`][starter_skills::SkillRegistry::builder]
/// uses for the host's own skills tree. Sub-directories without a
/// `SKILL.md` are ignored so a stray `node_modules/` cannot accidentally
/// register a bundle.
///
/// Bundle directories are returned in deterministic order (sorted by
/// path) so downstream errors like
/// [`LoadError::DuplicateSkillId`][starter_skills::LoadError::DuplicateSkillId]
/// reproduce across runs.
///
/// **Trust:** every entry returned here will land
/// [`Trust::Quarantined`][starter_skills::Trust::Quarantined] when
/// passed to [`SkillRegistry::extend`][starter_skills::SkillRegistry::extend]
/// regardless of the bundle's frontmatter `trust:` field
/// (R-skills-3 row 3).
pub fn contributed_skills(
    manifest: &Manifest,
    extension_root: &Path,
) -> Result<Vec<ContributedSkill>, ContributeSkillsError> {
    let mut out: Vec<ContributedSkill> = Vec::new();
    for entry in &manifest.contributes.skills {
        let resolved = extension_root.join(&entry.dir);
        let read = std::fs::read_dir(&resolved).map_err(|source| {
            ContributeSkillsError::InvalidSkillsDir {
                dir: resolved.clone(),
                source,
            }
        })?;
        let mut bundle_dirs: Vec<PathBuf> = Vec::new();
        for dirent in read {
            let dirent = dirent.map_err(|source| ContributeSkillsError::InvalidSkillsDir {
                dir: resolved.clone(),
                source,
            })?;
            let file_type =
                dirent
                    .file_type()
                    .map_err(|source| ContributeSkillsError::InvalidSkillsDir {
                        dir: resolved.clone(),
                        source,
                    })?;
            if !file_type.is_dir() {
                continue;
            }
            let candidate = dirent.path();
            if candidate.join("SKILL.md").is_file() {
                bundle_dirs.push(candidate);
            }
        }
        bundle_dirs.sort();
        tracing::debug!(
            target: "starter_ext_flow::skills",
            extension_id = %manifest.id.as_str(),
            dir = %resolved.display(),
            count = bundle_dirs.len(),
            "discovered contributes.skills bundles (will land quarantined)"
        );
        for dir in bundle_dirs {
            out.push(ContributedSkill::new(dir));
        }
    }
    Ok(out)
}

/// Resolved metadata for one extension-contributed node kind, paired
/// with its descriptor entry.
///
/// Carries the bundle-relative paths the host's REST surface needs
/// (`/api/node-kinds/<kind>/settings-schema`,
///  `/api/node-kinds/<kind>/description`) resolved against the
/// extension's bundle root, plus the advisory `streaming` flag, the
/// editor-facing facet tags, and the extension id that owns the kind.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ContributedNodeKindMeta {
    /// The extension id that contributed this kind. Surfaced on
    /// `GET /api/node-kinds` so the editor can group kinds by their
    /// providing extension.
    pub extension_id: String,
    /// Reverse-DNS kind id.
    pub kind: String,
    /// Absolute path on disk to the JSON Schema describing the kind's
    /// `settings:` body.
    pub settings_schema_path: PathBuf,
    /// Absolute path on disk to the static markdown description, if
    /// the manifest declared one.
    pub description_path: Option<PathBuf>,
    /// Palette facet tags from the manifest. Free-form.
    pub facets: Vec<String>,
    /// Advisory streaming flag (R-flow-node-1: no new streaming
    /// shape; the proxy in slice B uses this to size its channels).
    pub streaming: bool,
}

impl ContributedNodeKindMeta {
    /// Construct a [`ContributedNodeKindMeta`]. External callers go
    /// through this constructor rather than the struct literal because
    /// the type is `#[non_exhaustive]`; slice B is expected to add
    /// fields (e.g. `auth: AuthGate`) without a breaking change.
    pub fn new(
        extension_id: impl Into<String>,
        kind: impl Into<String>,
        settings_schema_path: PathBuf,
        description_path: Option<PathBuf>,
        facets: Vec<String>,
        streaming: bool,
    ) -> Self {
        Self {
            extension_id: extension_id.into(),
            kind: kind.into(),
            settings_schema_path,
            description_path,
            facets,
            streaming,
        }
    }
}

/// One walker output: the dynamic-registry entry plus the bundle
/// metadata the host's REST surface needs to serve schema/description
/// files.
pub struct ContributedNodeKind {
    /// Entry to insert into a
    /// [`DynamicNodeKindRegistry`][starter_flow_spi::node::DynamicNodeKindRegistry].
    pub entry: DynamicNodeKindEntry,
    /// Resolved on-disk paths + manifest metadata for the host's REST
    /// surface to read at request time.
    pub meta: ContributedNodeKindMeta,
}

impl std::fmt::Debug for ContributedNodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContributedNodeKind")
            .field("meta", &self.meta)
            .finish_non_exhaustive()
    }
}

/// Walk every `contributes.nodes[]` declared on `manifest` and return
/// the [`ContributedNodeKind`] values the host should fold into its
/// [`DynamicNodeKindRegistry`][starter_flow_spi::node::DynamicNodeKindRegistry]
/// (alongside the metadata it needs to serve schema/description files
/// over REST).
///
/// `extension_root` is the directory the extension's `block.yaml`
/// lives in; `settings_schema` and `description_file` paths are
/// resolved against it (they are NOT read here — the REST surface
/// streams them lazily at request time so an unmodified bundle on
/// disk reflects in the next response without a reload).
///
/// `behavior_factory` is the closure that materialises an
/// [`Arc<dyn NodeBehavior>`] per kind. Stage 1 (slice A) callers
/// pass [`unbound_behavior_factory`], producing the typed
/// `no_behaviour_bound` error on invoke. Stage 2 (slice B) callers
/// close over a `SupervisorHandle` and construct a `ProcessNodeProxy`
/// per kind — that swap is the load-bearing acceptance gate for the
/// FLOW-NODES track.
///
/// The host's reverse-DNS namespace validator
/// (`starter_ext_host::validate::validate_manifest`) is expected to
/// have run first — that's where R-flow-node-3 rejects reserved
/// prefixes and non-descendant kinds. This walker only catches
/// syntactic [`KindId`] failures the validator missed (defence in
/// depth) and otherwise trusts the manifest.
///
/// By convention the i18n catalog keys for each kind follow the
/// reverse-DNS template `<kind>.{label,summary,help}`. The walker
/// uses that shape to populate the descriptor's `*_key` fields so
/// the host's i18n bundle (or an extension-shipped `contributes.i18n`
/// catalog) can resolve them without per-kind manifest plumbing.
pub fn contributed_node_kinds<F>(
    manifest: &Manifest,
    extension_root: &Path,
    behavior_factory: F,
) -> Result<Vec<ContributedNodeKind>, ContributeNodesError>
where
    F: Fn(&KindId) -> Arc<dyn NodeBehavior> + Clone + Send + Sync + 'static,
{
    let mut out: Vec<ContributedNodeKind> = Vec::with_capacity(manifest.contributes.nodes.len());
    for node in &manifest.contributes.nodes {
        let kind_id = KindId::new(node.kind.clone()).map_err(|source| {
            ContributeNodesError::InvalidKindId {
                kind: node.kind.clone(),
                source,
            }
        })?;

        let label_key = format!("{}.label", node.kind);
        let summary_key = format!("{}.summary", node.kind);
        let help_key = format!("{}.help", node.kind);
        let descriptor =
            NodeDescriptor::new_owned(node.kind.clone(), label_key, summary_key, help_key);

        let kind_for_factory = kind_id.clone();
        let factory = behavior_factory.clone();
        let entry = DynamicNodeKindEntry::new(descriptor, move || factory(&kind_for_factory));

        let meta = ContributedNodeKindMeta {
            extension_id: manifest.id.as_str().to_owned(),
            kind: node.kind.clone(),
            settings_schema_path: extension_root.join(&node.settings_schema),
            description_path: node
                .description_file
                .as_ref()
                .map(|p| extension_root.join(p)),
            facets: node.facets.clone(),
            streaming: node.streaming,
        };

        tracing::debug!(
            target: "starter_ext_flow::nodes",
            extension_id = %manifest.id.as_str(),
            kind = %node.kind,
            streaming = node.streaming,
            "discovered contributes.nodes entry"
        );

        out.push(ContributedNodeKind { entry, meta });
    }
    Ok(out)
}

/// Convenience: a [`behavior_factory`][contributed_node_kinds]
/// callable suitable for stage 1 (slice A) callers. Every kind
/// materialises an [`UnboundNodeBehavior`] that returns the typed
/// `no_behaviour_bound` error on invoke.
pub fn unbound_behavior_factory(
) -> impl Fn(&KindId) -> Arc<dyn NodeBehavior> + Clone + Send + Sync + 'static {
    |kind: &KindId| -> Arc<dyn NodeBehavior> { Arc::new(UnboundNodeBehavior::new(kind.clone())) }
}

/// Placeholder [`NodeBehavior`] used by slice A's walker.
///
/// Returns [`NodeError::Domain { code: "no_behaviour_bound", .. }`]
/// on every invoke. The presence of this error in a fired flow is the
/// proof that the dynamic-registry path is wired end-to-end *without*
/// the slice B `ProcessNodeProxy` and supervisor wire — per the
/// FLOW-NODES SCOPE's slice A acceptance criterion.
pub struct UnboundNodeBehavior {
    kind: KindId,
}

impl UnboundNodeBehavior {
    /// Construct a placeholder behaviour for a specific kind id.
    pub fn new(kind: KindId) -> Self {
        Self { kind }
    }
}

#[async_trait]
impl NodeBehavior for UnboundNodeBehavior {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, _input: SlotMap) -> Result<SlotMap, NodeError> {
        Err(NodeError::Domain {
            code: "no_behaviour_bound",
            message: format!(
                "extension-contributed node kind {:?} has no behaviour bound yet \
                 (slice A placeholder; ProcessNodeProxy lands in slice B per \
                 DOCS/extensions/scope/FLOW-NODES.md R-flow-node-5)",
                self.kind.as_str()
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_with_one_node() -> Manifest {
        let yaml = r#"
v: 1
id: com.nube.mqtt
version: 0.1.0
display_name: "MQTT"
runtime: { kind: process, bin: ./bin/mqtt-driver }
contributes:
  nodes:
    - kind: com.nube.mqtt.publish
      settings_schema: schemas/publish.json
      description_file: docs/publish.md
      facets: ["transport"]
      streaming: false
    - kind: com.nube.mqtt.subscribe
      settings_schema: schemas/subscribe.json
      streaming: true
"#;
        serde_yaml::from_str(yaml).expect("manifest parses")
    }

    #[test]
    fn walker_resolves_paths_and_meta() {
        let m = manifest_with_one_node();
        let root = Path::new("/tmp/com.nube.mqtt");
        let kinds = contributed_node_kinds(&m, root, unbound_behavior_factory()).unwrap();
        assert_eq!(kinds.len(), 2);

        let pub_ = &kinds[0];
        assert_eq!(pub_.meta.extension_id, "com.nube.mqtt");
        assert_eq!(pub_.meta.kind, "com.nube.mqtt.publish");
        assert_eq!(
            pub_.meta.settings_schema_path,
            root.join("schemas/publish.json")
        );
        assert_eq!(
            pub_.meta.description_path,
            Some(root.join("docs/publish.md"))
        );
        assert_eq!(pub_.meta.facets, vec!["transport"]);
        assert!(!pub_.meta.streaming);

        let sub = &kinds[1];
        assert_eq!(sub.meta.kind, "com.nube.mqtt.subscribe");
        assert!(sub.meta.description_path.is_none());
        assert!(sub.meta.streaming);
    }

    #[test]
    fn walker_descriptor_uses_kind_template_for_i18n_keys() {
        let m = manifest_with_one_node();
        let root = Path::new("/tmp/com.nube.mqtt");
        let kinds = contributed_node_kinds(&m, root, unbound_behavior_factory()).unwrap();
        let d = kinds[0].entry.descriptor();
        assert_eq!(d.kind, "com.nube.mqtt.publish");
        assert_eq!(d.label_key, "com.nube.mqtt.publish.label");
        assert_eq!(d.summary_key, "com.nube.mqtt.publish.summary");
        assert_eq!(d.help_key, "com.nube.mqtt.publish.help");
    }

    /// The placeholder behaviour returns the typed
    /// `no_behaviour_bound` error — slice A's load-bearing proof that
    /// the dynamic-registry path is wired without a supervisor wire.
    #[tokio::test]
    async fn placeholder_behavior_returns_no_behaviour_bound() {
        let kind = KindId::new("com.nube.mqtt.publish").unwrap();
        let behavior = UnboundNodeBehavior::new(kind.clone());
        assert_eq!(behavior.kind_id().as_str(), kind.as_str());

        // Build a minimal NodeCtx for the invoke. The exact field
        // values do not matter — the body returns early without
        // reading them.
        struct NoCancel;
        impl starter_flow_spi::Cancel for NoCancel {
            fn is_cancelled(&self) -> bool {
                false
            }
            fn cancelled<'a>(
                &'a self,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
                Box::pin(std::future::pending())
            }
        }
        let node_id = starter_flow_spi::node::NodeId::new("flow.test.mqtt").unwrap();
        let cancel = NoCancel;
        let ctx = NodeCtx::new(
            starter_flow_spi::flow::RunId::new(),
            &node_id,
            &cancel,
            starter_flow_spi::skill::SkillSelection::NONE,
            &starter_flow_spi::state::NOOP_NODE_STATE_STORE,
        );

        let err = behavior
            .invoke(ctx, SlotMap::new())
            .await
            .expect_err("placeholder must return Err");
        match err {
            NodeError::Domain { code, message } => {
                assert_eq!(code, "no_behaviour_bound");
                assert!(message.contains("com.nube.mqtt.publish"));
            }
            other => panic!("expected NodeError::Domain; got {other:?}"),
        }
    }
}
