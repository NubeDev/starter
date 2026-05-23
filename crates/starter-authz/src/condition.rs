//! Tiny condition expression language for rule `condition` strings.
//!
//! Grammar (deliberately small — SCOPE.md R8: "anything more goes
//! through a custom `PolicyEngine` impl"):
//!
//! ```text
//! expr      := or_expr
//! or_expr   := and_expr ('or' and_expr)*
//! and_expr  := unary    ('and' unary)*
//! unary     := 'not' unary | atom
//! atom      := '(' expr ')'
//!            | path op value
//!            | path 'in' list
//!            | path 'contains' value      ; Phase 7b — R13
//!            | path                        ; truthy test
//! op        := '==' | '!='
//! value     := string | bool | path
//! list      := '[' value (',' value)* ']'
//! path      := IDENT ('.' IDENT)*    ; e.g. oauth.email_domain
//! string    := '"' ... '"'
//! bool      := 'true' | 'false'
//! ```
//!
//! `contains` is the Phase 7b array-membership operator: the LHS
//! must resolve to a JSON array (typically `principal.teams`) and
//! the RHS is a literal or another path. Loud failure (parallel
//! to R8's missing-attribute shape, not R3's silent deny): when
//! the LHS resolves to a value that is not an array, the
//! evaluator returns a `ContainsLhsNotArray` error so the engine
//! can surface a typed deny reason rather than silently return
//! `false`. A missing LHS is treated as an empty array (no team
//! membership → no match), matching the additive-by-default
//! Phase 7b contract.
//!
//! Paths are resolved against a [`Context`] carrying the principal
//! attributes (typically the `Principal.extra` JSON blob, with
//! `subject`, `role` and the owner shim added at the root).
//! Missing attributes are treated as not-equal — never as true
//! (SCOPE.md "oauth-attributes-drive-a-rule" smoke test).

use std::collections::BTreeMap;

use serde_json::Value;

use crate::error::{Error, Result};

/// Variables visible to a condition. Constructed by the engine
/// from the [`starter_spi::auth::Principal`] and the
/// [`starter_spi::authz::ResourceRef`] before each `check`.
#[derive(Debug, Clone, Default)]
pub struct Context {
    /// Root JSON object. Lookups traverse `.`-separated paths into
    /// this value.
    pub vars: Value,
}

impl Context {
    /// Build a context from a JSON object map. Convenience for
    /// tests.
    pub fn from_map(map: BTreeMap<String, Value>) -> Self {
        Self {
            vars: Value::Object(map.into_iter().collect()),
        }
    }

    fn resolve(&self, path: &[&str]) -> Option<&Value> {
        let mut cur = &self.vars;
        for part in path {
            match cur {
                Value::Object(o) => match o.get(*part) {
                    Some(v) => cur = v,
                    None => return None,
                },
                _ => return None,
            }
        }
        Some(cur)
    }
}

/// Parsed expression. Cloned cheaply onto every rule at load time
/// so per-`check` evaluation does not re-parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// Constant boolean.
    Lit(bool),
    /// `path` (truthy test).
    Truthy(Vec<String>),
    /// `path == value` / `path != value`.
    Cmp {
        /// Left-hand path resolved against the [`Context`].
        path: Vec<String>,
        /// Comparison operator.
        op: CmpOp,
        /// Right-hand operand (literal or another path).
        rhs: Operand,
    },
    /// `path in [v1, v2, ...]`.
    In {
        /// Left-hand path resolved against the [`Context`].
        path: Vec<String>,
        /// Candidate values; membership is by equality.
        values: Vec<Operand>,
    },
    /// Phase 7b — `path contains value`. The LHS must resolve to
    /// a JSON array; otherwise [`Expr::try_eval`] returns
    /// [`EvalError::ContainsLhsNotArray`]. RHS may be a literal
    /// or another path.
    Contains {
        /// Left-hand path; must resolve to a JSON array at eval
        /// time.
        path: Vec<String>,
        /// Value the array must contain (literal or path).
        rhs: Operand,
    },
    /// `a and b`.
    And(Box<Expr>, Box<Expr>),
    /// `a or b`.
    Or(Box<Expr>, Box<Expr>),
    /// `not a`.
    Not(Box<Expr>),
}

/// Comparison operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    /// `==`
    Eq,
    /// `!=`
    Neq,
}

/// Right-hand side of a comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    /// String literal.
    Str(String),
    /// Boolean literal.
    Bool(bool),
    /// Path reference — resolved against the [`Context`] at eval
    /// time. Lets rules say e.g. `subject == object.owner`.
    Path(Vec<String>),
}

