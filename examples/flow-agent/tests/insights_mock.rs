//! S2 — insights_mock REST surface smoke.
//!
//! Loads the seeded fixtures, exercises the router directly via
//! `tower::ServiceExt::oneshot`, and asserts that the JSON shapes
//! survive a round trip for list/detail/filter/dry-run/upsert.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::path::PathBuf;
use tower::ServiceExt;

use flow_agent::insights_mock::{router, InsightsFixtures, InsightsState};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/insights")
}

fn fresh_state_with_tmp_copy() -> (InsightsState, tempfile::TempDir) {
    // Copy fixtures into a tmp dir so the test doesn't mutate the
    // checked-in seed data when exercising writes.
    let tmp = tempfile::tempdir().expect("tempdir");
    for name in [
        "rules.json",
        "verdicts.json",
        "pipelines.json",
        "coverage.json",
        "tags-index.json",
    ] {
        std::fs::copy(fixtures_dir().join(name), tmp.path().join(name)).expect("copy fixture");
    }
    let data = InsightsFixtures::load(tmp.path()).expect("load fixtures");
    (InsightsState::new(data), tmp)
}

async fn body_json(resp: axum::http::Response<Body>) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn list_rules_returns_seed_data() {
    let (state, _tmp) = fresh_state_with_tmp_copy();
    let app: axum::Router = router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/insights/rules")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    let arr = v.as_array().expect("array");
    assert!(arr.len() >= 7, "expected ≥7 seed rules, got {}", arr.len());
    assert!(arr.iter().any(|r| r["id"] == "meter.baseline-deviation@1"));
}

#[tokio::test]
async fn get_rule_detail_and_404() {
    let (state, _tmp) = fresh_state_with_tmp_copy();
    let app: axum::Router = router(state);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/insights/rules/device.online@1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["id"], "device.online@1");

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/insights/rules/no.such.rule@9")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_verdicts_filters_by_rule_id_and_severity() {
    let (state, _tmp) = fresh_state_with_tmp_copy();
    let app: axum::Router = router(state);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/insights/verdicts?rule_id=meter.baseline-deviation%401")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let arr = body_json(resp).await;
    let arr = arr.as_array().unwrap();
    assert!(!arr.is_empty());
    assert!(arr
        .iter()
        .all(|v| v["rule_id"] == "meter.baseline-deviation@1"));

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/insights/verdicts?severity=Critical")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let arr = body_json(resp).await;
    let arr = arr.as_array().unwrap();
    assert!(arr.iter().all(|v| v["severity"] == "Critical"));
    assert!(!arr.is_empty(), "expected at least one Critical verdict");
}

#[tokio::test]
async fn dry_run_synthesises_from_latest_verdict() {
    let (state, _tmp) = fresh_state_with_tmp_copy();
    let app: axum::Router = router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/insights/rules/meter.baseline-deviation@1/dry-run")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["dry_run"], true);
    assert_eq!(v["rule_id"], "meter.baseline-deviation@1");
}

#[tokio::test]
async fn upsert_pipeline_round_trips_to_disk() {
    let (state, tmp) = fresh_state_with_tmp_copy();
    let app: axum::Router = router(state);

    let body = json!({
        "id": "new-iot-thresholds",
        "name": "new",
        "description": "agent-added",
        "tags": ["domain:iot"],
        "graph": {"nodes": [], "edges": []}
    });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/insights/pipelines")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let on_disk: Value =
        serde_json::from_slice(&std::fs::read(tmp.path().join("pipelines.json")).unwrap()).unwrap();
    let arr = on_disk.as_array().unwrap();
    assert!(arr.iter().any(|p| p["id"] == "new-iot-thresholds"));
}
