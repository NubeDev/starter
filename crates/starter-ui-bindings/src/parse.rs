//! Parser for the binding grammar (SCOPE.md § Data bindings).
//!
//! The parser is **stateless** — it knows nothing about the entity
//! graph, the user, or the page. It emits a [`Binding`] whose
//! `source` + `steps` capture the full intent of the expression; the
//! evaluator (`eval.rs`) does all the resolution. This split is what
//! lets one parsed expression be evaluated against N different
//! targets without re-parsing (the "one page, N targets" property
//! load-bearing for AI-generated pages).

use thiserror::Error;

/// The source selector at the head of a binding expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// `$target` — the entity the resolve was issued against.
    Target,
    /// `$self` — the component the binding is declared on.
    SelfNode,
    /// `$stack.alias` — a named frame from the context stack.
    Stack { alias: String },
    /// `$user` — the principal making the request. The first `.field`
    /// step is the claim name; further steps walk into the claim's
    /// JSON structure.
    User,
    /// `$page` — the in-flight `page_state` JSON object. The first
    /// `.field` step picks a top-level key; further steps walk
    /// nested values.
    Page,
    /// `$item` — synthetic source pushed by the Repeat expander.
    /// Resolves to the current iteration's array element. Further
    /// `.ident` steps walk into the item's JSON shape.
    Item,
    /// `$index` — synthetic source pushed by the Repeat expander.
    /// Resolves to the zero-based iteration index as a JSON number.
    Index,
    /// `$msg` — catalogue lookup. The first `.ident` step is the
    /// message key; further steps walk into the resolved JSON value.
    Msg,
}

/// One walk step in the binding's path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// `.ident` — read a slot on the current cursor, or a field on
    /// the current JSON value when the cursor is unset.
    Slot(String),
    /// `/ident` — walk to a named child of the current cursor.
    Child(String),
}

/// Qualifier suffix on a binding expression. `?` makes the binding
/// optional — lookup errors collapse to an empty Null value instead
/// of an evaluator error. `!` is the explicit "required" form;
/// behaviour is identical to `Default` today but reserved so authors
/// can declare intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Qualifier {
    Default,
    Optional,
    Required,
}

/// A parsed binding expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub source: Source,
    /// Steps in declaration order. Length-prefixed evaluation: the
    /// state after step *N* is fully determined by steps `0..=N`.
    pub steps: Vec<Step>,
    /// Trailing `?` (Optional) or `!` (Required) qualifier; defaults
    /// to `Default` when absent.
    pub qualifier: Qualifier,
}

/// Parse error variants. Carried structurally so the resolver can
/// branch without parsing free text.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("empty binding expression")]
    Empty,
    #[error(
        "binding must start with `$target`, `$self`, `$stack`, `$user`, or `$page` — got `{0}`"
    )]
    UnknownSource(String),
    #[error("`$stack` must be followed by `.<alias>`")]
    StackNeedsAlias,
    #[error("empty identifier after `{0}`")]
    EmptyIdent(&'static str),
    #[error("unexpected trailing characters: `{0}`")]
    Trailing(String),
}

