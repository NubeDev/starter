//! P1 + P1a acceptance (DOCS §7–§9, §8a/§8b):
//!
//! - run_template launches instantly, the projector tracks progress, and
//!   trusted identity slots are seeded from the verified Principal.
//! - a fatal node failure halts the run (§8b policy), the index row goes
//!   Failed + resumable with a cursor, and resume_run replays + re-enters
//!   at the cursor to completion.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::registry::NodeKindRegistry;
use starter_flow::run::FlowRunner;
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue,
};
use starter_flow_spi::Principal;
use starter_setup::service::{RunService, RunServiceConfig, SetupEngine};
use starter_setup_spi::envelope::TemplateEnvelope;
use starter_setup_spi::model::{SetupRunStatus, TemplateSource};
use starter_setup_spi::store::SetupRunStore;
use starter_spi::auth::Role;
use starter_store_sqlite::flow::{SqliteRunStore, FLOW_MIGRATION_SOURCE};
use starter_store_sqlite::setup::{
    SqliteSetupRunStore, SqliteTemplateStore, SETUP_MIGRATION_SOURCE,
};
use starter_store_sqlite::{migrate, testing::ephemeral, Pool};

/// A flaky, idempotent side-effect node: reads `in`, writes `out`. While
/// `should_fail` is set it returns a Backend error (the §8b fatal path);
/// once cleared it succeeds. It also records the trusted `caller_user_id`
/// slot it saw into a shared cell so the test can assert identity seeding.
struct FlakyNode {
    kind: KindId,
    should_fail: Arc<AtomicBool>,
    saw_user: Arc<std::sync::Mutex<Option<String>>>,
}

#[async_trait]
impl NodeBehavior for FlakyNode {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }
    fn trigger_slots(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn read_slots(&self) -> &'static [&'static str] {
        &["caller_user_id", "caller_team_ids", "caller_tenant_id"]
    }
    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        if let Some(SlotValue::String(u)) = input.get("caller_user_id") {
            *self.saw_user.lock().unwrap() = Some(u.clone());
        }
        if self.should_fail.load(Ordering::SeqCst) {
            return Err(NodeError::Backend("simulated gateway timeout".into()));
        }
        let mut out = SlotMap::new();
        out.insert("out".into(), SlotValue::String("ok".into()));
        Ok(out)
    }
}

const TEMPLATE: &str = r#"
id: com.test.flaky-flow
version: 1.0.0
display_name: Flaky Flow
category: Test
input_schema:
  type: object
  required: [barcode]
  properties:
    barcode: { type: string }
input_bindings:
  - { field: barcode, slot: com.test.step.in }
output_bindings:
  - { slot: com.test.step.out, field: result }
access:
  allowed_teams: []
flow:
  nodes:
    - { id: com.test.step, kind: com.test.flaky }
  links: []
"#;

async fn boot_pool() -> Pool {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(FLOW_MIGRATION_SOURCE)
        .with_source(SETUP_MIGRATION_SOURCE)
        .run()
        .await
        .expect("migrations apply");
    pool
}

