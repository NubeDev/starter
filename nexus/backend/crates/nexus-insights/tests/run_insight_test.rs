//! End-to-end `run_insight_rows` tests: params binding and a realistic
//! simulator-shaped script.

use nexus_insights::run_insight_rows;
use serde_json::{json, Value};

#[tokio::test]
async fn params_are_visible_to_the_script() {
    let rows = vec![
        json!({ "kw": 10.0 }),
        json!({ "kw": 50.0 }),
        json!({ "kw": 90.0 }),
    ];
    // The threshold comes from params, not the script text.
    let out = run_insight_rows(
        r#"df.filter_gt("kw", params.threshold)"#.to_string(),
        rows,
        json!({ "threshold": 40.0 }),
    )
    .await
    .expect("params script succeeds");
    assert_eq!(out.len(), 2);
}

#[tokio::test]
async fn realistic_anomaly_pipeline_over_timeseries() {
    // A simulator-shaped series: a steady signal with one spike.
    let mut rows: Vec<Value> = (0..24)
        .map(|h| {
            json!({
                "ts": format!("2024-01-01T{h:02}:00:00Z"),
                "kw": 100.0,
            })
        })
        .collect();
    rows[12] = json!({ "ts": "2024-01-01T12:00:00Z", "kw": 500.0 });

    let out = run_insight_rows(
        r#"
            df.resample("ts", "1 hour", [#{ col: "kw", func: "avg" }])
              .anomalies("kw", 2.0)
        "#
        .to_string(),
        rows,
        json!({}),
    )
    .await
    .expect("realistic pipeline succeeds");

    assert_eq!(out.len(), 24, "one row per hourly bucket");
    let anomalies = out.iter().filter(|r| r["kw_anomaly"] == json!(true)).count();
    assert_eq!(anomalies, 1, "only the spike is flagged");
}

#[tokio::test]
async fn empty_input_is_an_empty_result_not_a_panic() {
    let out = run_insight_rows(r#"df"#.to_string(), vec![], json!({}))
        .await
        .expect("empty insight succeeds");
    assert!(out.is_empty());
}

#[tokio::test]
async fn non_frame_result_is_a_runtime_error() {
    // A script whose final expression is not a Frame must fail cleanly, not panic.
    let err = run_insight_rows(r#"42"#.to_string(), vec![json!({ "a": 1 })], json!({}))
        .await
        .expect_err("a non-frame result must error");
    assert!(format!("{err}").contains("insight"));
}
