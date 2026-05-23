//! `TagQuery` AST + parser (T7).
//!
//! Grammar (recap from DOCS/Tags/SCOPE.md):
//!
//! ```text
//! expr   := or
//! or     := and ( 'or' and )*
//! and    := not ( 'and' not )*
//! not    := 'not' not | atom
//! atom   := key | key ':' literal | '(' expr ')'
//! literal:= STRING | INTEGER | 'true' | 'false'
//! key    := IDENT ( '.' IDENT )*
//! STRING := double-quoted
//! ```

use std::fmt;
use std::str::FromStr;

use nom::{
    branch::alt,
    bytes::complete::{tag as ntag, take_while1},
    character::complete::{char as nchar, multispace0, satisfy},
    combinator::{map, opt, recognize, value as nvalue, verify},
    error::{Error as NomError, ErrorKind},
    multi::many0,
    sequence::{delimited, pair, preceded},
    Err as NomErr, IResult,
};
use serde::{Deserialize, Serialize};

use crate::error::TagParseError;
use crate::set::TagValue;

/// The query AST. `Has(k)` is the bare-tag sugar (T3) and is treated as
/// `Eq(k, Bool(true))` by every compiler.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TagQuery {
    Has(String),
    Eq(String, TagValue),
    And(Vec<TagQuery>),
    Or(Vec<TagQuery>),
    Not(Box<TagQuery>),
}

impl FromStr for TagQuery {
    type Err = TagParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim().is_empty() {
            return Err(TagParseError::Empty);
        }
        // Pre-scan for float literals so we get a typed error rather
        // than a generic syntax failure (T2 / D6).
        if let Some(lit) = find_float_literal(s) {
            return Err(TagParseError::FloatLiteral { literal: lit });
        }
        match parse_expr(s) {
            Ok((rest, q)) => {
                let rest = rest.trim_start();
                if !rest.is_empty() {
                    return Err(TagParseError::Trailing {
                        offset: s.len() - rest.len(),
                        tail: rest.chars().take(32).collect(),
                    });
                }
                Ok(q)
            }
            Err(NomErr::Error(e) | NomErr::Failure(e)) => Err(TagParseError::Syntax {
                near: e.input.chars().take(32).collect(),
            }),
            Err(NomErr::Incomplete(_)) => Err(TagParseError::Syntax {
                near: String::new(),
            }),
        }
    }
}

impl fmt::Display for TagQuery {
    /// Canonical rendering (round-trips through `FromStr`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TagQuery::Has(k) => f.write_str(k),
            TagQuery::Eq(k, v) => match v {
                TagValue::Bool(true) => write!(f, "{k}:true"),
                TagValue::Bool(false) => write!(f, "{k}:false"),
                TagValue::Str(s) => write!(f, "{k}:{}", quote_string(s)),
            },
            TagQuery::And(xs) => write_join(f, xs, " and "),
            TagQuery::Or(xs) => write_join(f, xs, " or "),
            TagQuery::Not(x) => write!(f, "not {}", paren_if_compound(x)),
        }
    }
}

fn write_join(f: &mut fmt::Formatter<'_>, xs: &[TagQuery], sep: &str) -> fmt::Result {
    for (i, x) in xs.iter().enumerate() {
        if i > 0 {
            f.write_str(sep)?;
        }
        write!(f, "{}", paren_if_compound(x))?;
    }
    Ok(())
}

fn paren_if_compound(q: &TagQuery) -> String {
    match q {
        TagQuery::And(_) | TagQuery::Or(_) | TagQuery::Not(_) => format!("({q})"),
        _ => q.to_string(),
    }
}

fn quote_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// ----- parser -----------------------------------------------------------

