//! Admin invoke audit — `POST /api/v1/admin/registry/tools/{id}/invoke`
//! through the changelog middleware writes exactly one
//! `tool.invoke` row attributed to the authenticated admin.
//!
//! Sibling of [`changelog_middleware_test`]; that file pins the
//! public `/api/v1/tools/*` path, this one pins the admin path
//! goes through the *same* middleware via the multi-prefix
//! matcher. The two together prove no rubix-agent-owned tool
//! surface escapes the audit.

use std::collections::HashMap;
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Extensions, Request, StatusCode};
use axum::middleware::{from_fn, Next};
use axum::Router;
use serde_json::json;
use tower::ServiceExt;

use rubix_agent::admin::AdminState;
use rubix_agent::middleware::{changelog_layer, ChangelogState};
use rubix_agent::registry::build_tool_registry;
use rubix_agent::routes::admin::admin_invoke_registrar;
use starter_changelog::{ChangeFilter, ChangeLog};
use starter_changelog_sqlite::{
    migration_source as changelog_migration_source, SqliteChangeLog, SqliteChangeRecorder,
};
use starter_spi::auth::{Principal, Role};
use starter_spi::changelog::{Actor, ChangeRecorder, Op};
use starter_store_sqlite::{migrate, testing::ephemeral, Pool};

fn with_test_principal<S>(router: Router<S>, subject: &'static str) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(from_fn(
        move |mut req: Request<Body>, next: Next| async move {
            let exts: &mut Extensions = req.extensions_mut();
            exts.insert(Principal {
                subject: subject.to_owned(),
                role: Role::Admin,
                scopes: vec![],
                tenant_id: None,
                teams: vec![],
                tenant_scope: Vec::new(),
                extra: serde_json::Value::Null,
            });
            next.run(req).await
        },
    ))
}

async fn fresh_pool() -> Pool {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(changelog_migration_source())
        .run()
        .await
        .expect("apply changelog migration");
    pool
}

fn state() -> AdminState {
    let tools = build_tool_registry(90, None, None, None, None, None);
    let tool_map: HashMap<String, Arc<dyn starter_spi::tool::Tool>> = tools
        .iter()
        .map(|t| (t.definition().name, t.clone()))
        .collect();
    AdminState::empty().with_tools(Arc::new(tool_map))
}

fn audited_router(pool: Pool, subject: Option<&'static str>) -> Router {
    let recorder: Arc<dyn ChangeRecorder> = Arc::new(SqliteChangeRecorder::new(pool));
    let inner = admin_invoke_registrar(state()).into_router();
    let audited = changelog_layer(
        inner,
        ChangelogState {
            recorder,
            tool_path_prefixes: vec![
                "/api/v1/tools/".to_owned(),
                "/api/v1/admin/registry/tools/".to_owned(),
            ],
        },
    );
    match subject {
        Some(s) => with_test_principal(audited, s),
        None => audited,
    }
}

fn post_admin_invoke() -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/api/v1/admin/registry/tools/rubix.system.disk/invoke")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"tenant": "tenant-7", "input": {"mount": "/"}}).to_string(),
        ))
        .expect("request builds")
}

#[tokio::test]
async fn admin_invoke_writes_one_changelog_row_attributed_to_admin() {
    let pool = fresh_pool().await;
    let app = audited_router(pool.clone(), Some("admin@test"));

    let resp = app.oneshot(post_admin_invoke()).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let _ = to_bytes(resp.into_body(), 64 * 1024).await;

    let log = SqliteChangeLog::new(pool);
    let page = log
        .list(&ChangeFilter::default())
        .await
        .expect("list changes");
    assert_eq!(
        page.items.len(),
        1,
        "expected exactly one audit row, got {}",
        page.items.len(),
    );
    let row = &page.items[0];
    match &row.actor {
        Actor::User { subject } => assert_eq!(subject, "admin@test"),
        other => panic!("expected Actor::User, got {other:?}"),
    }
    assert_eq!(row.resource.kind, "tool.invoke");
    assert_eq!(row.resource.id.as_deref(), Some("rubix.system.disk"));
    match &row.op {
        Op::Custom(s) => assert_eq!(s, "invoke"),
        other => panic!("expected Op::Custom(\"invoke\"), got {other:?}"),
    }
    // The captured payload preserves the admin-provided tenant
    // so SIEM consumers can join audit rows to tenant-scoped
    // workloads without re-parsing the URL.
    let after = row.after.as_ref().expect("payload captured");
    assert_eq!(
        after.get("tenant").and_then(|v| v.as_str()),
        Some("tenant-7"),
        "expected tenant in audit payload; got {after}",
    );
}

#[tokio::test]
async fn anonymous_admin_invoke_writes_no_changelog_row() {
    let pool = fresh_pool().await;
    let app = audited_router(pool.clone(), None);

    let resp = app.oneshot(post_admin_invoke()).await.expect("oneshot");
    // Without a principal the handler still runs (the audit gate
    // is upstream); but no audit row should land.
    let _ = (resp.status(), to_bytes(resp.into_body(), 64 * 1024).await);

    let log = SqliteChangeLog::new(pool);
    let page = log
        .list(&ChangeFilter::default())
        .await
        .expect("list changes");
    assert!(
        page.items.is_empty(),
        "anonymous admin invoke must not produce audit rows, got {:?}",
        page.items,
    );
}
