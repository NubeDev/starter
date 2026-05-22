//! Binding evaluator.
//!
//! Walks a parsed [`Binding`](crate::Binding) against an
//! [`EvalContext`] one step at a time. The evaluator owns no graph
//! state itself — every traversal goes through the
//! [`EntityGraph`](crate::EntityGraph) trait the host implements, and
//! every slot read is funnelled through `read_slot_logged` so the
//! subscription planner sees a complete trace.
//!
//! Length-prefixed evaluation: each step takes
//! `(cursor, json_value) → (cursor', json_value')`. The cursor is
//! `Some(entity_id)` after a graph-rooted source (`$target`,
//! `$self`, `$stack`) or a child walk; it goes `None` once a slot
//! read yields a non-noderef value, at which point further `.ident`
//! steps are field accesses on the JSON value instead of slot reads.

use std::cell::RefCell;
use std::collections::HashMap;

use serde_json::Value as JsonValue;
use thiserror::Error;

use crate::graph::{EntityGraph, EntityId};
use crate::parse::{Binding, Source, Step};
use crate::subscription::SlotAccess;

/// Runtime inputs for evaluating one binding.
///
/// Mirrors Rubix's `dashboard-runtime::EvalContext` in intent but
/// names the fields after the *grammar* sources (`target`, `self_id`,
/// `stack`, `user`, `page`) rather than Rubix's internal types — D5
/// (DIVERGENCE.md) is exactly this rename.
pub struct EvalContext<'a, G: EntityGraph + ?Sized> {
    pub graph: &'a G,
    /// The entity the resolve was issued against. `None` when the
    /// page is not target-scoped; any `$target` binding then errors
    /// with [`BindingError::NoTarget`].
    pub target: Option<&'a str>,
    /// The component the binding is declared on. `None` for trees
    /// authored without ids on bound components — the evaluator
    /// surfaces the structural problem rather than guessing.
    pub self_id: Option<&'a str>,
    /// Named stack frames (`$stack.alias`). Maps alias to entity id.
    pub stack: &'a HashMap<String, EntityId>,
    /// Principal claims (`$user.claim`). JSON object.
    pub user: &'a serde_json::Map<String, JsonValue>,
    /// In-flight page state (`$page.field`). JSON object.
    pub page: &'a serde_json::Map<String, JsonValue>,
    /// Optional recorder for `(entity_id, slot)` slot reads. The
    /// subscription planner consumes this to emit one subject per
    /// unique slot the resolve touched — per-target by construction
    /// because the target id is what seeded the cursor.
    pub access_log: Option<&'a RefCell<Vec<SlotAccess>>>,
}

impl<'a, G: EntityGraph + ?Sized> EvalContext<'a, G> {
    /// Construct an empty context with the given graph. Convenience
    /// for tests; production resolvers will populate `target`,
    /// `stack`, `user`, `page`, and `access_log` explicitly.
    pub fn new(graph: &'a G) -> Self {
        // SAFETY-of-mind: the empty borrowed maps are static so a
        // bare `EvalContext::new(&graph)` is enough to evaluate a
        // `$target`-only binding.
        static EMPTY_STACK: std::sync::OnceLock<HashMap<String, EntityId>> =
            std::sync::OnceLock::new();
        static EMPTY_OBJ: std::sync::OnceLock<serde_json::Map<String, JsonValue>> =
            std::sync::OnceLock::new();
        Self {
            graph,
            target: None,
            self_id: None,
            stack: EMPTY_STACK.get_or_init(HashMap::new),
            user: EMPTY_OBJ.get_or_init(serde_json::Map::new),
            page: EMPTY_OBJ.get_or_init(serde_json::Map::new),
            access_log: None,
        }
    }