impl Expr {
    /// Parse a condition string.
    pub fn parse(s: &str) -> Result<Self> {
        let tokens = tokenize(s).map_err(|reason| Error::Condition {
            expr: s.to_string(),
            reason,
        })?;
        let mut p = Parser {
            toks: &tokens,
            i: 0,
        };
        let expr = p.parse_or().map_err(|reason| Error::Condition {
            expr: s.to_string(),
            reason,
        })?;
        if p.i != tokens.len() {
            return Err(Error::Condition {
                expr: s.to_string(),
                reason: format!("trailing tokens at position {}", p.i),
            });
        }
        Ok(expr)
    }

    /// Evaluate against a context. Returns `false` on any missing
    /// attribute or type mismatch (deliberate: missing != true).
    ///
    /// `contains` type-errors are mapped to `false` here for
    /// backward source-compatibility — call [`Expr::try_eval`]
    /// when you want the typed error surfaced (the engine does).
    pub fn eval(&self, ctx: &Context) -> bool {
        self.try_eval(ctx).unwrap_or(false)
    }

    /// Phase 7b — typed evaluator. Returns
    /// [`EvalError::ContainsLhsNotArray`] when a `contains`
    /// expression's LHS resolves to a value that is not a JSON
    /// array (the spec's "loud failure" path, parallel to R8's
    /// missing-attribute shape). All other failure modes (missing
    /// path, type mismatch on `==` / `!=`, etc.) still surface as
    /// `Ok(false)` so consumers get the existing "missing != true"
    /// semantics.
    pub fn try_eval(&self, ctx: &Context) -> std::result::Result<bool, EvalError> {
        Ok(match self {
            Expr::Lit(b) => *b,
            Expr::Truthy(path) => match ctx.resolve(&path_refs(path)) {
                Some(Value::Bool(b)) => *b,
                Some(Value::Null) | None => false,
                Some(Value::String(s)) => !s.is_empty(),
                Some(Value::Number(n)) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
                Some(Value::Array(a)) => !a.is_empty(),
                Some(Value::Object(o)) => !o.is_empty(),
            },
            Expr::Cmp { path, op, rhs } => {
                let lhs = ctx.resolve(&path_refs(path));
                let rhs_v = resolve_operand(rhs, ctx);
                let eq = match (lhs, rhs_v.as_ref()) {
                    (Some(a), Some(b)) => json_equal(a, b),
                    _ => false,
                };
                match op {
                    CmpOp::Eq => eq,
                    CmpOp::Neq => !eq && lhs.is_some() && rhs_v.is_some(),
                }
            }
            Expr::In { path, values } => {
                let Some(lhs) = ctx.resolve(&path_refs(path)) else {
                    return Ok(false);
                };
                values.iter().any(|v| {
                    resolve_operand(v, ctx)
                        .as_ref()
                        .map(|rv| json_equal(lhs, rv))
                        .unwrap_or(false)
                })
            }
            Expr::Contains { path, rhs } => {
                let lhs = ctx.resolve(&path_refs(path));
                let Some(rhs_v) = resolve_operand(rhs, ctx) else {
                    return Ok(false);
                };
                match lhs {
                    // Missing LHS is treated as empty array: no
                    // match, no error. The additive-by-default
                    // Phase 7b contract says a principal that has
                    // never been wired with `.teams` simply
                    // doesn't match team rules.
                    None => false,
                    Some(Value::Array(items)) => {
                        items.iter().any(|item| json_equal(item, &rhs_v))
                    }
                    Some(other) => {
                        return Err(EvalError::ContainsLhsNotArray {
                            path: path.join("."),
                            actual_type: type_name(other),
                        });
                    }
                }
            }
            Expr::And(a, b) => a.try_eval(ctx)? && b.try_eval(ctx)?,
            Expr::Or(a, b) => a.try_eval(ctx)? || b.try_eval(ctx)?,
            Expr::Not(a) => !a.try_eval(ctx)?,
        })
    }
}

/// Typed evaluation errors raised by [`Expr::try_eval`]. The
/// engine maps these to a deny with a stable reason code.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum EvalError {
    /// `contains` LHS resolved to a value that is not a JSON
    /// array. The rule is malformed for the data shape it was
    /// asked to evaluate against — surfaced as a typed deny
    /// rather than a silent `false`.
    #[error(
        "`contains` left-hand path `{path}` resolved to {actual_type}, expected an array"
    )]
    ContainsLhsNotArray {
        /// The dotted path the rule referenced.
        path: String,
        /// JSON type name of what it actually resolved to.
        actual_type: &'static str,
    },
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn path_refs(p: &[String]) -> Vec<&str> {
    p.iter().map(|s| s.as_str()).collect()
}

