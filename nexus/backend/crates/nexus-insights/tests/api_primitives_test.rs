//! Golden-frame tests for each curated primitive: run a one-line script through
//! `run_insight_rows` and assert the transformed rows. These exercise the full
//! Rhai → engine → DataFusion → JSON path, so they catch a broken SQL lowering as
//! well as a broken registration.

use nexus_insights::run_insight_rows;
use serde_json::{json, Value};

/// Run `script` over `rows` with empty params, returning the result rows.
async fn run(script: &str, rows: Vec<Value>) -> Vec<Value> {
    run_insight_rows(script.to_string(), rows, json!({}))
        .await
        .expect("insight should succeed")
}

fn sample() -> Vec<Value> {
    vec![
        json!({ "kw": 10.0, "site": "A" }),
        json!({ "kw": 20.0, "site": "B" }),
        json!({ "kw": 30.0, "site": "A" }),
    ]
}

#[tokio::test]
async fn select_keeps_named_columns() {
    let out = run(r#"df.select(["kw"])"#, sample()).await;
    assert_eq!(out.len(), 3);
    assert!(out[0].get("kw").is_some());
    assert!(out[0].get("site").is_none());
}

#[tokio::test]
async fn rename_changes_one_column() {
    let out = run(r#"df.rename("kw", "power")"#, sample()).await;
    assert!(out[0].get("power").is_some());
    assert!(out[0].get("kw").is_none());
}

#[tokio::test]
async fn filter_gt_reduces_rows() {
    let out = run(r#"df.filter_gt("kw", 15.0)"#, sample()).await;
    assert_eq!(out.len(), 2);
}

#[tokio::test]
async fn filter_eq_on_string() {
    let out = run(r#"df.filter_eq("site", "A")"#, sample()).await;
    assert_eq!(out.len(), 2);
}

#[tokio::test]
async fn rolling_mean_adds_column_same_rows() {
    let out = run(r#"df.rolling_mean("kw", 2)"#, sample()).await;
    assert_eq!(out.len(), 3);
    assert!(out[0].get("kw_roll_mean").is_some());
    // First row's 2-row trailing mean is just itself.
    assert_eq!(out[0]["kw_roll_mean"], json!(10.0));
    assert_eq!(out[1]["kw_roll_mean"], json!(15.0));
}

#[tokio::test]
async fn rolling_sum_min_max() {
    let out = run(
        r#"df.rolling_sum("kw", 3).rolling_min("kw", 3).rolling_max("kw", 3)"#,
        sample(),
    )
    .await;
    assert_eq!(out[2]["kw_roll_sum"], json!(60.0));
    assert_eq!(out[2]["kw_roll_min"], json!(10.0));
    assert_eq!(out[2]["kw_roll_max"], json!(30.0));
}

#[tokio::test]
async fn lag_diff_pct_change() {
    let out = run(r#"df.lag("kw", 1).diff("kw").pct_change("kw")"#, sample()).await;
    assert_eq!(out.len(), 3);
    assert!(out[0]["kw_lag"].is_null());
    assert_eq!(out[1]["kw_lag"], json!(10.0));
    assert_eq!(out[1]["kw_diff"], json!(10.0));
    assert_eq!(out[1]["kw_pct_change"], json!(1.0));
}

#[tokio::test]
async fn zscore_is_centered() {
    let out = run(r#"df.zscore("kw")"#, sample()).await;
    // Mean 20, so the middle row's zscore is 0.
    assert_eq!(out[1]["kw_zscore"], json!(0.0));
}

#[tokio::test]
async fn anomalies_flags_outlier() {
    let mut rows = sample();
    rows.push(json!({ "kw": 1000.0, "site": "A" }));
    let out = run(r#"df.anomalies("kw", 1.5)"#, rows).await;
    let flagged = out
        .iter()
        .filter(|r| r["kw_anomaly"] == json!(true))
        .count();
    assert_eq!(flagged, 1);
}

#[tokio::test]
async fn head_and_tail() {
    let head = run(r#"df.head(2)"#, sample()).await;
    assert_eq!(head.len(), 2);
    assert_eq!(head[0]["kw"], json!(10.0));
    let tail = run(r#"df.tail(1)"#, sample()).await;
    assert_eq!(tail.len(), 1);
    assert_eq!(tail[0]["kw"], json!(30.0));
}

#[tokio::test]
async fn sort_descending() {
    let out = run(r#"df.sort("kw", false)"#, sample()).await;
    assert_eq!(out[0]["kw"], json!(30.0));
    assert_eq!(out[2]["kw"], json!(10.0));
}

#[tokio::test]
async fn fill_null_zero_and_mean() {
    let rows = vec![
        json!({ "kw": 10.0 }),
        json!({ "kw": Value::Null }),
        json!({ "kw": 30.0 }),
    ];
    let zero = run(r#"df.fill_null("kw", "zero")"#, rows.clone()).await;
    assert_eq!(zero[1]["kw"], json!(0.0));
    let mean = run(r#"df.fill_null("kw", "mean")"#, rows).await;
    assert_eq!(mean[1]["kw"], json!(20.0));
}

#[tokio::test]
async fn describe_emits_statistic_rows() {
    let out = run(r#"df.describe()"#, sample()).await;
    let labels: Vec<&str> = out.iter().filter_map(|r| r["statistic"].as_str()).collect();
    assert!(labels.contains(&"count"));
    assert!(labels.contains(&"mean"));
    assert!(labels.contains(&"max"));
}

#[tokio::test]
async fn resample_buckets_by_time() {
    let rows = vec![
        json!({ "ts": "2024-01-01T00:00:00Z", "kw": 10.0 }),
        json!({ "ts": "2024-01-01T00:30:00Z", "kw": 20.0 }),
        json!({ "ts": "2024-01-01T01:00:00Z", "kw": 30.0 }),
    ];
    let out = run(
        r#"df.resample("ts", "1 hour", [#{ col: "kw", func: "avg" }])"#,
        rows,
    )
    .await;
    // Two hourly buckets: [00:00, 01:00) avg 15, [01:00, 02:00) avg 30.
    assert_eq!(out.len(), 2);
    assert_eq!(out[0]["kw"], json!(15.0));
    assert_eq!(out[1]["kw"], json!(30.0));
}

#[tokio::test]
async fn chained_pipeline() {
    let rows = vec![
        json!({ "ts": "2024-01-01T00:00:00Z", "kw": 10.0 }),
        json!({ "ts": "2024-01-01T00:30:00Z", "kw": 20.0 }),
        json!({ "ts": "2024-01-01T01:00:00Z", "kw": 1000.0 }),
    ];
    let out = run(
        r#"df.resample("ts", "1 hour", [#{ col: "kw", func: "avg" }]).anomalies("kw", 0.9)"#,
        rows,
    )
    .await;
    assert_eq!(out.len(), 2);
    assert!(out.iter().any(|r| r["kw_anomaly"] == json!(true)));
}
