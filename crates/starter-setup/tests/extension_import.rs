//! P5 acceptance: the `contributes.setup_templates[]` import path imports the
//! REAL `com.acme.devices` bundle template (DOCS §9), validating it against the
//! bundle's declared node kinds and recording `source = Extension`. Also covers
//! the disable-removes-templates path.
//!
//! By importing the actual bundle file (not an inline fixture), this test is a
//! consistency check: if the bundle's template references a node kind whose id
//! drifts from what the extension contributes, this fails.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use starter_flow::registry::NodeKindRegistry;
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap};
use starter_setup::extension::{
    contributions_from_pairs, import_bundled_templates, remove_bundled_templates,
};
use starter_setup_spi::model::TemplateId;
use starter_setup_spi::store::{TemplateStore, GLOBAL_TENANT_SENTINEL};
use starter_store_sqlite::setup::{SqliteTemplateStore, SETUP_MIGRATION_SOURCE};
use starter_store_sqlite::{migrate, testing::ephemeral, Pool};

/// A no-op stand-in for a contributed device node kind (the real behaviour
/// lives in the extension's process binary; here we only need the kind to be
/// *registered* so body validation passes).
struct StubKind(KindId);

#[async_trait]
impl NodeBehavior for StubKind {
    fn kind_id(&self) -> &KindId {
        &self.0
    }
    fn trigger_slots(&self) -> &'static [&'static str] {
        &["in"]
    }
    async fn invoke(&self, _ctx: NodeCtx<'_>, _input: SlotMap) -> Result<SlotMap, NodeError> {
        Ok(SlotMap::new())
    }
}

fn bundle_dir() -> PathBuf {
    // crates/starter-setup/tests -> repo root -> nexus/extensions/com.acme.devices
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../nexus/extensions/com.acme.devices")
        .canonicalize()
        .expect("bundle dir resolves")
}

async fn boot() -> (Pool, Arc<NodeKindRegistry>) {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(SETUP_MIGRATION_SOURCE)
        .run()
        .await
        .unwrap();
    let kinds = Arc::new(NodeKindRegistry::new());
    for k in ["com.acme.devices.device_create", "com.acme.devices.sensor_register"] {
        kinds
            .register(Arc::new(StubKind(KindId::new(k).unwrap())))
            .await
            .unwrap();
    }
    (pool, kinds)
}

#[tokio::test]
async fn imports_and_removes_the_real_bundle_template() {
    let (pool, kinds) = boot().await;
    let store = SqliteTemplateStore::new(pool);

    let contributions =
        contributions_from_pairs([("com.acme.add-device", "templates/add-device.yaml")]);

    // Import (DOCS §9) — global catalog, source = Extension.
    let ids = import_bundled_templates(
        &bundle_dir(),
        "com.acme.devices",
        &contributions,
        &store,
        &kinds,
    )
    .await
    .expect("bundle import");
    assert_eq!(ids, vec![TemplateId::from("com.acme.add-device")]);

    // The template is in the global catalog with source = Extension.
    let id = TemplateId::from("com.acme.add-device");
    let t = store
        .get(Some(GLOBAL_TENANT_SENTINEL), &id, None)
        .await
        .unwrap()
        .expect("imported");
    assert_eq!(t.display_name, "Add a device");
    match &t.source {
        starter_setup_spi::model::TemplateSource::Extension { ext_id } => {
            assert_eq!(ext_id, "com.acme.devices")
        }
        other => panic!("expected Extension source, got {other:?}"),
    }
    // The flow references the contributed node kinds (validation passed).
    assert_eq!(t.flow_body.nodes.len(), 2);

    // Disable removes them (DOCS §9).
    remove_bundled_templates(&contributions, &store)
        .await
        .unwrap();
    assert!(store
        .get(Some(GLOBAL_TENANT_SENTINEL), &id, None)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn import_rejects_unknown_node_kind() {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(SETUP_MIGRATION_SOURCE)
        .run()
        .await
        .unwrap();
    let store = SqliteTemplateStore::new(pool);
    // Empty registry — the bundle's node kinds are NOT registered, so body
    // validation must reject the import.
    let kinds = Arc::new(NodeKindRegistry::new());
    let contributions =
        contributions_from_pairs([("com.acme.add-device", "templates/add-device.yaml")]);
    let err = import_bundled_templates(
        &bundle_dir(),
        "com.acme.devices",
        &contributions,
        &store,
        &kinds,
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, starter_setup_spi::error::SetupError::InvalidBody(_)),
        "expected InvalidBody, got {err:?}"
    );
}
