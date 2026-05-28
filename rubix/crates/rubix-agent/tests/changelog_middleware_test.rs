//! Integration coverage for the changelog middleware on the tools
//! router. Pins three guarantees:
//!
//!   1. One authenticated tool call → exactly one `starter_changes`
//!      row, actor = the principal's subject, resource.kind =
//!      `tool.invoke`, resource.id = the tool id from the path.
//!   2. Anonymous requests are not audited (no principal → no row).
//!   3. The redactor drops obviously-secret-looking keys from the
//!      captured payload.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Extensions, Request, StatusCode};
use axum::middleware::{from_fn, Next};
use axum::Router;
use tower::ServiceExt;

use rubix_agent::middleware::{changelog_layer, ChangelogState};
use rubix_agent::registry::build_tool_registry;
use rubix_agent::routes::tools::{router as tools_router, ToolsState};
use starter_changelog::{ChangeFilter, ChangeLog};
use starter_changelog_sqlite::{
    migration_source as changelog_migration_source, SqliteChangeLog, SqliteChangeRecorder,
};
use starter_spi::auth::{Principal, Role};
use starter_spi::changelog::{Actor, ChangeRecorder, Op};
use starter_store_sqlite::{migrate, testing::ephemeral, Pool};

/// Stamp a `Principal` on every request — stands in for a real
/// `with_principal(authenticator)` layer (which would need a live
/// Postgres). The middleware under test only reads the extension.
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

fn audited_router(pool: Pool, subject: Option<&'static str>) -> Router {
    let bundle = Arc::new(rubix_spi::i18n::rubix_bundle().expect("rubix bundle parses"));
    let tools = build_tool_registry(90, None, None, None, None, None);
    let inner = tools_router(ToolsState::new(tools, bundle));

    let recorder: Arc<dyn ChangeRecorder> = Arc::new(SqliteChangeRecorder::new(pool));
    let audited = changelog_layer(
        inner,
        ChangelogState {
            recorder,
            tool_path_prefixes: vec!["/api/v1/tools/".to_owned()],
        },
    );
    match subject {
        Some(s) => with_test_principal(audited, s),
        None => audited,
    }
}

fn post_disk() -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/api/v1/tools/rubix.system.disk")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"mount":"/"}"#))
        .expect("request builds")
}

#[tokio::test]
async fn authenticated_tool_call_writes_exactly_one_changelog_row() {
    let pool = fresh_pool().await;
    let app = audited_router(pool.clone(), Some("operator-123"));

    let resp = app.oneshot(post_disk()).await.expect("oneshot");
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
        Actor::User { subject } => assert_eq!(subject, "operator-123"),
        other => panic!("expected Actor::User, got {other:?}"),
    }
    assert_eq!(row.resource.kind, "tool.invoke");
    assert_eq!(row.resource.id.as_deref(), Some("rubix.system.disk"));
    match &row.op {
        Op::Custom(s) => assert_eq!(s, "invoke"),
        other => panic!("expected Op::Custom(\"invoke\"), got {other:?}"),
    }
}

#[tokio::test]
async fn anonymous_tool_call_writes_no_changelog_row() {
    let pool = fresh_pool().await;
    let app = audited_router(pool.clone(), None);

    let resp = app.oneshot(post_disk()).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let _ = to_bytes(resp.into_body(), 64 * 1024).await;

    let log = SqliteChangeLog::new(pool);
    let page = log
        .list(&ChangeFilter::default())
        .await
        .expect("list changes");
    assert!(
        page.items.is_empty(),
        "anonymous requests must not produce audit rows, got {:?}",
        page.items,
    );
}

#[tokio::test]
async fn audit_row_redacts_secret_keys_from_payload() {
    let pool = fresh_pool().await;
    let app = audited_router(pool.clone(), Some("operator-redact"));

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/tools/rubix.system.disk")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"mount":"/","password":"hunter2"}"#))
        .expect("request builds");
    let resp = app.oneshot(req).await.expect("oneshot");
    let _ = to_bytes(resp.into_body(), 64 * 1024).await;

    let log = SqliteChangeLog::new(pool);
    let page = log
        .list(&ChangeFilter::default())
        .await
        .expect("list changes");
    let row = page.items.first().expect("one audit row");
    let after = row.after.as_ref().expect("payload captured");
    assert!(
        after.get("mount").is_some(),
        "payload retains non-secret keys: {after}"
    );
    assert!(
        after.get("password").is_none(),
        "redactor must drop `password`: {after}",
    );
}
