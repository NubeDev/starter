//! Sandbox kill-switch tests: a pathological script must return a clean
//! `LimitExceeded` error, never hang and never panic across the script↔engine
//! edge. These are the security-relevant tests for RW-06.

use std::time::Duration;

use nexus_insights::{run_insight_rows, run_insight_rows_with_limits, InsightError, Limits};
use serde_json::{json, Value};

/// A one-row input frame for scripts that only exercise the sandbox, not the data.
fn seed() -> Vec<Value> {
    vec![json!({ "kw": 1.0 })]
}

/// Run a script over the seed with `limits`, expecting it to fail; return the error.
async fn expect_err(script: &str, limits: Limits) -> InsightError {
    run_insight_rows_with_limits(script.to_string(), seed(), json!({}), limits)
        .await
        .expect_err("script should fail")
}

#[tokio::test]
async fn infinite_loop_trips_max_operations() {
    let limits = Limits {
        max_operations: 100_000,
        deadline: Duration::from_secs(30),
        ..Limits::default()
    };
    let err = expect_err("let i = 0; while true { i += 1; } df", limits).await;
    assert!(matches!(err, InsightError::LimitExceeded(_)), "got {err:?}");
}

#[tokio::test]
async fn huge_string_trips_size_cap() {
    let limits = Limits {
        max_string_size: 1024,
        ..Limits::default()
    };
    let err = expect_err(r#"let s = "x"; loop { s += s; } df"#, limits).await;
    assert!(matches!(err, InsightError::LimitExceeded(_)), "got {err:?}");
}

#[tokio::test]
async fn deadline_fires_on_slow_script() {
    let limits = Limits {
        max_operations: u64::MAX,
        deadline: Duration::from_millis(50),
        ..Limits::default()
    };
    let err = expect_err("let i = 0; while i < 1000000000 { i += 1; } df", limits).await;
    assert!(matches!(err, InsightError::LimitExceeded(_)), "got {err:?}");
}

#[tokio::test]
async fn import_is_disabled() {
    let err = expect_err(r#"import "anything" as m; df"#, Limits::default()).await;
    // A blocked import is a compile/runtime error, never a panic.
    assert!(matches!(
        err,
        InsightError::Compile(_) | InsightError::Runtime(_)
    ));
}

#[tokio::test]
async fn syntax_error_is_compile_error() {
    let err = expect_err("df.select(", Limits::default()).await;
    assert!(matches!(err, InsightError::Compile(_)), "got {err:?}");
}

#[tokio::test]
async fn unknown_column_is_runtime_error() {
    let err = expect_err(r#"df.select(["nope"])"#, Limits::default()).await;
    assert!(matches!(err, InsightError::Runtime(_)), "got {err:?}");
}

#[tokio::test]
async fn explosion_attempt_cannot_grow_rows() {
    // The curated surface has no join; a script that tries to chain reductions
    // can only ever shrink the frame. Even a deliberate attempt stays bounded.
    let rows: Vec<Value> = (0..100).map(|i| json!({ "kw": i as f64 })).collect();
    let out = run_insight_rows(
        r#"df.rolling_mean("kw", 5).head(1000000)"#.to_string(),
        rows,
        json!({}),
    )
    .await
    .expect("bounded script succeeds");
    assert_eq!(out.len(), 100, "row count never exceeds the input");
}
