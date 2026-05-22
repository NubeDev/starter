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

use starter_ui_ir::{Component, ComponentTree};

use crate::eval::{evaluate, BindingError, EvalContext};
use crate::graph::EntityGraph;
use crate::parse::{Binding, ParseError};

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
        let value = evaluate(&binding, ctx).map_err(SubstituteError::Eval)?;
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

fn walk<G: EntityGraph + ?Sized>(
    node: &mut Component,
    ctx: &EvalContext<'_, G>,
) -> Result<(), SubstituteError> {
    match node {
        Component::Text { content, .. } | Component::Heading { content, .. } => {
            *content = substitute_text(content, ctx)?;
        }
        Component::Page { children, .. }
        | Component::Row { children, .. }
        | Component::Col { children, .. } => {
            for c in children {
                walk(c, ctx)?;
            }
        }
        // Other variants pass through unchanged in Phase 2 — Phase 3
        // (builder) and Phase 4 (renderer) port the full Bindable
        // dispatch onto the rest of the IR.
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
        };
        assert_eq!(
            substitute_text("Hi {{$user.name}}!", &ctx).unwrap(),
            "Hi Ada!"
        );
    }
}
