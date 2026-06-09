//! Catalog of built-in ArkFlow output types and their key fields.

use crate::dto::catalog::{ComponentKind, Field, FieldKind::*};

/// The outputs a user can pick from in the builder.
pub fn list() -> Vec<ComponentKind> {
    vec![
        ComponentKind::new(
            "stdout",
            "Stdout",
            "Print each message to the server's standard output.",
            vec![Field::new("append_newline", Bool, false).with("true", "Newline after each message.")],
        ),
        ComponentKind::new(
            "drop",
            "Drop",
            "Discard everything — useful when only side effects matter.",
            vec![],
        ),
        ComponentKind::new(
            "collector",
            "Collector (in-memory)",
            "Capture rows in memory so the UI can display them. Injected automatically on Run.",
            vec![Field::new("run_id", Text, true).with("auto", "Set by the server per run.")],
        ),
        ComponentKind::new(
            "kafka",
            "Kafka",
            "Produce messages to a Kafka topic.",
            vec![
                Field::new("brokers", List, true).with("localhost:9092", "Bootstrap brokers."),
                Field::new("topic", Text, true).with("processed", "Destination topic."),
            ],
        ),
        ComponentKind::new(
            "mqtt",
            "MQTT",
            "Publish messages to an MQTT topic.",
            vec![
                Field::new("host", Text, true).with("localhost", "Broker host."),
                Field::new("port", Number, true).with("1883", "Broker port."),
                Field::new("topic", Text, true).with("out/topic", "Destination topic."),
            ],
        ),
        ComponentKind::new(
            "http",
            "HTTP client",
            "POST each message to an HTTP endpoint.",
            vec![Field::new("url", Text, true).with("https://example.com/hook", "Target URL.")],
        ),
        ComponentKind::new(
            "sql",
            "SQL sink",
            "Insert rows into a database table.",
            vec![
                Field::new("url", Text, true).with("postgres://...", "Connection string."),
                Field::new("table", Text, true).with("readings", "Destination table."),
            ],
        ),
    ]
}
