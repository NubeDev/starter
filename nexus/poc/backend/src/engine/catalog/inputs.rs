//! Catalog of built-in ArkFlow input types and their key fields.

use crate::dto::catalog::{ComponentKind, Field, FieldKind::*};

/// The inputs a user can pick from in the builder.
pub fn list() -> Vec<ComponentKind> {
    vec![
        ComponentKind::new(
            "generate",
            "Generate",
            "Emit a fixed JSON message on an interval — handy for testing.",
            vec![
                Field::new("context", Code, true)
                    .with(r#"{ "value": 10, "sensor": "temp_1" }"#, "JSON document emitted each tick."),
                Field::new("interval", Duration, true).with("1s", "Time between batches, e.g. 1s, 500ms."),
                Field::new("batch_size", Number, false).with("10", "Messages per batch."),
                Field::new("count", Number, false).with("100", "Stop after N messages (omit = forever)."),
            ],
        ),
        ComponentKind::new(
            "memory",
            "Memory",
            "Replay an inline list of JSON messages, then finish.",
            vec![Field::new("messages", List, true)
                .with(r#"{"v":1}"#, "One JSON document per line; the stream ends once drained.")],
        ),
        ComponentKind::new(
            "http",
            "HTTP server",
            "Receive messages over an HTTP endpoint this stream hosts.",
            vec![
                Field::new("address", Text, true).with("0.0.0.0:8090", "Bind address for the listener."),
                Field::new("path", Text, true).with("/ingest", "Route messages are POSTed to."),
            ],
        ),
        ComponentKind::new(
            "kafka",
            "Kafka",
            "Consume from one or more Kafka topics.",
            vec![
                Field::new("brokers", List, true).with("localhost:9092", "Bootstrap brokers."),
                Field::new("topics", List, true).with("events", "Topics to subscribe to."),
                Field::new("consumer_group", Text, true).with("nexus-poc", "Consumer group id."),
                Field::new("start_from_latest", Bool, false).with("false", "Start at latest vs earliest offset."),
            ],
        ),
        ComponentKind::new(
            "mqtt",
            "MQTT",
            "Subscribe to MQTT topics from a broker.",
            vec![
                Field::new("host", Text, true).with("localhost", "Broker host."),
                Field::new("port", Number, true).with("1883", "Broker port."),
                Field::new("client_id", Text, true).with("nexus-poc", "MQTT client id."),
                Field::new("topics", List, true).with("sensors/#", "Topic filters."),
            ],
        ),
        ComponentKind::new(
            "sql",
            "SQL source",
            "Pull rows from a file, object store, or database into the pipeline.",
            vec![Field::new("select_sql", Code, true)
                .with("SELECT * FROM read_csv('data.csv')", "DataFusion SELECT executed against the source.")],
        ),
    ]
}
