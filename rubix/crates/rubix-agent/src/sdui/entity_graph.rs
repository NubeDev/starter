//! `RubixEntityGraph` — rubix's impl of
//! [`starter_ui_bindings::EntityGraph`].
//!
//! Per `03-host-glue.md`, the v1 graph is a deliberately *flat
//! synthetic graph*: it exposes a handful of `system:*`, `flow:<id>`
//! and `user:<id>` entities so bound dashboards have real slots to
//! read without the platform shipping a generic typed-node store.
//! The shape mirrors the spec table — one `match` arm per entity
//! kind. New kinds bolt on as new arms (verb-per-arm).
//!
//! The "flow-engine slot-read seam" the scope mentions is **not**
//! `starter_flow_spi::GraphStore::read_slot` — that trait is keyed on
//! a `(flow_id, slot)` `SlotRef` and only exposes per-run flow
//! outputs. Goal-1 v1 instead routes through a small
//! [`SystemSlotReader`] seam that the boot wiring fills in with the
//! tool registry; tests fill it in with an in-memory fake. When the
//! real flow-engine slot store grows a tenant/system scope (post
//! Goal-1) we can swap the seam without changing this file's `match`.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value as JsonValue;
use starter_store_postgres::pool::Pool;
use starter_ui_bindings::{ChildLink, EntityGraph};

/// Pluggable reader for the synthetic `system:*` entity kind.
///
/// In production the boot wiring registers a reader that dispatches
/// each slot onto the matching `rubix.system.*` tool (cached for ~5s
/// — handled by the boot file, not here). Tests pass a
/// [`StaticSystemReader`] so they can drive the `match` without
/// spinning the whole tool registry.
pub trait SystemSlotReader: Send + Sync + 'static {
    /// Resolve `slot` on the synthetic `system` entity. Returning
    /// `None` makes the binding evaluator surface
    /// `BindingError::UnknownSlot` (unless the binding is `?`).
    fn read(&self, slot: &str) -> Option<JsonValue>;
}

/// Static `system` reader backed by a `HashMap<slot, value>`. Used
/// by the unit tests and as the zero-value default.
#[derive(Debug, Default, Clone)]
pub struct StaticSystemReader {
    slots: HashMap<String, JsonValue>,
}

impl StaticSystemReader {
    /// Empty reader — every slot returns `None`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `slot = value`.
    pub fn with(mut self, slot: impl Into<String>, value: JsonValue) -> Self {
        self.slots.insert(slot.into(), value);
        self
    }
}

impl SystemSlotReader for StaticSystemReader {
    fn read(&self, slot: &str) -> Option<JsonValue> {
        self.slots.get(slot).cloned()
    }
}

/// Rubix's entity graph. Cheap to clone — every inner is an `Arc`.
#[derive(Clone)]
pub struct RubixEntityGraph {
    /// Optional Postgres pool for `flow:<id>` / `user:<id>`
    /// lookups. Tests that exercise only `system:*` pass `None`.
    pool: Option<Pool>,
    /// Pluggable `system:*` slot reader. Always present (defaults
    /// to an empty [`StaticSystemReader`]).
    system: Arc<dyn SystemSlotReader>,
}

impl std::fmt::Debug for RubixEntityGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RubixEntityGraph")
            .field("pool", &self.pool.is_some())
            .finish()
    }
}

impl RubixEntityGraph {
    /// Build with the production wiring (PG pool + system reader).
    pub fn new(pool: Pool, system: Arc<dyn SystemSlotReader>) -> Self {
        Self {
            pool: Some(pool),
            system,
        }
    }

    /// Build a graph with no Postgres pool — every `flow:` /
    /// `user:` lookup returns `None`. Used in tests and on the
    /// laptop boot path.
    pub fn poolless(system: Arc<dyn SystemSlotReader>) -> Self {
        Self {
            pool: None,
            system,
        }
    }
}

impl EntityGraph for RubixEntityGraph {
    fn read_slot(&self, entity_id: &str, slot: &str) -> Option<JsonValue> {
        match split_kind(entity_id) {
            Kind::System => self.system.read(slot),
            // Pool-backed kinds need an async round-trip to PG; the
            // upstream `EntityGraph` trait is sync (the binding
            // evaluator runs inside a single resolve future and
            // re-reads via dedupe). Phase B.2 will move these reads
            // behind a per-resolve `slot_cache` populated in
            // `lookup_page`; until then the v1 surface only answers
            // `system:*` synchronously and returns `None` for
            // everything else — exactly the behaviour the scope
            // doc's "anything else → None" line calls for.
            Kind::Flow(_) | Kind::User(_) | Kind::Unknown => None,
        }
    }

    fn read_children(&self, entity_id: &str) -> Vec<ChildLink> {
        match split_kind(entity_id) {
            // v1 returns no children. The scope doc earmarks
            // tenant→flows / flow→runs / user→teams for Phase B.2.
            _ => Vec::new(),
        }
    }
}

enum Kind<'a> {
    System,
    /// `flow:<id>` — id retained for the future PG-backed read path
    /// even though the v1 `read_slot` arm short-circuits to `None`.
    #[allow(dead_code)]
    Flow(&'a str),
    /// `user:<id>` — id retained for the same reason as `Flow`.
    #[allow(dead_code)]
    User(&'a str),
    Unknown,
}

fn split_kind(entity_id: &str) -> Kind<'_> {
    if entity_id == "system" {
        return Kind::System;
    }
    if let Some(rest) = entity_id.strip_prefix("flow:") {
        return Kind::Flow(rest);
    }
    if let Some(rest) = entity_id.strip_prefix("user:") {
        return Kind::User(rest);
    }
    Kind::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_slot_returns_some_when_reader_has_it() {
        let reader = Arc::new(
            StaticSystemReader::new()
                .with("disk_percent", JsonValue::from(42))
                .with("disk_free_bytes", JsonValue::from(1024_i64)),
        );
        let g = RubixEntityGraph::poolless(reader);
        assert_eq!(g.read_slot("system", "disk_percent"), Some(42.into()));
        assert_eq!(g.read_slot("system", "missing"), None);
    }

    #[test]
    fn unknown_entity_kind_returns_none() {
        let g = RubixEntityGraph::poolless(Arc::new(StaticSystemReader::new()));
        assert_eq!(g.read_slot("zzz:1", "x"), None);
        assert_eq!(g.read_slot("flow:f1", "revision_id"), None);
        assert!(g.read_children("system").is_empty());
    }

    #[test]
    fn split_kind_classifies_each_supported_prefix() {
        assert!(matches!(split_kind("system"), Kind::System));
        assert!(matches!(split_kind("flow:abc"), Kind::Flow("abc")));
        assert!(matches!(split_kind("user:xyz"), Kind::User("xyz")));
        assert!(matches!(split_kind("nope"), Kind::Unknown));
    }
}