    fn record(&self, entity_id: &str, slot: &str) {
        if let Some(log) = self.access_log {
            log.borrow_mut().push(SlotAccess {
                entity_id: entity_id.to_string(),
                slot: slot.to_string(),
            });
        }
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum BindingError {
    #[error("`$target` used but no target is in scope for this resolve")]
    NoTarget,
    #[error("`$self` used but the binding's component has no id")]
    NoSelf,
    #[error("`$stack.{0}` — no frame with that alias is in the context stack")]
    UnknownStackAlias(String),
    #[error("`$user.{0}` — claim not present on the principal")]
    UnknownUserClaim(String),
    #[error("`$page.{0}` — field not present in page state")]
    UnknownPageField(String),
    #[error("entity `{entity}` has no child named `{child}`")]
    UnknownChild { entity: EntityId, child: String },
    #[error("entity `{entity}` has no slot named `{slot}`")]
    UnknownSlot { entity: EntityId, slot: String },
    #[error("cannot walk slot `{slot}` — current value is not an object")]
    WalkThroughNonObject { slot: String },
    #[error("cannot walk child `{child}` — no graph cursor (source is not graph-rooted)")]
    NoCursorForChild { child: String },
}

/// Evaluate one parsed binding against `ctx`.
pub fn evaluate<G: EntityGraph + ?Sized>(
    binding: &Binding,
    ctx: &EvalContext<'_, G>,
) -> Result<JsonValue, BindingError> {
    let (mut cursor, mut value) = seed(binding, ctx)?;

    for step in &binding.steps {
        match step {
            Step::Slot(slot) => {
                if let Some(entity) = cursor.as_ref() {
                    let v = ctx.graph.read_slot(entity, slot).ok_or_else(|| {
                        BindingError::UnknownSlot {
                            entity: entity.clone(),
                            slot: slot.clone(),
                        }
                    })?;
                    ctx.record(entity, slot);
                    // A slot read consumes the cursor: further `.ident`
                    // steps are field accesses on the returned JSON
                    // value, not slot reads on a node. This matches
                    // Rubix's "ref-walk degrades to JSON walk" rule for
                    // non-noderef slot values.
                    cursor = None;
                    value = Some(v);
                } else {
                    let v = value.take().unwrap_or(JsonValue::Null);
                    value = Some(walk_field(&v, slot)?);
                }
            }
            Step::Child(child) => {
                let entity = cursor
                    .as_ref()
                    .ok_or_else(|| BindingError::NoCursorForChild {
                        child: child.clone(),
                    })?;
                let next = ctx
                    .graph
                    .read_children(entity)
                    .into_iter()
                    .find(|c| c.name == *child)
                    .ok_or_else(|| BindingError::UnknownChild {
                        entity: entity.clone(),
                        child: child.clone(),
                    })?;
                cursor = Some(next.id);
                // The value at the cursor is opaque until the next
                // slot read; expose it as Null so a leaf `$target/foo`
                // (no trailing slot) still produces *some* value
                // rather than erroring — matches the SCOPE.md grammar
                // which permits a trailing child step.
                value = Some(JsonValue::Null);
            }
        }
    }

    Ok(value.unwrap_or(JsonValue::Null))
}

/// Seed the cursor + value from the binding's source. Returns
/// `(cursor, value)` — at least one of them is always `Some`.
fn seed<G: EntityGraph + ?Sized>(
    binding: &Binding,
    ctx: &EvalContext<'_, G>,
) -> Result<(Option<EntityId>, Option<JsonValue>), BindingError> {
    match &binding.source {
        Source::Target => {
            let id = ctx.target.ok_or(BindingError::NoTarget)?;
            Ok((Some(id.to_string()), None))
        }
        Source::SelfNode => {
            let id = ctx.self_id.ok_or(BindingError::NoSelf)?;
            Ok((Some(id.to_string()), None))
        }
        Source::Stack { alias } => {
            let id = ctx
                .stack
                .get(alias)
                .ok_or_else(|| BindingError::UnknownStackAlias(alias.clone()))?;
            Ok((Some(id.clone()), None))
        }
        Source::User => {
            // No graph cursor — `$user.claim` is a field walk on the
            // claims object. The evaluator's Step::Slot branch handles
            // the rest.
            Ok((None, Some(JsonValue::Object(ctx.user.clone()))))
        }
        Source::Page => Ok((None, Some(JsonValue::Object(ctx.page.clone())))),
    }
}

fn walk_field(value: &JsonValue, field: &str) -> Result<JsonValue, BindingError> {
    match value.as_object() {
        Some(obj) => Ok(obj.get(field).cloned().unwrap_or(JsonValue::Null)),
        None => Err(BindingError::WalkThroughNonObject {
            slot: field.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{ChildLink, EntityGraph};
    use serde_json::json;

    /// Minimal in-memory graph fixture. `slots[(entity, slot)] = value`
    /// and `children[entity] = [(name, child_id), ...]`.
    #[derive(Default)]
    struct Fixture {
        slots: HashMap<(String, String), JsonValue>,
        children: HashMap<String, Vec<ChildLink>>,
    }
    impl Fixture {
        fn slot(mut self, e: &str, s: &str, v: JsonValue) -> Self {
            self.slots.insert((e.into(), s.into()), v);
            self
        }
        fn child(mut self, parent: &str, name: &str, id: &str) -> Self {
            self.children
                .entry(parent.into())
                .or_default()
                .push(ChildLink {
                    name: name.into(),
                    id: id.into(),
                });
            self
        }
    }
    impl EntityGraph for Fixture {
        fn read_slot(&self, entity_id: &str, slot: &str) -> Option<JsonValue> {
            self.slots.get(&(entity_id.into(), slot.into())).cloned()
        }
        fn read_children(&self, entity_id: &str) -> Vec<ChildLink> {
            self.children.get(entity_id).cloned().unwrap_or_default()
        }
    }

    #[test]
    fn target_child_slot_resolves() {
        let g =
            Fixture::default()
                .child("t1", "temp", "t1.temp")
                .slot("t1.temp", "value", json!(21.5));
        let stack = HashMap::new();
        let user = serde_json::Map::new();
        let page = serde_json::Map::new();
        let ctx = EvalContext {
            graph: &g,
            target: Some("t1"),
            self_id: None,
            stack: &stack,
            user: &user,
            page: &page,
            access_log: None,
        };
        let b = Binding::parse("$target/temp.value").unwrap();
        assert_eq!(evaluate(&b, &ctx).unwrap(), json!(21.5));
    }

    #[test]
    fn missing_target_is_structural_error() {
        let g = Fixture::default();
        let stack = HashMap::new();
        let user = serde_json::Map::new();
        let page = serde_json::Map::new();
        let ctx = EvalContext {
            graph: &g,
            target: None,
            self_id: None,
            stack: &stack,
            user: &user,
            page: &page,
            access_log: None,
        };
        let b = Binding::parse("$target/temp.value").unwrap();
        assert_eq!(evaluate(&b, &ctx).unwrap_err(), BindingError::NoTarget);
    }

    #[test]
    fn user_claim_walk() {
        let g = crate::graph::NullGraph;
        let stack = HashMap::new();
        let mut user = serde_json::Map::new();
        user.insert("orgId".into(), json!("sys"));
        let page = serde_json::Map::new();
        let ctx = EvalContext {
            graph: &g,
            target: None,
            self_id: None,
            stack: &stack,
            user: &user,
            page: &page,
            access_log: None,
        };
        let b = Binding::parse("$user.orgId").unwrap();
        assert_eq!(evaluate(&b, &ctx).unwrap(), json!("sys"));
    }

    #[test]
    fn access_log_records_only_slot_reads() {
        let g =
            Fixture::default()
                .child("t1", "temp", "t1.temp")
                .slot("t1.temp", "value", json!(1));
        let stack = HashMap::new();
        let user = serde_json::Map::new();
        let page = serde_json::Map::new();
        let log = RefCell::new(Vec::new());
        let ctx = EvalContext {
            graph: &g,
            target: Some("t1"),
            self_id: None,
            stack: &stack,
            user: &user,
            page: &page,
            access_log: Some(&log),
        };
        let b = Binding::parse("$target/temp.value").unwrap();
        evaluate(&b, &ctx).unwrap();
        let entries = log.into_inner();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entity_id, "t1.temp");
        assert_eq!(entries[0].slot, "value");
    }
}