fn principal() -> Principal {
    Principal {
        subject: "u-42".into(),
        role: Role::Writer,
        scopes: vec![],
        tenant_id: Some("acme".into()),
        teams: vec!["hvac-ops".into()],
        tenant_scope: vec![],
        extra: serde_json::Value::Null,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_fails_then_resumes_from_cursor() {
    let pool = boot_pool().await;

    // Registry with our flaky kind.
    let should_fail = Arc::new(AtomicBool::new(true));
    let saw_user = Arc::new(std::sync::Mutex::new(None));
    let kinds = Arc::new(NodeKindRegistry::new());
    kinds
        .register(Arc::new(FlakyNode {
            kind: KindId::new("com.test.flaky").unwrap(),
            should_fail: should_fail.clone(),
            saw_user: saw_user.clone(),
        }))
        .await
        .unwrap();

    // Engine: FlowRunner with the §8b halt policy + a SPI run store for
    // checkpoints/resume.
    let graph: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let spi_runs = Arc::new(SqliteRunStore::new(pool.clone()));
    let in_mem = Arc::new(starter_flow::run::InMemoryRunStore::new());
    let runner = Arc::new(
        FlowRunner::new(graph, in_mem)
            .with_config(SetupEngine::runner_config())
            .with_spi_run_store(spi_runs),
    );
    let engine = SetupEngine::new(runner, kinds.clone());

    // Stores + service.
    let templates = Arc::new(SqliteTemplateStore::new(pool.clone()));
    let runs = Arc::new(SqliteSetupRunStore::new(pool.clone()));
    let service = RunService::new(
        templates.clone(),
        runs.clone(),
        engine,
        RunServiceConfig::default(),
    );

    // Import + persist the template.
    let template = TemplateEnvelope::from_yaml(TEMPLATE)
        .unwrap()
        .into_template(Some("acme".into()), TemplateSource::Api)
        .unwrap();
    use starter_setup_spi::store::TemplateStore;
    templates.put(template.clone()).await.unwrap();

    // ---- Launch; the flaky node fails → run halts + becomes resumable.
    let input = serde_json::json!({ "barcode": "0X1A" });
    let handle = service
        .run_template(&template, &principal(), &input)
        .await
        .expect("launch");
    let run_id = handle.run;
    let status = tokio::time::timeout(std::time::Duration::from_secs(5), handle.join)
        .await
        .expect("run timed out")
        .expect("join");
    assert_eq!(format!("{status:?}"), "Failed(\"node com.test.step failed: backend failure: simulated gateway timeout\")");

    // Trusted identity was seeded from the verified principal (NOT form).
    assert_eq!(saw_user.lock().unwrap().as_deref(), Some("u-42"));

    // Give the projector a beat to persist the terminal state.
    let setup_run = wait_for_status(&runs, run_id, SetupRunStatus::Failed).await;
    assert!(setup_run.resumable);
    assert_eq!(setup_run.failed_node.as_deref(), Some("com.test.step"));
    // It is in the open set (resumable).
    assert!(runs.list_open().await.unwrap().contains(&run_id));

    // ---- Clear the failure and resume from the cursor → completes.
    should_fail.store(false, Ordering::SeqCst);
    let handle2 = service
        .resume_run(&template, run_id)
        .await
        .expect("resume");
    let status2 = tokio::time::timeout(std::time::Duration::from_secs(5), handle2.join)
        .await
        .expect("resume timed out")
        .expect("join2");
    assert_eq!(format!("{status2:?}"), "Completed");

    let done = wait_for_status(&runs, run_id, SetupRunStatus::Completed).await;
    assert_eq!(done.status, SetupRunStatus::Completed);
    assert!(done.finished_at.is_some());
    // No longer open.
    assert!(!runs.list_open().await.unwrap().contains(&run_id));
}

#[tokio::test]
async fn rejects_input_violating_schema() {
    let pool = boot_pool().await;
    let kinds = Arc::new(NodeKindRegistry::new());
    kinds
        .register(Arc::new(FlakyNode {
            kind: KindId::new("com.test.flaky").unwrap(),
            should_fail: Arc::new(AtomicBool::new(false)),
            saw_user: Arc::new(std::sync::Mutex::new(None)),
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
    let templates = Arc::new(SqliteTemplateStore::new(pool.clone()));
    let runs = Arc::new(SqliteSetupRunStore::new(pool.clone()));
    let service = RunService::new(templates, runs, engine, RunServiceConfig::default());

    let template = TemplateEnvelope::from_yaml(TEMPLATE)
        .unwrap()
        .into_template(Some("acme".into()), TemplateSource::Api)
        .unwrap();

    // Missing required `barcode`.
    let bad = serde_json::json!({});
    match service.run_template(&template, &principal(), &bad).await {
        Err(starter_setup_spi::error::SetupError::InvalidInput(_)) => {}
        Ok(_) => panic!("expected InvalidInput, got Ok"),
        Err(other) => panic!("expected InvalidInput, got {other:?}"),
    }
}

async fn wait_for_status(
    runs: &SqliteSetupRunStore,
    run_id: starter_flow_spi::flow::RunId,
    want: SetupRunStatus,
) -> starter_setup_spi::model::SetupRun {
    for _ in 0..50 {
        if let Some(r) = runs.get(run_id).await.unwrap() {
            if r.status == want {
                return r;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("run {run_id} never reached {want:?}");
}
