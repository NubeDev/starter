//! HR-5 integration tests for `starter-flow-watch`.
//!
//! These tests construct a real [`DefinitionManager`] backed by an
//! in-memory `FlowStore` and exercise the file → publish path
//! through [`apply_file_event`] / [`boot_walk`].

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tempfile::TempDir;

use starter_flow::definition::DefinitionManager;
use starter_flow::registry::NodeKindRegistry;
use starter_flow_spi::definition::FlowDefinitionEvent;
use starter_flow_spi::flow::{
    FlowError, FlowId, FlowResult, FlowRevision, FlowRevisionId, FlowStore,
};
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap};
use starter_flow_watch::{apply_file_event, boot_walk, parse_flow_file, FileEvent};

/// In-memory `FlowStore` mirror; mirrors the shape used in
/// `starter-flow`'s own definition tests.
#[derive(Default)]
struct MemStore {
    inner: Mutex<HashMap<FlowId, Vec<FlowRevision>>>,
}

impl MemStore {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

#[async_trait]
impl FlowStore for MemStore {
    async fn load(
        &self,
        flow_id: FlowId,
        revision: Option<FlowRevisionId>,
    ) -> FlowResult<FlowRevision> {
        let g = self.inner.lock().unwrap();
        let revs = g
            .get(&flow_id)
            .ok_or_else(|| FlowError::NotFound {
                kind: "flow",
                id: flow_id.to_string(),
            })?;
        let rev = match revision {
            None => revs.last().cloned(),
            Some(id) => revs.iter().find(|r| r.revision_id == id).cloned(),
        };
        rev.ok_or_else(|| FlowError::NotFound {
            kind: "flow",
            id: flow_id.to_string(),
        })
    }

    async fn put(&self, revision: FlowRevision) -> FlowResult<FlowRevisionId> {
        let id = revision.revision_id;
        let flow = revision.flow_id.clone();
        let mut g = self.inner.lock().unwrap();
        g.entry(flow).or_default().push(revision);
        Ok(id)
    }

    async fn list(&self) -> FlowResult<Vec<FlowId>> {
        Ok(self.inner.lock().unwrap().keys().cloned().collect())
    }

    async fn revisions(&self, flow_id: FlowId) -> FlowResult<Vec<FlowRevisionId>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .get(&flow_id)
            .map(|v| v.iter().rev().map(|r| r.revision_id).collect())
            .unwrap_or_default())
    }

    async fn head(&self, flow_id: FlowId) -> FlowResult<Option<FlowRevisionId>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .get(&flow_id)
            .and_then(|v| v.last().map(|r| r.revision_id)))
    }
}

struct AnyKind {
    kind: KindId,
}

#[async_trait]
impl NodeBehavior for AnyKind {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }
    async fn invoke(&self, _ctx: NodeCtx<'_>, _input: SlotMap) -> Result<SlotMap, NodeError> {
        Ok(SlotMap::new())
    }
}

async fn build_manager() -> Arc<DefinitionManager> {
    let store = MemStore::new();
    let kinds = Arc::new(NodeKindRegistry::new());
    kinds
        .register(Arc::new(AnyKind {
            kind: KindId::new("com.example.any").unwrap(),
        }))
        .await
        .unwrap();
    Arc::new(DefinitionManager::new(store, kinds))
}

fn write(path: &Path, contents: &str) {
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(contents.as_bytes()).unwrap();
}