fn resolve_operand(op: &Operand, ctx: &Context) -> Option<Value> {
    match op {
        Operand::Str(s) => Some(Value::String(s.clone())),
        Operand::Bool(b) => Some(Value::Bool(*b)),
        Operand::Path(p) => ctx.resolve(&path_refs(p)).cloned(),
    }
}

fn json_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::Null, Value::Null) => true,
        _ => a == b,
    }
}

// --------------------------------------------------------------- lexer

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Ident(String),
    Str(String),
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Dot,
    EqEq,
    NotEq,
    And,
    Or,
    Not,
    In,
    Contains,
    True,
    False,
}

fn tokenize(src: &str) -> std::result::Result<Vec<Tok>, String> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b' ' | b'\t' | b'\n' | b'\r' => {
                i += 1;
            }
            b'(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            b')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            b'[' => {
                out.push(Tok::LBracket);
                i += 1;
            }
            b']' => {
                out.push(Tok::RBracket);
                i += 1;
            }
            b',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            b'.' => {
                out.push(Tok::Dot);
                i += 1;
            }
            b'=' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    out.push(Tok::EqEq);
                    i += 2;
                } else {
                    return Err(format!("unexpected `=` at {i}; did you mean `==`?"));
                }
            }
            b'!' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    out.push(Tok::NotEq);
                    i += 2;
                } else {
                    return Err(format!("unexpected `!` at {i}; did you mean `!=`?"));
                }
            }
            b'"' | b'\'' => {
                let quote = c;
                let start = i + 1;
                let mut j = start;
                while j < bytes.len() && bytes[j] != quote {
                    j += 1;
                }
                if j >= bytes.len() {
                    return Err(format!("unterminated string starting at {i}"));
                }
                let s = std::str::from_utf8(&bytes[start..j])
                    .map_err(|_| format!("non-utf8 string at {i}"))?
                    .to_string();
                out.push(Tok::Str(s));
                i = j + 1;
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let ident = std::str::from_utf8(&bytes[start..i])
                    .map_err(|_| format!("non-utf8 ident at {start}"))?;
                out.push(match ident {
                    "and" => Tok::And,
                    "or" => Tok::Or,
                    "not" => Tok::Not,
                    "in" => Tok::In,
                    "contains" => Tok::Contains,
                    "true" => Tok::True,
                    "false" => Tok::False,
                    _ => Tok::Ident(ident.to_string()),
                });
            }
            other => {
                return Err(format!(
                    "unexpected character `{}` at position {i}",
                    other as char
                ));
            }
        }
    }
    Ok(out)
}

// --------------------------------------------------------------- parser

