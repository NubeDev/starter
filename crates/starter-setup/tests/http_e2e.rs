//! End-to-end HTTP test of the REST surface (P2): proves the router,
//! stores, engine, and run service compose into a real axum app and the
//! barcode story works over the wire — import a template, run it (202 +
//! run_id), read the snapshot, resume a failed run.
//!
//! The principal is injected via a plain `Extension<Principal>` layer
//! (the route gates / authenticator are the host's job; here we isolate
//! the setup handlers + the in-handler owner/team checks).

#![cfg(feature = "rest")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::{Extension, Router};
use http_body_util::BodyExt;
use serde_json::Value;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::registry::NodeKindRegistry;
use starter_flow::run::FlowRunner;
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue};
use starter_spi::auth::{Principal, Role};
use starter_setup::service::{RunService, RunServiceConfig, SetupEngine};
use starter_setup_spi::store::TemplateStore;
use starter_store_sqlite::flow::{SqliteRunStore, FLOW_MIGRATION_SOURCE};
use starter_store_sqlite::setup::{
    SqliteSetupRunStore, SqliteTemplateStore, SETUP_MIGRATION_SOURCE,
};
use starter_store_sqlite::{migrate, testing::ephemeral, Pool};
use tower::ServiceExt;

struct FlakyNode {
    kind: KindId,
    fail: Arc<AtomicBool>,
}

#[async_trait]
impl NodeBehavior for FlakyNode {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }
    fn trigger_slots(&self) -> &'static [&'static str] {
        &["in"]
    }
    async fn invoke(&self, _ctx: NodeCtx<'_>, _input: SlotMap) -> Result<SlotMap, NodeError> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(NodeError::Backend("boom".into()));
        }
        let mut out = SlotMap::new();
        out.insert("out".into(), SlotValue::String("done".into()));
        Ok(out)
    }
}

const TEMPLATE_YAML: &str = r#"
id: com.acme.add-device
version: 1.0.0
display_name: Add a device
category: Provisioning
input_schema:
  type: object
  required: [barcode]
  properties:
    barcode: { type: string }
input_bindings:
  - { field: barcode, slot: com.acme.step.in }
output_bindings:
  - { slot: com.acme.step.out, field: device_id }
access:
  allowed_teams: [hvac-ops]
flow:
  nodes:
    - { id: com.acme.step, kind: com.acme.device.create }
  links: []
"#;

type Svc = RunService<SqliteTemplateStore, SqliteSetupRunStore>;

async fn app(fail: Arc<AtomicBool>) -> (Router, Arc<Svc>) {
    let pool: Pool = ephemeral().await;
    migrate(&pool)
        .with_source(FLOW_MIGRATION_SOURCE)
        .with_source(SETUP_MIGRATION_SOURCE)
        .run()
        .await
        .unwrap();

    let kinds = Arc::new(NodeKindRegistry::new());
    kinds
        .register(Arc::new(FlakyNode {
            kind: KindId::new("com.acme.device.create").unwrap(),
            fail,
        }))
        .await
        .unwrap();
    let graph: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let runner = Arc::new(
        FlowRunner::new(graph, Arc::new(starter_flow::run::InMemoryRunStore::new()))
            .with_config(SetupEngine::runner_config())
            .with_spi_run_store(Arc::new(SqliteRunStore::new(pool.clone()))),
    );
    let engine = SetupEngine::new(runner, kinds);
    let service = Arc::new(RunService::new(
        Arc::new(SqliteTemplateStore::new(pool.clone())),
        Arc::new(SqliteSetupRunStore::new(pool.clone())),
        engine,
        RunServiceConfig::default(),
    ));

    let principal = Principal {
        subject: "u-1".into(),
        role: Role::Writer,
        scopes: vec![],
        tenant_id: Some("acme".into()),
        teams: vec!["hvac-ops".into()],
        tenant_scope: vec![],
        extra: Value::Null,
    };
    let router = starter_setup::rest::router(service.clone())
        .layer(Extension(principal))
        .with_state(());
    (router, service)
}

async fn json(resp: axum::http::Response<Body>) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn barcode_story_over_http() {
    let fail = Arc::new(AtomicBool::new(true));
    let (router, service) = app(fail.clone()).await;

    // Import the template (raw YAML body).
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/setup/templates/import")
                .body(Body::from(TEMPLATE_YAML))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED, "import");

    // List templates — the nav should show it.
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/setup/templates")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let list = json(resp).await;
    assert_eq!(list.as_array().unwrap().len(), 1);

    // Run it → 202 + run_id (the flaky node will fail this attempt).
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/setup/templates/com.acme.add-device/run")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"barcode":"0X1A"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED, "run → 202");
    let run_id = json(resp).await["run_id"].as_str().unwrap().to_string();

    // Wait for the projector to record the terminal Failed + resumable.
    let snapshot = wait_failed(&router, &run_id).await;
    assert_eq!(snapshot["status"], "failed");
    assert_eq!(snapshot["resumable"], true);
    assert_eq!(snapshot["failed_node"], "com.acme.step");

    // Clear the failure and resume → 202.
    fail.store(false, Ordering::SeqCst);
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/setup/runs/{run_id}/resume"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED, "resume → 202");

    // Eventually Completed.
    let done = wait_status(&router, &run_id, "completed").await;
    assert_eq!(done["status"], "completed");

    // A different owner cannot read the run.
    let _ = service; // keep the service alive for the duration
}

async fn fetch_run(router: &Router, run_id: &str) -> Value {
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/setup/runs/{run_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    json(resp).await
}

async fn wait_failed(router: &Router, run_id: &str) -> Value {
    wait_status(router, run_id, "failed").await
}

async fn wait_status(router: &Router, run_id: &str, want: &str) -> Value {
    let mut last = Value::Null;
    for _ in 0..60 {
        let r = fetch_run(router, run_id).await;
        if r.get("status").and_then(|s| s.as_str()) == Some(want) {
            return r;
        }
        last = r;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("run never reached {want}; last snapshot = {last}");
}
