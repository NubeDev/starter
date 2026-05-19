//! Integration tests for [`starter_prefs::routes`].
//!
//! Spin up the router against an in-memory sqlite store, stub the
//! `Principal` via a request extension, and assert against the four
//! contracts the stage 6 plan pins:
//!
//! 1. **GET-after-PATCH reflects the change** — write a value via
//!    PATCH, GET sees it.
//! 2. **PATCH null reverts to org/default** — write a user-layer
//!    override, then PATCH the same field to JSON `null`; GET falls
//!    back through org → default per R3.
//! 3. **GET /v1/units returns the same JSON + ETag on every call** —
//!    the registry is compile-time static.
//! 4. **Admin-only paths return 403 for a non-admin Principal** and
//!    401 when no `Principal` extension is attached.

#![cfg(all(feature = "routes", feature = "sqlite"))]

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use starter_prefs::resolver::SystemDefaults;
use starter_prefs::routes::{prefs_router, PrefsRoutesState};
use starter_prefs::store::SqlitePrefsStore;
use starter_spi::auth::{Principal, Role, Scope};
use starter_spi::preferences::ResolvedPreferences;
use tower::ServiceExt;

// ---------------------------------------------------------------------
// Fixtures.
// ---------------------------------------------------------------------

async fn fresh_app() -> Router {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let store = SqlitePrefsStore::new(pool);
    store.migrate().await.unwrap();
    let state = PrefsRoutesState::new(Arc::new(store), SystemDefaults::starter());
    prefs_router::<()>(state).with_state(())
}