struct Parser<'a> {
    toks: &'a [Tok],
    i: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.i)
    }

    fn bump(&mut self) -> Option<&Tok> {
        let t = self.toks.get(self.i);
        if t.is_some() {
            self.i += 1;
        }
        t
    }

    fn eat(&mut self, want: &Tok) -> bool {
        if self.peek() == Some(want) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn parse_or(&mut self) -> std::result::Result<Expr, String> {
        let mut lhs = self.parse_and()?;
        while self.eat(&Tok::Or) {
            let rhs = self.parse_and()?;
            lhs = Expr::Or(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> std::result::Result<Expr, String> {
        let mut lhs = self.parse_unary()?;
        while self.eat(&Tok::And) {
            let rhs = self.parse_unary()?;
            lhs = Expr::And(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> std::result::Result<Expr, String> {
        if self.eat(&Tok::Not) {
            let inner = self.parse_unary()?;
            return Ok(Expr::Not(Box::new(inner)));
        }
        self.parse_atom()
    }

    fn parse_atom(&mut self) -> std::result::Result<Expr, String> {
        match self.peek().cloned() {
            Some(Tok::LParen) => {
                self.bump();
                let e = self.parse_or()?;
                if !self.eat(&Tok::RParen) {
                    return Err("missing `)`".into());
                }
                Ok(e)
            }
            Some(Tok::True) => {
                self.bump();
                Ok(Expr::Lit(true))
            }
            Some(Tok::False) => {
                self.bump();
                Ok(Expr::Lit(false))
            }
            Some(Tok::Ident(_)) => {
                let path = self.parse_path()?;
                match self.peek() {
                    Some(Tok::EqEq) => {
                        self.bump();
                        let rhs = self.parse_operand()?;
                        Ok(Expr::Cmp {
                            path,
                            op: CmpOp::Eq,
                            rhs,
                        })
                    }
                    Some(Tok::NotEq) => {
                        self.bump();
                        let rhs = self.parse_operand()?;
                        Ok(Expr::Cmp {
                            path,
                            op: CmpOp::Neq,
                            rhs,
                        })
                    }
                    Some(Tok::Contains) => {
                        self.bump();
                        let rhs = self.parse_operand()?;
                        Ok(Expr::Contains { path, rhs })
                    }
                    Some(Tok::In) => {
                        self.bump();
                        if !self.eat(&Tok::LBracket) {
                            return Err("expected `[` after `in`".into());
                        }
                        let mut values = Vec::new();
                        if !self.eat(&Tok::RBracket) {
                            loop {
                                values.push(self.parse_operand()?);
                                if self.eat(&Tok::RBracket) {
                                    break;
                                }
                                if !self.eat(&Tok::Comma) {
                                    return Err("expected `,` or `]` in list".into());
                                }
                            }
                        }
                        Ok(Expr::In { path, values })
                    }
                    _ => Ok(Expr::Truthy(path)),
                }
            }
            other => Err(format!("unexpected token: {other:?}")),
        }
    }

    fn parse_path(&mut self) -> std::result::Result<Vec<String>, String> {
        let mut parts = Vec::new();
        match self.bump() {
            Some(Tok::Ident(s)) => parts.push(s.clone()),
            other => return Err(format!("expected identifier, got {other:?}")),
        }
        while self.eat(&Tok::Dot) {
            match self.bump() {
                Some(Tok::Ident(s)) => parts.push(s.clone()),
                other => return Err(format!("expected identifier after `.`, got {other:?}")),
            }
        }
        Ok(parts)
    }

    fn parse_operand(&mut self) -> std::result::Result<Operand, String> {
        match self.peek().cloned() {
            Some(Tok::Str(s)) => {
                self.bump();
                Ok(Operand::Str(s))
            }
            Some(Tok::True) => {
                self.bump();
                Ok(Operand::Bool(true))
            }
            Some(Tok::False) => {
                self.bump();
                Ok(Operand::Bool(false))
            }
            Some(Tok::Ident(_)) => {
                let path = self.parse_path()?;
                Ok(Operand::Path(path))
            }
            other => Err(format!("expected operand, got {other:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(json: serde_json::Value) -> Context {
        Context { vars: json }
    }

    #[test]
    fn equality() {
        let e = Expr::parse(r#"oauth.email_domain == "acme.com""#).unwrap();
        let c = ctx(serde_json::json!({"oauth": {"email_domain": "acme.com"}}));
        assert!(e.eval(&c));
        let c = ctx(serde_json::json!({"oauth": {"email_domain": "other.com"}}));
        assert!(!e.eval(&c));
    }

    #[test]
    fn missing_attr_is_not_equal() {
        let e = Expr::parse(r#"oauth.email_domain == "acme.com""#).unwrap();
        let c = ctx(serde_json::json!({}));
        assert!(!e.eval(&c));
    }

    #[test]
    fn and_or_not() {
        let e =
            Expr::parse(r#"oauth.email_domain == "acme.com" and oauth.email_verified"#).unwrap();
        let c =
            ctx(serde_json::json!({"oauth": {"email_domain": "acme.com", "email_verified": true}}));
        assert!(e.eval(&c));
        let c = ctx(
            serde_json::json!({"oauth": {"email_domain": "acme.com", "email_verified": false}}),
        );
        assert!(!e.eval(&c));

        let e = Expr::parse(r#"not (oauth.email_verified)"#).unwrap();
        let c = ctx(serde_json::json!({"oauth": {"email_verified": false}}));
        assert!(e.eval(&c));
    }

    #[test]
    fn membership() {
        let e = Expr::parse(r#"oauth.provider in ["google", "github"]"#).unwrap();
        let c = ctx(serde_json::json!({"oauth": {"provider": "github"}}));
        assert!(e.eval(&c));
        let c = ctx(serde_json::json!({"oauth": {"provider": "facebook"}}));
        assert!(!e.eval(&c));
    }

    #[test]
    fn path_to_path_equality() {
        let e = Expr::parse(r#"subject == object.owner"#).unwrap();
        let c = ctx(serde_json::json!({"subject": "alice", "object": {"owner": "alice"}}));
        assert!(e.eval(&c));
        let c = ctx(serde_json::json!({"subject": "bob", "object": {"owner": "alice"}}));
        assert!(!e.eval(&c));
    }

    #[test]
    fn unterminated_string_errors() {
        let err = Expr::parse(r#"a == "x"#).unwrap_err();
        match err {
            Error::Condition { .. } => {}
            other => panic!("expected Condition error, got {other:?}"),
        }
    }
}
