//! Wire extension-contributed flow node kinds into the host's
//! [`NodeKindRegistry`].
//!
//! Pure composition over the upstream
//! [`starter_ext_flow::contributed_node_kinds`] walker — no bespoke
//! manifest walking lives in rubix per SCOPE R2. For every
//! `Validated` extension in the sealed [`ExtensionRegistry`]:
//!
//! 1. Walk `contributes.nodes[]` via the upstream adapter.
//! 2. Materialise each entry's [`NodeBehavior`] (slice A binds the
//!    upstream [`UnboundNodeBehavior`] placeholder that returns
//!    `NodeError::Domain { code: "no_behaviour_bound", .. }` on
//!    invoke; slice B's `ProcessNodeProxy` swap is owned upstream).
//! 3. Register the behaviour on the shared [`NodeKindRegistry`] via
//!    its public `register` entry point — the path the upstream doc
//!    comment explicitly reserves for "outside the host (e.g. an
//!    extension via the future `starter-ext-flow` adapter)".
//!
//! The same composer is intended to absorb `contributes.skills` and
//! `contributes.flows` calls into the rubix host the moment rubix
//! grows the matching host-side registries (today rubix-skills only
//! ships content; a live `SkillRegistry` will land in the
//! starter-skills track).
//!
//! See `rubix/docs/design/extensions/README.md` for the end-to-end
//! bootflow this verb plugs into.

use starter_ext_flow::{contributed_node_kinds, unbound_behavior_factory, ContributeNodesError};
use starter_ext_host::ExtensionRegistry;
use starter_flow::registry::{NodeKindRegistry, RegistryError};

/// Errors surfaced from [`register_contributed_nodes`].
///
/// Each variant names the offending extension id (or `id_hint` if
/// the manifest never validated far enough to expose an id) so an
/// operator can locate the bundle without grepping logs.
#[derive(Debug, thiserror::Error)]
pub enum ExtensionsFlowError {
    /// The upstream walker rejected one of the `contributes.nodes[]`
    /// entries (typically an invalid reverse-DNS [`KindId`] that
    /// slipped past the host validator).
    #[error("walk contributes.nodes for extension `{id}`: {source}")]
    Walk {
        /// Extension id (or `id_hint` when unvalidated).
        id: String,
        /// Underlying walker failure.
        #[source]
        source: ContributeNodesError,
    },

    /// The host's [`NodeKindRegistry`] refused the contributed kind
    /// — most commonly a duplicate id (two extensions claiming the
    /// same kind) or a reserved-prefix violation
    /// (`starter.flow.*`). Both are upstream-validated *before* we
    /// land here, so reaching this variant is a real defect; we
    /// surface it as fatal-to-the-bundle but the boot path treats
    /// the error as soft (logs + skip) so the remaining extensions
    /// still come up.
    #[error("register contributed node kind `{kind}` from `{id}`: {source}")]
    Register {
        /// Extension id (or `id_hint`).
        id: String,
        /// Reverse-DNS kind id that failed.
        kind: String,
        /// Underlying registry failure.
        #[source]
        source: RegistryError,
    },
}

