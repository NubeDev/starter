//! The simulator input emits synthetic device telemetry for the pipeline. Here
//! the `hvac` profile is driven through the live runner into the broadcast sink,
//! shaped with SQL — the same input+pipeline path a real flow takes, with no
//! external dependency. The first emit fires immediately, so the row arrives
//! without waiting a full interval.

use std::time::Duration;

use nexus_engine::sink::broadcast_store;
use nexus_engine::stream_registry::{attach, register, Attach};
use nexus_engine::{LiveRunner, StreamKey};
use serde_json::json;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn simulator_feeds_a_stream_with_synthetic_rows() {
    let runner = LiveRunner::new().expect("register builders");
    let key = StreamKey {
        spec: "simulator-test".into(),
        datasource_id: "sim".into(),
        tenant_id: "acme".into(),
        permission: "view".into(),
    };
    let run_id = "simulator-run";
    let mut sub = match attach(&key, run_id) {
        Attach::StartNew { run_id } => {
            let token = CancellationToken::new();
            let input = json!({
                "type": "simulator",
                "profile": "hvac",
                "interval": "1s",
                "device_id": "ahu-1",
                "seed": 7,
            });
            let processors = vec![
                json!({ "type": "json_to_arrow" }),
                json!({ "type": "sql", "query": "SELECT device_id, temp_c, fan_speed FROM flow" }),
            ];
            runner
                .spawn(input, processors, &run_id, token.clone())
                .expect("spawn");
            register(key.clone(), run_id, token)
        }
        Attach::Existing(_) => panic!("first attach starts new"),
    };

    let event = tokio::time::timeout(Duration::from_secs(5), sub.receiver().recv())
        .await
        .expect("event in time")
        .expect("event");
    assert_eq!(event.rows[0]["device_id"], "ahu-1");
    // hvac temps stay in the simulated 18..=24 band.
    let temp = event.rows[0]["temp_c"].as_f64().expect("temp_c is a number");
    assert!((18.0..=24.0).contains(&temp), "temp {temp} in band");

    drop(sub);
    broadcast_store::close(run_id);
}
