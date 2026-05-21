//! The host-supplied entity graph the binding engine walks.
//!
//! D5 (DIVERGENCE.md): starter does not pin the binding engine to one
//! node-graph implementation. Consumers implement [`EntityGraph`]
//! against whatever they have — a database, a service layer, an
//! in-memory fixture, a Rubix-style typed node store. The grammar
//! itself (`$target/child.slot`) is unchanged.
//!
//! Per **S-D1** (SCOPE.md § Decisions) the trait stays here, in
//! `starter-ui-bindings`, until a **second** SDUI consumer wants it.
//! Promotion to `starter-spi` is mechanical; demotion isn't, so we
//! wait for the signal.

use serde_json::Value as JsonValue;

/// Stable identifier for a node in the host graph.
///
/// A `String` rather than a typed newtype because the grammar is
/// host-agnostic by design — Rubix uses UUIDs, a SQL-backed consumer
/// might use `"buildings/42"`, a fixture might use `"target-1"`.
/// Hosts that want stricter typing can validate via
/// [`EntityGraph::entity_id_regex`] (see below).
pub type EntityId = String;

/// One named child slot returned by [`EntityGraph::read_children`].
///
/// `name` is the path segment after the `/` in the grammar
/// (`$target/temp` → `name = "temp"`); `id` is the resolved child's
/// entity id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildLink {
    pub name: String,
    pub id: EntityId,
}

/// The host's view of its own entity graph, presented as the two
/// operations the binding grammar needs: read a named slot, walk to a
/// named child.
///
/// Why these three methods and no more:
///
/// - [`read_slot`](EntityGraph::read_slot) implements the `.` operator
///   in the grammar — every leaf binding ends in a slot read, so this
///   is the only method strictly required to resolve a value.
/// - [`read_children`](EntityGraph::read_children) implements the `/`
///   operator. Returning the full child list (rather than a
///   `read_child(parent, name)` lookup) keeps the trait surface small
///   while letting hosts that don't natively index children by name
///   answer in one pass; the binding evaluator filters by name itself.
/// - [`entity_id_regex`](EntityGraph::entity_id_regex) is optional and
///   defaults to `None`. It exists for **ai-builder R7** (see
///   `DOCS/frontend/ai-builder/SCOPE.md`) — when the AI authoring
///   surface wants to suggest target ids, hosts with a stable id
///   format expose its regex here and the suggester can validate
///   candidates locally. Hosts without a fixed format (UUIDs, opaque
///   strings) return `None` and the suggester accepts anything.
///
/// Implementations should be **side-effect free** for the duration of
/// a resolve call — the evaluator may re-read the same `(id, slot)`
/// pair during one binding (the `read_slot` path that backs a
/// noderef-style chain) and through the subscription planner's
/// dedupe.
pub trait EntityGraph {
    /// Read a single named slot on `entity_id`. `None` distinguishes
    /// "entity missing" / "slot not declared" from "slot present and
    /// holds `JsonValue::Null`" — callers preserve both signals.
    fn read_slot(&self, entity_id: &str, slot: &str) -> Option<JsonValue>;

    /// List the children of `entity_id`. Order is host-defined; the
    /// evaluator looks children up by name, so a stable order matters
    /// only when two children share a name (in which case the first
    /// match wins — same rule as Rubix's `dashboard-runtime`).
    fn read_children(&self, entity_id: &str) -> Vec<ChildLink>;

    /// Optional regex describing the host's entity id format. `None`
    /// when ids are opaque (UUIDs, free-form strings). Used by
    /// ai-builder R7 to validate suggested target ids without round-
    /// tripping through the host; binding evaluation does not consult
    /// it.
    fn entity_id_regex(&self) -> Option<&str> {
        None
    }
}

/// A graph with no entities — useful for resolving bindings that
/// only touch `$user` / `$page` / `$vars`, and for tests that want a
/// `&dyn EntityGraph` placeholder. Every read returns `None`; the
/// child list is always empty.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullGraph;

impl EntityGraph for NullGraph {
    fn read_slot(&self, _entity_id: &str, _slot: &str) -> Option<JsonValue> {
        None
    }
    fn read_children(&self, _entity_id: &str) -> Vec<ChildLink> {
        Vec::new()
    }
}