/// HR-5: `boot_walk` publishes every flow file in the directory and
/// returns the `(path, flow_id)` pairs so the watch loop can resolve
/// later `Remove` events.
#[tokio::test]
async fn hr5_boot_walk_publishes_every_flow_file() {
    let dir = TempDir::new().unwrap();
    write(
        &dir.path().join("a.json"),
        r#"{"flow_id":"examples.watch.a","nodes":[{"id":"watch.n","kind":"com.example.any"}],"links":[]}"#,
    );
    write(
        &dir.path().join("b.json"),
        r#"{"flow_id":"examples.watch.b","nodes":[{"id":"watch.n","kind":"com.example.any"}],"links":[]}"#,
    );
    // Non-flow file is ignored.
    write(&dir.path().join("README.md"), "ignore me");

    let mgr = build_manager().await;
    let walked = boot_walk(&mgr, dir.path()).await;
    assert_eq!(walked.len(), 2);

    let listed = mgr.store().list().await.unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(mgr.active_topologies().len().await, 2);
}

/// HR-5: an `Upsert` event for an unchanged file short-circuits via
/// HR1; no second revision is written.
#[tokio::test]
async fn hr5_upsert_unchanged_file_short_circuits() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("a.json");
    write(
        &path,
        r#"{"flow_id":"examples.watch.a","nodes":[{"id":"watch.n","kind":"com.example.any"}],"links":[]}"#,
    );
    let mgr = build_manager().await;

    apply_file_event(&mgr, FileEvent::Upsert(path.clone()), |_| None)
        .await
        .unwrap();
    let after_first = mgr
        .store()
        .revisions(FlowId::new("examples.watch.a").unwrap())
        .await
        .unwrap()
        .len();
    assert_eq!(after_first, 1);

    apply_file_event(&mgr, FileEvent::Upsert(path), |_| None)
        .await
        .unwrap();
    let after_second = mgr
        .store()
        .revisions(FlowId::new("examples.watch.a").unwrap())
        .await
        .unwrap()
        .len();
    assert_eq!(after_second, 1, "idempotent re-upsert must short-circuit");
}

/// HR-5 / HR-7: a `Remove` event calls `publish_delete`, which
/// emits `Removed` on the bus and unmounts the active topology.
#[tokio::test]
async fn hr5_remove_event_publishes_delete() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("a.json");
    write(
        &path,
        r#"{"flow_id":"examples.watch.a","nodes":[{"id":"watch.n","kind":"com.example.any"}],"links":[]}"#,
    );

    let mgr = build_manager().await;
    let walked = boot_walk(&mgr, dir.path()).await;
    let mapping: HashMap<_, _> = walked.into_iter().collect();
    assert_eq!(mgr.active_topologies().len().await, 1);

    let mut rx = mgr.subscribe();

    apply_file_event(
        &mgr,
        FileEvent::Remove(path.clone()),
        move |p| mapping.get(p).cloned(),
    )
    .await
    .unwrap();

    assert_eq!(mgr.active_topologies().len().await, 0);

    // Drain events looking for Removed.
    let mut saw_removed = false;
    while let Ok(Ok(ev)) =
        tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await
    {
        if matches!(ev, FlowDefinitionEvent::Removed { .. }) {
            saw_removed = true;
            break;
        }
    }
    assert!(saw_removed, "Removed event must be emitted");
}

/// HR-5: `parse_flow_file` rejects bodies that don't carry a
/// `flow_id` field — the publish chokepoint never sees them.
#[tokio::test]
async fn hr5_malformed_file_does_not_publish() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("a.json");
    write(&path, r#"{"nodes":[],"links":[]}"#);

    let mgr = build_manager().await;
    let err = apply_file_event(&mgr, FileEvent::Upsert(path), |_| None)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        starter_flow_watch::WatchError::MissingFlowId { .. }
    ));
    assert_eq!(mgr.store().list().await.unwrap().len(), 0);
}

/// HR-5: `parse_flow_file` exposes the canonical body the publish
/// chokepoint sees. Sanity-check that the watch crate does not
/// rewrite the body in transit.
#[test]
fn parse_flow_file_preserves_body() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("a.json");
    let body = r#"{"flow_id":"examples.watch.a","nodes":[],"links":[]}"#;
    write(&path, body);
    let parsed = parse_flow_file(&path).unwrap();
    let back: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(parsed.body, back);
}