impl Binding {
    /// Parse one expression. The argument is the *body* of the
    /// `{{ ... }}` template — whitespace inside the braces is
    /// trimmed by [`substitute_text`](crate::substitute_text) before
    /// the call.
    pub fn parse(expr: &str) -> Result<Self, ParseError> {
        let mut expr = expr.trim();
        if expr.is_empty() {
            return Err(ParseError::Empty);
        }
        // Trailing qualifier — strip first so subsequent ident parsing
        // does not see the `?` / `!`.
        let qualifier = match expr.as_bytes().last().copied() {
            Some(b'?') => {
                expr = &expr[..expr.len() - 1];
                Qualifier::Optional
            }
            Some(b'!') => {
                expr = &expr[..expr.len() - 1];
                Qualifier::Required
            }
            _ => Qualifier::Default,
        };
        let expr = expr.trim_end();
        if expr.is_empty() {
            return Err(ParseError::Empty);
        }
        if !expr.starts_with('$') {
            return Err(ParseError::UnknownSource(expr.to_string()));
        }

        // Split the source token — everything up to the first `.`,
        // `/`, or end of string. `$stack` is the only source whose
        // alias is part of the source token; we handle that after.
        let head_end = expr.find(['.', '/']).unwrap_or(expr.len());
        let head = &expr[..head_end];
        let mut rest = &expr[head_end..];

        let source = match head {
            "$target" => Source::Target,
            "$self" => Source::SelfNode,
            "$user" => Source::User,
            "$page" => Source::Page,
            "$item" => Source::Item,
            "$index" => Source::Index,
            "$msg" => Source::Msg,
            "$stack" => {
                // Required `.alias` segment. We do not allow
                // `$stack/foo` — `$stack` itself is not a graph
                // cursor, only its frame is.
                let alias_rest = rest.strip_prefix('.').ok_or(ParseError::StackNeedsAlias)?;
                let (alias, after) = take_ident(alias_rest, "$stack.")?;
                rest = after;
                Source::Stack {
                    alias: alias.to_string(),
                }
            }
            other => return Err(ParseError::UnknownSource(other.to_string())),
        };

        let mut steps = Vec::new();
        while !rest.is_empty() {
            let (op, after_op) = match rest.as_bytes()[0] {
                b'.' => (StepOp::Slot, &rest[1..]),
                b'/' => (StepOp::Child, &rest[1..]),
                _ => return Err(ParseError::Trailing(rest.to_string())),
            };
            let label = match op {
                StepOp::Slot => ".",
                StepOp::Child => "/",
            };
            let (ident, after) = take_ident(after_op, label)?;
            steps.push(match op {
                StepOp::Slot => Step::Slot(ident.to_string()),
                StepOp::Child => Step::Child(ident.to_string()),
            });
            rest = after;
        }

        Ok(Self {
            source,
            steps,
            qualifier,
        })
    }
}

#[derive(Copy, Clone)]
enum StepOp {
    Slot,
    Child,
}

/// Take an identifier (`[A-Za-z0-9_-]+`) and return it plus the
/// remainder. `label` is the preceding operator string used in the
/// error message ("$stack.", ".", "/") so the diagnostic points at
/// what was expected.
fn take_ident<'a>(s: &'a str, label: &'static str) -> Result<(&'a str, &'a str), ParseError> {
    let end = s.find(['.', '/']).unwrap_or(s.len());
    if end == 0 {
        return Err(ParseError::EmptyIdent(label));
    }
    Ok((&s[..end], &s[end..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bare_sources() {
        assert_eq!(Binding::parse("$target").unwrap().source, Source::Target);
        assert_eq!(Binding::parse("$self").unwrap().source, Source::SelfNode);
        assert_eq!(Binding::parse("$user").unwrap().source, Source::User);
        assert_eq!(Binding::parse("$page").unwrap().source, Source::Page);
    }

    #[test]
    fn parse_stack_alias() {
        let b = Binding::parse("$stack.target").unwrap();
        assert_eq!(
            b.source,
            Source::Stack {
                alias: "target".into()
            }
        );
        assert!(b.steps.is_empty());
    }

    #[test]
    fn parse_target_child_then_slot() {
        // The worked example from SCOPE.md.
        let b = Binding::parse("$target/temp.value").unwrap();
        assert_eq!(b.source, Source::Target);
        assert_eq!(
            b.steps,
            vec![Step::Child("temp".into()), Step::Slot("value".into())]
        );
    }

    #[test]
    fn parse_multi_child_walk() {
        let b = Binding::parse("$target/site/owner.name").unwrap();
        assert_eq!(
            b.steps,
            vec![
                Step::Child("site".into()),
                Step::Child("owner".into()),
                Step::Slot("name".into()),
            ]
        );
    }

    #[test]
    fn parse_user_claim_chain() {
        let b = Binding::parse("$user.orgId").unwrap();
        assert_eq!(b.source, Source::User);
        assert_eq!(b.steps, vec![Step::Slot("orgId".into())]);
    }

    #[test]
    fn parse_rejects_unknown_source() {
        assert!(matches!(
            Binding::parse("$unknown.x").unwrap_err(),
            ParseError::UnknownSource(_)
        ));
        assert!(matches!(
            Binding::parse("foo").unwrap_err(),
            ParseError::UnknownSource(_)
        ));
        assert_eq!(Binding::parse("").unwrap_err(), ParseError::Empty);
    }

    #[test]
    fn parse_rejects_bare_stack() {
        assert_eq!(
            Binding::parse("$stack").unwrap_err(),
            ParseError::StackNeedsAlias
        );
    }

    #[test]
    fn parse_rejects_empty_segments() {
        assert!(matches!(
            Binding::parse("$target/").unwrap_err(),
            ParseError::EmptyIdent("/")
        ));
        assert!(matches!(
            Binding::parse("$target/temp.").unwrap_err(),
            ParseError::EmptyIdent(".")
        ));
    }
}
