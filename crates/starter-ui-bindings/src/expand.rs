//! Repeat expansion pass (G3).
//!
//! Walks a [`ComponentTree`] and, at every [`Component::Repeat`] node,
//! evaluates the `source` binding to a JSON array, clones `template`
//! once per item, and replaces the `Repeat` in its parent's children
//! with the cloned instances. Each iteration runs against a derived
//! [`EvalContext`] whose `item` and `index` fields are the current
//! frame — synthetic `$item` and `$index` sources in the grammar read
//! these directly.
//!
//! Runs **before** [`crate::substitute_tree`] so that by the time the
//! standard substitution pass walks the tree, every Repeat has
//! already been expanded into normal subtrees.

use serde_json::Value as JsonValue;
use starter_ui_ir::{Component, ComponentTree};

use crate::eval::{evaluate, BindingError, EvalContext};
use crate::graph::EntityGraph;
use crate::parse::Binding;
use crate::substitute::{substitute_subtree, SubstituteError};

/// Errors surfaced by [`expand_repeats`].
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum ExpandError {
    #[error("Repeat.source parse failed: {0}")]
    Parse(#[from] crate::parse::ParseError),
    #[error("Repeat.source evaluation failed: {0}")]
    Eval(#[from] BindingError),
    #[error("Repeat.source did not resolve to a JSON array")]
    NotAnArray,
    #[error("per-iteration substitution failed: {0}")]
    Substitute(#[from] SubstituteError),
}

/// Expand every [`Component::Repeat`] in `tree` against `ctx`. The
/// host's `ctx` typically has `item: None, index: None`; the expander
/// pushes per-iteration frames into derived contexts.
pub fn expand_repeats<G: EntityGraph + ?Sized>(
    tree: &mut ComponentTree,
    ctx: &EvalContext<'_, G>,
) -> Result<(), ExpandError> {
    expand_node(&mut tree.root, ctx)
}

fn expand_node<G: EntityGraph + ?Sized>(
    node: &mut Component,
    ctx: &EvalContext<'_, G>,
) -> Result<(), ExpandError> {
    // First, descend — expanding any Repeats inside child containers.
    // We expand the *children list* in place so a Repeat that produces
    // more Repeats gets handled by the recursive call on each clone.
    expand_children(node, ctx)
}

fn expand_children<G: EntityGraph + ?Sized>(
    node: &mut Component,
    ctx: &EvalContext<'_, G>,
) -> Result<(), ExpandError> {
    match node {
        Component::Page { children, .. }
        | Component::Row { children, .. }
        | Component::Col { children, .. }
        | Component::Grid { children, .. }
        | Component::Section { children, .. }
        | Component::Card { children, .. }
        | Component::Dialog { children, .. }
        | Component::Drawer { children, .. } => {
            expand_vec(children, ctx)?;
        }
        Component::Tabs { tabs, .. } => {
            for t in tabs {
                expand_vec(&mut t.children, ctx)?;
            }
        }
        Component::Wizard { steps, .. } => {
            for step in steps {
                expand_vec(&mut step.children, ctx)?;
            }
        }
        Component::FieldGroup { control, .. } => expand_node(control, ctx)?,
        Component::List { item, .. } => expand_node(item, ctx)?,
        Component::Menu {
            trigger: Some(t), ..
        } => expand_node(t, ctx)?,
        // A bare Repeat at the root has no parent's `children` to be
        // replaced in. Recurse into its template only; the caller is
        // expected to have already handled it via expand_vec.
        Component::Repeat { template, .. } => expand_node(template, ctx)?,
        _ => {}
    }
    Ok(())
}

fn expand_vec<G: EntityGraph + ?Sized>(
    children: &mut Vec<Component>,
    ctx: &EvalContext<'_, G>,
) -> Result<(), ExpandError> {
    let mut out: Vec<Component> = Vec::with_capacity(children.len());
    for child in children.drain(..) {
        match child {
            Component::Repeat {
                id,
                source,
                alias: _,
                template,
            } => {
                let arr = eval_array(&source, ctx)?;
                for (i, item) in arr.iter().enumerate() {
                    let mut clone = (*template).clone();
                    let iter_ctx = with_item_frame(ctx, item, i);
                    // Recurse so nested Repeats inside the template
                    // are expanded against the current frame.
                    expand_node(&mut clone, &iter_ctx)?;
                    // Substitute bindings using the iteration frame so
                    // `{{$item}}` / `{{$index}}` are baked into the
                    // clone before the outer substitute pass runs
                    // (which has no item/index frame).
                    substitute_subtree(&mut clone, &iter_ctx)?;
                    // Stamp a synthetic id derived from the Repeat
                    // node's id + iteration index, so each clone has
                    // a stable, re-resolvable key. When the Repeat
                    // itself was authored without an id, fall back to
                    // the literal "repeat" so the index alone keys
                    // the clones (better than nothing for SSE patch).
                    let parent_key = id.as_deref().unwrap_or("repeat");
                    clone.assign_synthetic_id(parent_key, i);
                    out.push(clone);
                }
            }
            mut other => {
                expand_node(&mut other, ctx)?;
                out.push(other);
            }
        }
    }
    *children = out;
    Ok(())
}

/// Construct an [`EvalContext`] that shares all parent fields but
/// pushes a fresh `(item, index)` frame for one Repeat iteration.
fn with_item_frame<'a, G: EntityGraph + ?Sized>(
    ctx: &'a EvalContext<'_, G>,
    item: &'a JsonValue,
    index: usize,
) -> EvalContext<'a, G> {
    EvalContext {
        graph: ctx.graph,
        target: ctx.target,
        self_id: ctx.self_id,
        stack: ctx.stack,
        user: ctx.user,
        page: ctx.page,
        access_log: ctx.access_log,
        item: Some(item),
        index: Some(index),
        catalogue: ctx.catalogue,
        locale: ctx.locale,
    }
}

/// Parse + evaluate a Repeat.source expression to a JSON array.
/// The grammar tolerates an optional surrounding `{{ ... }}` so the
/// authoring DSL can stay consistent with other binding fields.
fn eval_array<G: EntityGraph + ?Sized>(
    source: &str,
    ctx: &EvalContext<'_, G>,
) -> Result<Vec<JsonValue>, ExpandError> {
    let expr = source
        .trim()
        .strip_prefix("{{")
        .and_then(|s| s.strip_suffix("}}"))
        .unwrap_or(source)
        .trim();
    let binding = Binding::parse(expr)?;
    let v = evaluate(&binding, ctx)?;
    match v {
        JsonValue::Array(a) => Ok(a),
        _ => Err(ExpandError::NotAnArray),
    }
}
