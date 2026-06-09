//! Catalog of built-in ArkFlow buffer types and their key fields.

use crate::dto::catalog::{ComponentKind, Field, FieldKind::*};

/// The buffers a user can place between input and pipeline.
pub fn list() -> Vec<ComponentKind> {
    vec![
        ComponentKind::new(
            "memory",
            "Memory",
            "Bounded in-memory queue that flushes on capacity or timeout.",
            vec![
                Field::new("capacity", Number, true).with("100", "Max buffered messages."),
                Field::new("timeout", Duration, true).with("10s", "Flush after this long."),
            ],
        ),
        ComponentKind::new(
            "tumbling_window",
            "Tumbling window",
            "Group messages into fixed, non-overlapping time windows.",
            vec![Field::new("interval", Duration, true).with("5s", "Window length.")],
        ),
        ComponentKind::new(
            "sliding_window",
            "Sliding window",
            "Overlapping windows advanced by a fixed step.",
            vec![
                Field::new("window_size", Number, true).with("10", "Messages per window."),
                Field::new("interval", Duration, true).with("1s", "Advance interval."),
            ],
        ),
        ComponentKind::new(
            "session_window",
            "Session window",
            "Group messages separated by gaps shorter than the timeout.",
            vec![Field::new("gap", Duration, true).with("30s", "Idle gap that closes a session.")],
        ),
    ]
}
