use starter_tags::prelude::*;
use std::str::FromStr;

fn set_from(pairs: &[(&str, TagValue)]) -> TagSet {
    let mut s = TagSet::new();
    for (k, v) in pairs {
        s.insert(*k, v.clone()).unwrap();
    }
    s
}

#[test]
fn bare_tag_requires_bool_true() {
    let q = TagQuery::from_str("sensor").unwrap();
    assert!(set_from(&[("sensor", TagValue::Bool(true))]).matches(&q));
    assert!(!set_from(&[("sensor", TagValue::Bool(false))]).matches(&q));
    assert!(!TagSet::new().matches(&q));
}

#[test]
fn eq_int_str_exact() {
    let q = TagQuery::from_str("port:8080").unwrap();
    assert!(set_from(&[("port", TagValue::Str("8080".into()))]).matches(&q));
    assert!(!set_from(&[("port", TagValue::Str("8081".into()))]).matches(&q));
    assert!(!set_from(&[("port", TagValue::Str("8080.0".into()))]).matches(&q));
    assert!(!set_from(&[("port", TagValue::Str(" 8080".into()))]).matches(&q));
}

#[test]
fn eq_bool_no_implicit_str_coercion() {
    let q = TagQuery::from_str("flag:true").unwrap();
    assert!(set_from(&[("flag", TagValue::Bool(true))]).matches(&q));
    // Str("true") cannot be inserted in the first place (M-2); even
    // if it were, the matcher must not coerce. We bypass via raw map:
    let mut raw = TagSet::new();
    raw.0.insert("flag".into(), TagValue::Str("true".into()));
    assert!(!raw.matches(&q));
}

#[test]
fn and_or_not() {
    let q = TagQuery::from_str("a and (b or not c)").unwrap();
    let s = set_from(&[
        ("a", TagValue::Bool(true)),
        ("b", TagValue::Bool(true)),
    ]);
    assert!(s.matches(&q));
    let s2 = set_from(&[
        ("a", TagValue::Bool(true)),
        ("c", TagValue::Bool(true)),
    ]);
    assert!(!s2.matches(&q));
}

#[test]
fn compile_to_match_returns_fn() {
    let q = TagQuery::from_str("sensor").unwrap();
    let m = compile_to_match(&q);
    assert!(m(&set_from(&[("sensor", TagValue::Bool(true))])));
    assert!(!m(&TagSet::new()));
}