/// Walk every `Validated` extension in `extensions` and register its
/// `contributes.nodes[]` onto `kinds`. Returns the total number of
/// kinds registered.
///
/// Slice A semantics (current): every contributed kind binds to the
/// upstream [`unbound_behavior_factory`] placeholder — invoking a
/// flow that uses the kind returns the typed `no_behaviour_bound`
/// error. The placeholder is the load-bearing proof per
/// `DOCS/extensions/scope/FLOW-NODES.md` that the dynamic-registry
/// path is wired without the slice B `ProcessNodeProxy` wire.
///
/// Errors from one extension do NOT abort the loop — they are
/// returned wrapped in [`ExtensionsFlowError`] from the *first*
/// offender; callers that want best-effort behaviour can call this
/// per-extension. The current boot wiring treats any error as a
/// warn-and-continue per the existing extension-host boot pattern.
pub async fn register_contributed_nodes(
    extensions: &ExtensionRegistry,
    kinds: &NodeKindRegistry,
) -> Result<usize, ExtensionsFlowError> {
    let mut count: usize = 0;
    for record in extensions.iter_validated() {
        let Some(manifest) = record.manifest.as_ref() else {
            continue;
        };
        if manifest.contributes.nodes.is_empty() {
            continue;
        }
        let id_label = record
            .id
            .as_ref()
            .map(|i| i.as_str().to_owned())
            .unwrap_or_else(|| record.id_hint.clone());

        let walked =
            contributed_node_kinds(manifest, &record.bundle_dir, unbound_behavior_factory())
                .map_err(|source| ExtensionsFlowError::Walk {
                    id: id_label.clone(),
                    source,
                })?;
        for contributed in walked {
            let behavior = contributed.entry.behavior();
            let kind_str = behavior.kind_id().as_str().to_owned();
            kinds
                .register(behavior)
                .await
                .map_err(|source| ExtensionsFlowError::Register {
                    id: id_label.clone(),
                    kind: kind_str.clone(),
                    source,
                })?;
            tracing::info!(
                target: "rubix.boot.extensions.flow",
                extension_id = %id_label,
                kind = %kind_str,
                "registered extension-contributed node kind (slice A placeholder)"
            );
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_ext_host::Loader;
    use starter_flow_spi::node::KindId;
    use tempfile::tempdir;

    /// Lay down a minimal extension bundle declaring two contributed
    /// node kinds. Mirrors the upstream `tests/fixtures/com.nube.mqtt`
    /// fixture shape but inlined here so the rubix test stays
    /// self-contained (no cross-workspace fixture path).
    fn write_fixture(root: &std::path::Path) {
        let bundle = root.join("com.acme.demo");
        std::fs::create_dir_all(bundle.join("schemas")).unwrap();
        std::fs::write(
            bundle.join("block.yaml"),
            r#"v: 1
id: com.acme.demo
version: 0.1.0
display_name: "Acme Demo"
runtime: { kind: process, bin: ./bin/acme-demo }
contributes:
  nodes:
    - kind: com.acme.demo.echo
      settings_schema: schemas/echo.json
      facets: ["util"]
      streaming: false
    - kind: com.acme.demo.tick
      settings_schema: schemas/tick.json
      facets: ["trigger"]
      streaming: true
"#,
        )
        .unwrap();
        std::fs::write(bundle.join("schemas/echo.json"), "{\"type\":\"object\"}").unwrap();
        std::fs::write(bundle.join("schemas/tick.json"), "{\"type\":\"object\"}").unwrap();
    }

    #[tokio::test]
    async fn registers_every_contributed_kind() {
        let dir = tempdir().unwrap();
        write_fixture(dir.path());

        let mut registry = ExtensionRegistry::new();
        let records = Loader::scan(dir.path()).validate_all();
        let _ = Loader::commit(records, &mut registry);
        registry.seal();

        let kinds = NodeKindRegistry::new();
        let n = register_contributed_nodes(&registry, &kinds).await.unwrap();
        assert_eq!(n, 2);

        // Both kinds resolve to the placeholder behaviour.
        let echo = KindId::new("com.acme.demo.echo").unwrap();
        let tick = KindId::new("com.acme.demo.tick").unwrap();
        assert!(kinds.lookup(&echo).await.is_some());
        assert!(kinds.lookup(&tick).await.is_some());
    }

    #[tokio::test]
    async fn empty_registry_is_a_noop() {
        let mut registry = ExtensionRegistry::new();
        registry.seal();
        let kinds = NodeKindRegistry::new();
        let n = register_contributed_nodes(&registry, &kinds).await.unwrap();
        assert_eq!(n, 0);
    }
}
