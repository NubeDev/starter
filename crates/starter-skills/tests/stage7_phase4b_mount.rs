//! Phase 4b stage 7 — on-mount hash verification + select-has-no-IO.
//!
//! Two normative smokes from the stage brief land here. The
//! resource-hash-mismatch test for the actual `ai-agent` body lives
//! alongside the body itself in
//! `crates/starter-flow-nodes/tests/stage7_ai_agent_mount.rs`; what
//! lives here is the registry-side proof that drives the same
//! invariants without coupling to the node-kind crate.
//!
//! 1. `select_does_no_io_on_a_built_registry` — a registry built
//!    against an [`ApprovalStore`] that **panics on `lookup`** (after
//!    a flag is flipped) still serves `select(...)` without touching
//!    the store. The build phase exercises `lookup` legitimately; we
//!    flip the panic switch *after* `build()` returns and prove no
//!    subsequent `select()` call reaches the store (R-skills-8:
//!    approvals are cached at build time).
//!
//! 2. `mount_verification_round_trips_then_aborts_on_drift` — a
//!    registry-produced [`SkillSelection::Selected`] carries
//!    absolute `file://` URIs + per-file `content_hash` values that
//!    [`starter_skills::read_and_verify`] round-trips against the
//!    bytes on disk; editing a resource between selection and mount
//!    surfaces [`ResourceMountError::HashMismatch`]; a subsequent
//!    `reload()` produces a fresh selection whose hash matches the
//!    edited bytes, so the next mount succeeds. The `ai-agent`-body
//!    half of the same smoke (the typed `NodeError::Domain` arm) is
//!    `resource_hash_mismatch_aborts_the_run` in
//!    `crates/starter-flow-nodes/tests/stage7_ai_agent_mount.rs`.
//!
//! [`SkillSelection::Selected`]: starter_flow_spi::skill::SkillSelection::Selected

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;

use starter_flow_spi::node::SlotMap;
use starter_flow_spi::skill::{SkillId, SkillSelection, SkillSelector};
use starter_flow_spi::Principal;
use starter_skills::{
    read_and_verify, ApprovalRow, ApprovalStore, ApprovalStoreError, FirstSkillSelector,
    InMemoryApprovalStore, ResourceMountError, SkillRegistry,
};

// ---------------------------------------------------------------------
// Local TempDir helper (mirrors the stage6 test's helper; no `tempfile`
// dep is on the dev path for `starter-skills`).
// ---------------------------------------------------------------------

struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let base = std::env::temp_dir().join(format!(
            "starter-skills-stage7-{tag}-{}-{nanos}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        Self(base)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write_file(dir: &Path, rel: &str, body: &str) {
    let p = dir.join(rel);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(p, body).unwrap();
}

fn principal() -> Principal {
    Principal {
        subject: "operator-alice".into(),
        role: starter_spi::auth::Role::Admin,
        scopes: Vec::new(),
        tenant_id: None,
        teams: Vec::new(),
        extra: serde_json::Value::Null,
    }
}

fn approved_skill_md_with_resource(id: &str, desc: &str, rel: &str) -> String {
    format!(
        "---\nid: {id}\ndescription: {desc}\ntrust: approved\nresources:\n  - file://{rel}\n---\nbody for {id}\n"
    )
}

// ---------------------------------------------------------------------
// PanickingApprovalStore — wraps `InMemoryApprovalStore` with a
// post-build trip-wire. The build phase legitimately calls `lookup`
// once per bundle, so we delegate until the test flips `armed = true`.
// Any subsequent `lookup` panics; the test then asserts that
// `SkillRegistry::select(...)` returns normally, proving R-skills-8.
// ---------------------------------------------------------------------

struct PanickingApprovalStore {
    inner: InMemoryApprovalStore,
    armed: AtomicBool,
    // The store also fails record/revoke when armed, so the test
    // catches any *write* path that select() may accidentally take
    // (it must not — select is purely an in-memory lookup against
    // the pre-computed `approved` map).
}

impl PanickingApprovalStore {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: InMemoryApprovalStore::new(),
            armed: AtomicBool::new(false),
        })
    }

    fn arm(&self) {
        self.armed.store(true, Ordering::SeqCst);
    }
}

#[async_trait]
impl ApprovalStore for PanickingApprovalStore {
    async fn record(&self, row: ApprovalRow) -> Result<(), ApprovalStoreError> {
        if self.armed.load(Ordering::SeqCst) {
            panic!("ApprovalStore::record must not be called during select()");
        }
        self.inner.record(row).await
    }
    async fn lookup(
        &self,
        skill_id: &SkillId,
        bundle_hash: &str,
    ) -> Result<Option<ApprovalRow>, ApprovalStoreError> {
        if self.armed.load(Ordering::SeqCst) {
            panic!(
                "ApprovalStore::lookup must not be called during select() \
                 (skill_id={skill_id}, bundle_hash={bundle_hash})"
            );
        }
        self.inner.lookup(skill_id, bundle_hash).await
    }
    async fn list(&self) -> Result<Vec<ApprovalRow>, ApprovalStoreError> {
        if self.armed.load(Ordering::SeqCst) {
            panic!("ApprovalStore::list must not be called during select()");
        }
        self.inner.list().await
    }
    async fn revoke(
        &self,
        skill_id: &SkillId,
        bundle_hash: &str,
    ) -> Result<(), ApprovalStoreError> {
        if self.armed.load(Ordering::SeqCst) {
            panic!("ApprovalStore::revoke must not be called during select()");
        }
        self.inner.revoke(skill_id, bundle_hash).await
    }
}