fn kw<'a>(k: &'static str) -> impl FnMut(&'a str) -> IResult<&'a str, ()> {
    move |i: &'a str| {
        let (i, _) = multispace0(i)?;
        let (i, _) = ntag(k)(i)?;
        // ensure not a prefix of a longer identifier
        if let Some(c) = i.chars().next() {
            if is_ident_cont(c) {
                return Err(NomErr::Error(NomError::new(i, ErrorKind::Tag)));
            }
        }
        let (i, _) = multispace0(i)?;
        Ok((i, ()))
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}
fn is_ident_cont(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn ident(i: &str) -> IResult<&str, &str> {
    recognize(pair(
        satisfy(is_ident_start),
        take_while1_or_empty(is_ident_cont),
    ))(i)
}

fn take_while1_or_empty<F>(pred: F) -> impl Fn(&str) -> IResult<&str, &str>
where
    F: Fn(char) -> bool,
{
    move |i: &str| {
        let end = i.find(|c| !pred(c)).unwrap_or(i.len());
        Ok((&i[end..], &i[..end]))
    }
}

fn dotted_key(i: &str) -> IResult<&str, String> {
    let (i, head) = ident(i)?;
    let (i, tail) = many0(preceded(nchar('.'), ident))(i)?;
    let mut s = String::from(head);
    for t in tail {
        s.push('.');
        s.push_str(t);
    }
    // reject reserved keywords as a bare key
    if matches!(s.as_str(), "and" | "or" | "not" | "true" | "false") {
        return Err(NomErr::Error(NomError::new(i, ErrorKind::Tag)));
    }
    Ok((i, s))
}

fn string_literal(i: &str) -> IResult<&str, String> {
    let (i, _) = nchar('"')(i)?;
    let mut out = String::new();
    let mut rest = i;
    loop {
        match rest.chars().next() {
            None => return Err(NomErr::Error(NomError::new(rest, ErrorKind::Char))),
            Some('"') => {
                rest = &rest[1..];
                return Ok((rest, out));
            }
            Some('\\') => {
                let after = &rest[1..];
                let mut chars = after.chars();
                let esc = chars
                    .next()
                    .ok_or_else(|| NomErr::Error(NomError::new(rest, ErrorKind::Escaped)))?;
                match esc {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    c => out.push(c),
                }
                rest = &after[esc.len_utf8()..];
            }
            Some(c) => {
                out.push(c);
                rest = &rest[c.len_utf8()..];
            }
        }
    }
}

fn integer_literal(i: &str) -> IResult<&str, i64> {
    let (i2, s) = recognize(pair(
        opt(nchar('-')),
        verify(take_while1(|c: char| c.is_ascii_digit()), |s: &str| {
            !s.is_empty()
        }),
    ))(i)?;
    // Reject if followed by '.' or 'e'/'E' (float lookahead).
    if let Some(c) = i2.chars().next() {
        if c == '.' || c == 'e' || c == 'E' {
            return Err(NomErr::Error(NomError::new(i2, ErrorKind::Digit)));
        }
    }
    let v: i64 = s
        .parse()
        .map_err(|_| NomErr::Error(NomError::new(i, ErrorKind::Digit)))?;
    Ok((i2, v))
}

fn literal(i: &str) -> IResult<&str, TagValue> {
    alt((
        map(nvalue((), kw("true")), |_| TagValue::Bool(true)),
        map(nvalue((), kw("false")), |_| TagValue::Bool(false)),
        map(string_literal, TagValue::Str),
        map(integer_literal, |v| TagValue::Str(v.to_string())),
    ))(i)
}

fn atom(i: &str) -> IResult<&str, TagQuery> {
    let (i, _) = multispace0(i)?;
    if let Ok((i2, q)) = delimited(nchar('('), parse_expr, nchar(')'))(i) {
        let (i2, _) = multispace0(i2)?;
        return Ok((i2, q));
    }
    // key (':' literal)?
    let (i, key) = dotted_key(i)?;
    let (i, _) = multispace0(i)?;
    if let Ok((i2, _)) = nchar::<_, NomError<&str>>(':')(i) {
        let (i2, _) = multispace0(i2)?;
        let (i2, lit) = literal(i2)?;
        return Ok((i2, TagQuery::Eq(key, lit)));
    }
    Ok((i, TagQuery::Has(key)))
}

fn parse_not(i: &str) -> IResult<&str, TagQuery> {
    let (i, _) = multispace0(i)?;
    if let Ok((i2, _)) = kw("not")(i) {
        let (i2, inner) = parse_not(i2)?;
        return Ok((i2, TagQuery::Not(Box::new(inner))));
    }
    atom(i)
}

fn parse_and(i: &str) -> IResult<&str, TagQuery> {
    let (i, first) = parse_not(i)?;
    let (i, rest) = many0(preceded(kw("and"), parse_not))(i)?;
    if rest.is_empty() {
        Ok((i, first))
    } else {
        let mut xs = vec![first];
        xs.extend(rest);
        Ok((i, TagQuery::And(xs)))
    }
}

fn parse_expr(i: &str) -> IResult<&str, TagQuery> {
    let (i, first) = parse_and(i)?;
    let (i, rest) = many0(preceded(kw("or"), parse_and))(i)?;
    if rest.is_empty() {
        Ok((i, first))
    } else {
        let mut xs = vec![first];
        xs.extend(rest);
        Ok((i, TagQuery::Or(xs)))
    }
}

// Scan for `key:<float>` style literals so we surface a typed error.
fn find_float_literal(src: &str) -> Option<String> {
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '"' {
            // skip string
            i += 1;
            while i < bytes.len() && bytes[i] as char != '"' {
                if bytes[i] as char == '\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                i += 1;
            }
            i += 1;
            continue;
        }
        if c == ':' {
            // skip ws
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                j += 1;
            }
            let start = j;
            if j < bytes.len() && bytes[j] as char == '-' {
                j += 1;
            }
            let digits_start = j;
            while j < bytes.len() && (bytes[j] as char).is_ascii_digit() {
                j += 1;
            }
            if j > digits_start && j < bytes.len() && matches!(bytes[j] as char, '.' | 'e' | 'E') {
                // continue capturing
                let mut k = j + 1;
                while k < bytes.len() {
                    let ch = bytes[k] as char;
                    if ch.is_ascii_digit() || matches!(ch, '.' | 'e' | 'E' | '+' | '-') {
                        k += 1;
                    } else {
                        break;
                    }
                }
                return Some(src[start..k].to_owned());
            }
        }
        i += 1;
    }
    None
}
