//! Template substitution helpers.
//!
//! The IR carries bindings inline in `"{{ ... }}"` tags inside string
//! fields (`Component::Text.content`, `Component::Heading.content`,
//! …). [`substitute_text`] parses each tag, evaluates it via
//! [`evaluate`](crate::evaluate), and replaces the tag with the
//! stringified result. [`substitute_tree`] walks a
//! [`ComponentTree`](starter_ui_ir::ComponentTree) and substitutes
//! the textual variants in-place — enough to exercise the "one page,
//! N targets" property end-to-end without porting the entire
//! per-variant Bindable dispatch (that's Phase 3+ work and lands in
//! `starter-ui-builder` / the renderer).

use starter_ui_ir::{Bindable, Component, ComponentTree};

use crate::eval::{evaluate, BindingError, EvalContext};
use crate::graph::EntityGraph;
use crate::parse::{Binding, ParseError, Qualifier};

/// Substitute every `{{ ... }}` tag inside `input` with the
/// evaluated binding's value. Non-string results are rendered with
/// [`serde_json::to_string`] (numbers as `21.5`, objects as JSON);
/// strings render without their surrounding quotes.
///
/// Returns the input unchanged when no tags are present — the common
/// case for layout-only strings (`"Outdoor Temp"` etc).
pub fn substitute_text<G: EntityGraph + ?Sized>(
    input: &str,
    ctx: &EvalContext<'_, G>,
) -> Result<String, SubstituteError> {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 2..];
        let close = after_open
            .find("}}")
            .ok_or_else(|| SubstituteError::Unterminated(rest[open..].to_string()))?;
        let expr = &after_open[..close];
        let binding = Binding::parse(expr.trim()).map_err(SubstituteError::Parse)?;
        let value = match evaluate(&binding, ctx) {
            Ok(v) => v,
            Err(e) if binding.qualifier == Qualifier::Optional => {
                // Optional binding swallows lookup errors and renders
                // as empty. Per the qualifier grammar (G2): a missing
                // claim/slot/child collapses, leaving the surrounding
                // template untouched.
                let _ = e;
                serde_json::Value::Null
            }
            Err(e) => return Err(SubstituteError::Eval(e)),
        };
        out.push_str(&value_to_text(&value));
        rest = &after_open[close + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Walk `tree` and substitute bindings inside the textual variants
/// (`Text.content`, `Heading.content`). Other variants are left
/// untouched — the full per-variant dispatch lands in Phase 3+ via
/// the [`Bindable`](starter_ui_ir::Bindable) trait the IR crate
/// already exposes.
pub fn substitute_tree<G: EntityGraph + ?Sized>(
    tree: &mut ComponentTree,
    ctx: &EvalContext<'_, G>,
) -> Result<(), SubstituteError> {
    walk(&mut tree.root, ctx)
}

/// Same as [`substitute_tree`] but operates on a single `Component`
/// subtree. Exposed for the Repeat expander which needs to substitute
/// each cloned template against a per-iteration context before
/// emitting it.
pub fn substitute_subtree<G: EntityGraph + ?Sized>(
    node: &mut Component,
    ctx: &EvalContext<'_, G>,
) -> Result<(), SubstituteError> {
    walk(node, ctx)
}

fn walk<G: EntityGraph + ?Sized>(
    node: &mut Component,
    ctx: &EvalContext<'_, G>,
) -> Result<(), SubstituteError> {
    // 1) Apply Bindable::visit_bindings on this node's own string
    //    fields. The closure can't return Result, so we stash the
    //    first error in a local Option and short-circuit after the
    //    call. Subsequent visits no-op (the stashed error is checked).
    let mut err: Option<SubstituteError> = None;
    node.visit_bindings(&mut |s| {
        if err.is_some() {
            return;
        }
        match substitute_text(s, ctx) {
            Ok(rewritten) => *s = rewritten,
            Err(e) => err = Some(e),
        }
    });
    if let Some(e) = err {
        return Err(e);
    }

    // 2) Recurse explicitly into child subtrees. visit_bindings is
    //    intentionally per-node — child descent stays here so the
    //    walker controls traversal order.
    match node {
        Component::Page { children, .. }
        | Component::Row { children, .. }
        | Component::Col { children, .. }
        | Component::Grid { children, .. }
        | Component::Section { children, .. }
        | Component::Card { children, .. }
        | Component::Dialog { children, .. }
        | Component::Drawer { children, .. } => {
            for c in children {
                walk(c, ctx)?;
            }
        }
        Component::Tabs { tabs, .. } => {
            for t in tabs {
                for c in &mut t.children {
                    walk(c, ctx)?;
                }
            }
        }
        Component::Wizard { steps, .. } => {
            for step in steps {
                for c in &mut step.children {
                    walk(c, ctx)?;
                }
            }
        }
        Component::Repeat { template, .. } => {
            walk(template, ctx)?;
        }
        Component::List { item, .. } => {
            walk(item, ctx)?;
        }
        Component::FieldGroup { control, .. } => {
            walk(control, ctx)?;
        }
        Component::Menu { trigger: Some(t), .. } => walk(t, ctx)?,
        _ => {}
    }
    Ok(())
}

fn value_to_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum SubstituteError {
    #[error("unterminated `{{ ... }}` tag starting at `{0}`")]
    Unterminated(String),
    #[error(transparent)]
    Parse(ParseError),
    #[error(transparent)]
    Eval(BindingError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::NullGraph;
    use std::collections::HashMap;

    #[test]
    fn no_tags_passthrough() {
        let g = NullGraph;
        let ctx = EvalContext::new(&g);
        assert_eq!(substitute_text("Hello world", &ctx).unwrap(), "Hello world");
    }

    #[test]
    fn user_claim_substituted_inline() {
        let g = NullGraph;
        let stack = HashMap::new();
        let mut user = serde_json::Map::new();
        user.insert("name".into(), serde_json::json!("Ada"));
        let page = serde_json::Map::new();
        let ctx = EvalContext {
            graph: &g,
            target: None,
            self_id: None,
            stack: &stack,
            user: &user,
            page: &page,
            access_log: None,
            item: None,
            index: None,
        };
        assert_eq!(
            substitute_text("Hi {{$user.name}}!", &ctx).unwrap(),
            "Hi Ada!"
        );
    }
}