// ---------------------------------------------------------------------
// Smoke 1 — select() touches no store after build.
// ---------------------------------------------------------------------

#[tokio::test]
async fn select_does_no_io_on_a_built_registry() {
    let tmp = TempDir::new("no-io");
    // One approved bundle is enough — we just need `select(...)` to
    // have a candidate to return.
    let bundle = tmp.path().join("greet");
    std::fs::create_dir_all(&bundle).unwrap();
    write_file(
        &bundle,
        "SKILL.md",
        "---\nid: starter.noio.greet\ndescription: Greets.\ntrust: approved\n---\nhi\n",
    );

    let store = PanickingApprovalStore::new();
    let registry = SkillRegistry::builder()
        .with_approval_store_arc(Arc::clone(&store) as Arc<dyn ApprovalStore>)
        .with_default_selector(FirstSkillSelector::new())
        .load_dir(tmp.path())
        .build()
        .await
        .expect("build ok (store is unarmed)");

    // Trip-wire on. Any further store call panics; the test then
    // asserts `select(...)` returns Ok without ever calling lookup.
    store.arm();

    let sel = (&registry as &dyn SkillSelector)
        .select(&SlotMap::new(), &principal())
        .await
        .expect("select must not touch the store (R-skills-8)");

    match sel {
        SkillSelection::Selected { skill_id, .. } => {
            assert_eq!(skill_id.as_str(), "starter.noio.greet");
        }
        other => panic!("expected Selected, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// Smoke 2 — resource hash mismatch surfaces ResourceMountError at the
// `read_and_verify` seam, and a reload() resolves to the new hash.
// ---------------------------------------------------------------------

#[tokio::test]
async fn mount_verification_round_trips_then_aborts_on_drift() {
    let tmp = TempDir::new("mount");
    let bundle = tmp.path().join("with-resource");
    std::fs::create_dir_all(&bundle).unwrap();
    write_file(
        &bundle,
        "SKILL.md",
        &approved_skill_md_with_resource(
            "starter.mount.greet",
            "Greets via a mounted resource.",
            "greeting.md",
        ),
    );
    write_file(&bundle, "greeting.md", "Hello, H1!\n");

    let registry = SkillRegistry::builder()
        .with_approval_store(InMemoryApprovalStore::new())
        .with_default_selector(FirstSkillSelector::new())
        .load_dir(tmp.path())
        .build()
        .await
        .expect("build ok");

    // Selection at H1 — the bytes on disk match the frozen
    // `ResourceRef.content_hash`, so `read_and_verify` succeeds.
    let sel_h1 = (&registry as &dyn SkillSelector)
        .select(&SlotMap::new(), &principal())
        .await
        .expect("select ok");
    let resources_h1 = match &sel_h1 {
        SkillSelection::Selected { resources, .. } => resources.clone(),
        other => panic!("expected Selected, got {other:?}"),
    };
    assert_eq!(resources_h1.len(), 1, "one mounted resource");
    let bytes = read_and_verify(&resources_h1[0]).expect("H1 round-trips");
    assert_eq!(bytes, b"Hello, H1!\n");

    // Drift: edit the resource bytes between selection and mount. A
    // racing `SkillRegistry::reload()` could do the same thing; the
    // load-bearing claim is that the *frozen* selection's hash no
    // longer matches what is on disk, so the mount aborts.
    write_file(&bundle, "greeting.md", "Hello, H2 (drifted)!\n");
    let err =
        read_and_verify(&resources_h1[0]).expect_err("expected HashMismatch after on-disk edit");
    match err {
        ResourceMountError::HashMismatch {
            expected,
            actual,
            uri,
        } => {
            assert_eq!(expected, resources_h1[0].content_hash);
            assert_ne!(actual, expected, "actual differs from frozen");
            assert!(
                uri.starts_with("file://"),
                "URI is the absolute file:// form the registry rewrote"
            );
        }
        other => panic!("expected HashMismatch, got {other:?}"),
    }

    // Subsequent runs against the edited bundle see the new
    // `content_hash` after `reload()` and proceed normally.
    registry.reload().await.expect("reload ok");
    let sel_h2 = (&registry as &dyn SkillSelector)
        .select(&SlotMap::new(), &principal())
        .await
        .expect("select ok");
    let resources_h2 = match &sel_h2 {
        SkillSelection::Selected { resources, .. } => resources.clone(),
        other => panic!("expected Selected, got {other:?}"),
    };
    assert_ne!(
        resources_h1[0].content_hash, resources_h2[0].content_hash,
        "fresh selection captures H2"
    );
    let bytes_h2 = read_and_verify(&resources_h2[0]).expect("H2 round-trips");
    assert_eq!(bytes_h2, b"Hello, H2 (drifted)!\n");
}