fn principal(role: Role, active_workspace: Option<&str>) -> Principal {
    let extra = match active_workspace {
        Some(ws) => json!({ "active_workspace": ws }),
        None => Value::Null,
    };
    Principal {
        subject: "alice".into(),
        role,
        scopes: Vec::<Scope>::new(),
        extra,
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
    serde_json::from_slice(&body).unwrap_or_else(|e| {
        panic!(
            "failed to decode body as JSON: {e}\nbody: {}",
            String::from_utf8_lossy(&body)
        )
    })
}

async fn body_bytes(resp: axum::response::Response) -> Vec<u8> {
    to_bytes(resp.into_body(), 64 * 1024)
        .await
        .unwrap()
        .to_vec()
}

// ---------------------------------------------------------------------
// /v1/me — GET-after-PATCH reflects the change.
// ---------------------------------------------------------------------

#[tokio::test]
async fn patch_me_then_get_reflects_change() {
    let app = fresh_app().await;
    let me = principal(Role::Reader, Some("ws1"));

    // PATCH temperature_unit to fahrenheit.
    let patch = json!({ "temperature_unit": "fahrenheit", "locale": "en-AU" });
    let resp = app
        .clone()
        .oneshot(
            req_with("PATCH", "/v1/me/preferences", me.clone())
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&patch).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // GET reads it back.
    let resp = app
        .oneshot(
            req_with("GET", "/v1/me/preferences", me)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let rp: ResolvedPreferences = body_json(resp).await;
    assert_eq!(rp.temperature_unit.to_string(), "fahrenheit");
    assert_eq!(rp.locale, "en-AU");
}

// ---------------------------------------------------------------------
// /v1/me — PATCH null reverts to org / default.
// ---------------------------------------------------------------------

#[tokio::test]
async fn patch_me_null_reverts_to_org_then_default() {
    let app = fresh_app().await;
    let admin = principal(Role::Admin, Some("ws1"));
    let me = principal(Role::Reader, Some("ws1"));

    // Org sets locale = en-AU.
    let resp = app
        .clone()
        .oneshot(
            req_with("PATCH", "/v1/orgs/ws1/preferences", admin.clone())
                .header("content-type", "application/json")
                .body(Body::from(json!({"locale": "en-AU"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // User overrides locale = fr-FR.
    let resp = app
        .clone()
        .oneshot(
            req_with("PATCH", "/v1/me/preferences", me.clone())
                .header("content-type", "application/json")
                .body(Body::from(json!({"locale": "fr-FR"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Confirm override took.
    let resp = app
        .clone()
        .oneshot(
            req_with("GET", "/v1/me/preferences", me.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let rp: ResolvedPreferences = body_json(resp).await;
    assert_eq!(rp.locale, "fr-FR");

    // PATCH user locale = null → inherit from org (en-AU).
    let resp = app
        .clone()
        .oneshot(
            req_with("PATCH", "/v1/me/preferences", me.clone())
                .header("content-type", "application/json")
                .body(Body::from(json!({"locale": null}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .clone()
        .oneshot(
            req_with("GET", "/v1/me/preferences", me.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let rp: ResolvedPreferences = body_json(resp).await;
    assert_eq!(rp.locale, "en-AU", "user null reverted to org layer");

    // PATCH org locale = null too → inherit from system default
    // (en-US per SystemDefaults::starter()).
    let resp = app
        .clone()
        .oneshot(
            req_with("PATCH", "/v1/orgs/ws1/preferences", admin)
                .header("content-type", "application/json")
                .body(Body::from(json!({"locale": null}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .oneshot(
            req_with("GET", "/v1/me/preferences", me)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let rp: ResolvedPreferences = body_json(resp).await;
    assert_eq!(rp.locale, "en-US", "org null reverted to system default");
}

// ---------------------------------------------------------------------
// /v1/units — stable JSON + ETag across calls.
// ---------------------------------------------------------------------

#[tokio::test]
async fn get_units_is_byte_stable_with_etag() {
    let app = fresh_app().await;

    let r1 = app
        .clone()
        .oneshot(req("GET", "/v1/units").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(r1.status(), StatusCode::OK);
    let etag1 = r1
        .headers()
        .get("etag")
        .expect("etag header present")
        .clone();
    let platform1 = r1
        .headers()
        .get("x-platform-version")
        .expect("x-platform-version header present")
        .clone();
    let b1 = body_bytes(r1).await;

    let r2 = app
        .oneshot(req("GET", "/v1/units").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let etag2 = r2.headers().get("etag").unwrap().clone();
    let platform2 = r2.headers().get("x-platform-version").unwrap().clone();
    let b2 = body_bytes(r2).await;

    assert_eq!(etag1, etag2, "ETag is stable across calls");
    assert_eq!(platform1, platform2, "X-Platform-Version is stable");
    assert_eq!(b1, b2, "payload bytes are stable");

    // Sanity-check the payload shape — contains the closed quantity
    // set.
    let doc: Value = serde_json::from_slice(&b1).unwrap();
    let qs = doc["quantities"].as_array().unwrap();
    let names: Vec<&str> = qs.iter().map(|q| q["quantity"].as_str().unwrap()).collect();
    for expected in ["temperature", "pressure", "speed", "length", "mass"] {
        assert!(
            names.contains(&expected),
            "missing quantity {expected:?} in {names:?}"
        );
    }
}

// ---------------------------------------------------------------------
// Admin-only paths reject non-admin principals.
// ---------------------------------------------------------------------

#[tokio::test]
async fn get_org_requires_admin_role() {
    let app = fresh_app().await;

    // No principal at all → 401.
    let resp = app
        .clone()
        .oneshot(
            req("GET", "/v1/orgs/ws1/preferences")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Reader principal → 403.
    let resp = app
        .clone()
        .oneshot(
            req_with(
                "GET",
                "/v1/orgs/ws1/preferences",
                principal(Role::Reader, None),
            )
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // Writer principal → 403.
    let resp = app
        .clone()
        .oneshot(
            req_with(
                "GET",
                "/v1/orgs/ws1/preferences",
                principal(Role::Writer, None),
            )
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // Admin → 200.
    let resp = app
        .oneshot(
            req_with(
                "GET",
                "/v1/orgs/ws1/preferences",
                principal(Role::Admin, None),
            )
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn patch_org_requires_admin_role() {
    let app = fresh_app().await;
    let body = || Body::from(json!({"locale": "en-AU"}).to_string());

    let resp = app
        .clone()
        .oneshot(
            req_with(
                "PATCH",
                "/v1/orgs/ws1/preferences",
                principal(Role::Writer, None),
            )
            .header("content-type", "application/json")
            .body(body())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let resp = app
        .oneshot(
            req_with(
                "PATCH",
                "/v1/orgs/ws1/preferences",
                principal(Role::Admin, None),
            )
            .header("content-type", "application/json")
            .body(body())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn me_requires_principal_extension() {
    let app = fresh_app().await;
    let resp = app
        .oneshot(
            req("GET", "/v1/me/preferences")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
