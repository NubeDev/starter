//! Slice A acceptance fixture for the FLOW-NODES track.
//!
//! Loads `tests/fixtures/com.nube.mqtt/block.yaml`, walks it through
//! `starter_ext_flow::contributed_node_kinds` with the slice A
//! placeholder factory, and verifies:
//!
//! 1. The manifest parses cleanly (no `deny_unknown_fields` violations
//!    on the new `contributes.nodes` block — R-flow-node-4).
//! 2. The walker resolves `settings_schema` and `description_file`
//!    paths against the bundle root and exposes them on the
//!    `ContributedNodeKindMeta` shape the host's REST surface needs
//!    (`GET /api/node-kinds/<kind>/{settings-schema,description}`).
//! 3. Each [`DynamicNodeKindEntry`] produces a placeholder
//!    [`UnboundNodeBehavior`] whose `invoke` returns
//!    `NodeError::Domain { code: "no_behaviour_bound", .. }` — the
//!    load-bearing slice A acceptance proof that the dynamic-registry
//!    path is wired end-to-end without the slice B `ProcessNodeProxy`.

use std::path::{Path, PathBuf};

use starter_ext_flow::{contributed_node_kinds, unbound_behavior_factory};
use starter_ext_spi::Manifest;
use starter_flow_spi::node::{
    DynamicNodeKindRegistry, KindId, NodeBehavior, NodeCtx, NodeError, NodeId, NodeKindRegistry,
    SlotMap,
};

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/com.nube.mqtt")
}

fn load_manifest() -> Manifest {
    let yaml = std::fs::read_to_string(fixture_root().join("block.yaml"))
        .expect("fixture block.yaml is readable");
    serde_yaml::from_str(&yaml).expect("fixture block.yaml parses")
}

#[test]
fn fixture_manifest_parses_with_contributes_nodes() {
    let m = load_manifest();
    assert_eq!(m.id.as_str(), "com.nube.mqtt");
    assert_eq!(m.contributes.nodes.len(), 2);
    let kinds: Vec<&str> = m
        .contributes
        .nodes
        .iter()
        .map(|n| n.kind.as_str())
        .collect();
    assert_eq!(
        kinds,
        vec!["com.nube.mqtt.publish", "com.nube.mqtt.subscribe"]
    );
}

#[test]
fn walker_resolves_paths_against_extension_root() {
    let m = load_manifest();
    let root = fixture_root();
    let kinds = contributed_node_kinds(&m, &root, unbound_behavior_factory()).unwrap();
    assert_eq!(kinds.len(), 2);

    let publish = &kinds[0];
    assert_eq!(publish.meta.kind, "com.nube.mqtt.publish");
    assert!(
        publish.meta.settings_schema_path.is_file(),
        "publish settings schema must exist on disk: {}",
        publish.meta.settings_schema_path.display()
    );
    assert!(
        publish
            .meta
            .description_path
            .as_ref()
            .map(|p| p.is_file())
            .unwrap_or(false),
        "publish description file must exist on disk"
    );
    assert_eq!(publish.meta.facets, vec!["transport", "io"]);
    assert!(!publish.meta.streaming);

    let subscribe = &kinds[1];
    assert_eq!(subscribe.meta.kind, "com.nube.mqtt.subscribe");
    assert!(subscribe.meta.streaming);
    assert!(subscribe.meta.facets.contains(&"trigger".to_owned()));
}

#[test]
fn dynamic_registry_round_trips_fixture_entries() {
    let m = load_manifest();
    let root = fixture_root();
    let kinds = contributed_node_kinds(&m, &root, unbound_behavior_factory()).unwrap();

    let mut reg = DynamicNodeKindRegistry::new();
    for k in kinds {
        reg.insert(k.entry);
    }

    let pub_kind = KindId::new("com.nube.mqtt.publish").unwrap();
    let d = reg.lookup(&pub_kind).expect("publish descriptor present");
    assert_eq!(d.kind, "com.nube.mqtt.publish");
    assert_eq!(d.label_key, "com.nube.mqtt.publish.label");
    assert_eq!(d.summary_key, "com.nube.mqtt.publish.summary");
    assert_eq!(d.help_key, "com.nube.mqtt.publish.help");

    let entries = reg.all();
    assert_eq!(entries.len(), 2);
}

/// Slice A acceptance: firing a flow that uses an extension-contributed
/// kind returns the typed `no_behaviour_bound` error — the proof that
/// the dynamic-registry path is wired without the slice B supervisor
/// wire (per `DOCS/extensions/scope/FLOW-NODES.md` § slice A).
#[tokio::test]
async fn placeholder_invoke_returns_no_behaviour_bound() {
    let m = load_manifest();
    let root = fixture_root();
    let kinds = contributed_node_kinds(&m, &root, unbound_behavior_factory()).unwrap();
    let entry = &kinds[0].entry;
    let behavior: std::sync::Arc<dyn NodeBehavior> = entry.behavior();

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

    let node = NodeId::new("flow.demo.mqtt-publish").unwrap();
    let cancel = NoCancel;
    let ctx = NodeCtx::new(
        starter_flow_spi::flow::RunId::new(),
        &node,
        &cancel,
        starter_flow_spi::skill::SkillSelection::NONE,
        &starter_flow_spi::state::NOOP_NODE_STATE_STORE,
    );

    let err = behavior
        .invoke(ctx, SlotMap::new())
        .await
        .expect_err("slice A placeholder must error on invoke");
    match err {
        NodeError::Domain { code, message } => {
            assert_eq!(code, "no_behaviour_bound");
            assert!(message.contains("com.nube.mqtt.publish"));
        }
        other => panic!("expected NodeError::Domain; got {other:?}"),
    }
}
