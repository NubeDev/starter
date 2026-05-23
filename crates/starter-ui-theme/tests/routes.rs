//! Route-level integration tests: build the router around an
//! in-memory sqlite store, hit each handler with `tower::ServiceExt`,
//! assert status + body.

#![cfg(all(feature = "sqlite", feature = "routes"))]

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use starter_spi::auth::{Principal, Role, Scope};
use starter_spi::ui::theme::{ShellConfig, ThemeDocument, ThemeSaveInput, ThemeStyles};
use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral, Pool};
use starter_ui_theme::{routes::theme_router, routes::ThemeState, store::SqliteThemeStore};
use tower::ServiceExt;

static UI_THEME_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/ui_theme_sqlite");

async fn fresh_router() -> Router {
    let pool: Pool = ephemeral().await;
    migrate(&pool)
        .with_source(MigrationSource {
            name: "ui_theme",
            migrator: &UI_THEME_MIGRATOR,
        })
        .run()
        .await
        .unwrap();
    let store = Arc::new(SqliteThemeStore::new(pool));
    theme_router::<()>(ThemeState::new(store)).with_state(())
}

fn principal(role: Role) -> Principal {
    Principal {
        subject: "u1".into(),
        role,
        scopes: Vec::<Scope>::new(),
        tenant_id: None,
        teams: Vec::new(),
        extra: serde_json::Value::Null,
    }
}

fn req(method: &str, path: &str) -> http::request::Builder {
    Request::builder().method(method).uri(path)
}

fn req_with(method: &str, path: &str, p: Principal) -> http::request::Builder {
    req(method, path).extension(p)
}

async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
async fn get_requires_authentication() {
    let app = fresh_router().await;
    let resp = app
        .oneshot(req("GET", "/api/v1/ui/theme").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_returns_default_document_for_authenticated() {
    let app = fresh_router().await;
    let resp = app
        .oneshot(
            req_with("GET", "/api/v1/ui/theme", principal(Role::Reader))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let doc: ThemeDocument = body_json(resp).await;
    assert!(doc.theme_styles.light.is_empty());
}

#[tokio::test]
async fn put_requires_admin() {
    let app = fresh_router().await;
    let body = serde_json::to_vec(&ThemeSaveInput::default()).unwrap();
    let resp = app
        .clone()
        .oneshot(
            req_with("PUT", "/api/v1/ui/theme", principal(Role::Writer))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // No principal → 401.
    let resp = app
        .oneshot(
            req("PUT", "/api/v1/ui/theme")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn put_round_trips_document() {
    let app = fresh_router().await;
    let input = ThemeSaveInput {
        theme_styles: ThemeStyles {
            light: [("primary".into(), "oklch(0.55 0.22 257)".into())]
                .into_iter()
                .collect(),
            ..Default::default()
        },
        shell: ShellConfig {
            nav_title: "My App".into(),
            hide_features: vec![],
        },
    };
    let resp = app
        .clone()
        .oneshot(
            req_with("PUT", "/api/v1/ui/theme", principal(Role::Admin))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&input).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let saved: ThemeDocument = body_json(resp).await;
    assert_eq!(saved.shell.nav_title, "My App");

    // A follow-up GET sees it.
    let resp = app
        .oneshot(
            req_with("GET", "/api/v1/ui/theme", principal(Role::Reader))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let doc: ThemeDocument = body_json(resp).await;
    assert_eq!(
        doc.theme_styles.light.get("primary").map(String::as_str),
        Some("oklch(0.55 0.22 257)"),
    );
}

#[tokio::test]
async fn put_rejects_unsafe_token_value() {
    let app = fresh_router().await;
    let input = ThemeSaveInput {
        theme_styles: ThemeStyles {
            light: [("primary".into(), "url(https://evil.example)".into())]
                .into_iter()
                .collect(),
            ..Default::default()
        },
        shell: ShellConfig::default(),
    };
    let resp = app
        .oneshot(
            req_with("PUT", "/api/v1/ui/theme", principal(Role::Admin))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&input).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let problem: starter_spi::dto::Problem = body_json(resp).await;
    assert_eq!(problem.kind, "invalid_input");
    let detail = problem.detail.unwrap_or_default();
    assert!(detail.contains("light.primary"));
    assert!(detail.contains("url("));
}

#[tokio::test]
async fn logo_post_get_delete_flow() {
    let app = fresh_router().await;

    // 415 on unsupported content-type.
    let resp = app
        .clone()
        .oneshot(
            req_with("POST", "/api/v1/ui/theme/logo", principal(Role::Admin))
                .header(CONTENT_TYPE, "application/octet-stream")
                .body(Body::from(b"x".to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

    // 204 on a valid upload.
    let resp = app
        .clone()
        .oneshot(
            req_with("POST", "/api/v1/ui/theme/logo", principal(Role::Admin))
                .header(CONTENT_TYPE, "image/png")
                .body(Body::from(b"PNGDATA".to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // GET serves bytes — public, no principal required.
    let resp = app
        .clone()
        .oneshot(
            req("GET", "/api/v1/ui/theme/logo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(CONTENT_TYPE).unwrap().to_str().unwrap(),
        "image/png",
    );
    let bytes = to_bytes(resp.into_body(), 4096).await.unwrap();
    assert_eq!(bytes.as_ref(), b"PNGDATA");

    // DELETE clears.
    let resp = app
        .clone()
        .oneshot(
            req_with("DELETE", "/api/v1/ui/theme/logo", principal(Role::Admin))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // GET is 404 after delete.
    let resp = app
        .oneshot(
            req("GET", "/api/v1/ui/theme/logo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn logo_oversize_rejected_with_413() {
    let app = fresh_router().await;
    let big = vec![0u8; starter_ui_theme::limits::LOGO_MAX_BYTES + 1];
    let resp = app
        .oneshot(
            req_with("POST", "/api/v1/ui/theme/logo", principal(Role::Admin))
                .header(CONTENT_TYPE, "image/png")
                .body(Body::from(big))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn favicon_admin_only() {
    let app = fresh_router().await;
    let resp = app
        .oneshot(
            req_with("POST", "/api/v1/ui/theme/favicon", principal(Role::Reader))
                .header(CONTENT_TYPE, "image/png")
                .body(Body::from(b"x".to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
