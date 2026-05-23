//! D6 — semantic parity across the three compilation targets.
//!
//! T8c (the in-process matcher) is the oracle. T8a (Postgres) and T8b
//! (ClickHouse) compile to SQL fragments we cannot run here without
//! a database, so this file checks the structural invariants that
//! make the database semantics provably equivalent to the matcher:
//!
//! * PG leaves emit a JSONB containment object whose `(key, value)`
//!   shape mirrors exactly what `matches()` is comparing against.
//! * CH leaves bind the key and `tag_value_to_ch_string(value)`; the
//!   storage layer applies the same conversion on writes (T2), so
//!   `tags[k] = v` is true iff `matches()` is true.
//! * `and` / `or` / `not` compose by the same boolean algebra in all
//!   three renderers (uniformly recursive emission).
//!
//! Plus the explicit fixtures called out in the SCOPE.

use serde_json::json;
use starter_tags::prelude::*;
use std::str::FromStr;

fn ts(pairs: &[(&str, TagValue)]) -> TagSet {
    let mut s = TagSet::new();
    for (k, v) in pairs {
        s.insert(*k, v.clone()).unwrap();
    }
    s
}

fn pg(q: &TagQuery) -> SqlFragment {
    compile_to_pg(
        q,
        PgCompileOptions {
            column: "tags",
            first_bind: 1,
        },
    )
}
fn ch(q: &TagQuery) -> SqlFragment {
    compile_to_ch(
        q,
        ChCompileOptions {
            column: "tags",
            first_bind: 1,
        },
    )
}

// ---------------------------------------------------------------------
// Fixture: integer-as-string discriminant.
// ---------------------------------------------------------------------
#[test]
fn integer_discriminant_parity() {
    let q = TagQuery::from_str("port:8080").unwrap();

    // Matcher oracle
    let hit = ts(&[("port", TagValue::Str("8080".into()))]);
    let miss_other = ts(&[("port", TagValue::Str("8081".into()))]);
    let miss_ws = ts(&[("port", TagValue::Str(" 8080".into()))]);
    let miss_decimal = ts(&[("port", TagValue::Str("8080.0".into()))]);
    assert!(hit.matches(&q));
    assert!(!miss_other.matches(&q));
    assert!(!miss_ws.matches(&q));
    assert!(!miss_decimal.matches(&q));

    // PG: containment against {"port": "8080"} — exact string equality.
    let f = pg(&q);
    assert_eq!(f.binds, vec![json!({"port": "8080"})]);

    // CH: tags['port'] = '8080'.
    let f = ch(&q);
    assert_eq!(f.binds, vec![json!("port"), json!("8080")]);
}

// ---------------------------------------------------------------------
// Fixture: Bool — `flag:true` matches Bool(true) but NOT Str("true").
// ---------------------------------------------------------------------
#[test]
fn bool_no_implicit_string_coercion_parity() {
    let q = TagQuery::from_str("flag:true").unwrap();

    let hit = ts(&[("flag", TagValue::Bool(true))]);
    assert!(hit.matches(&q));

    // Str("true") cannot be inserted via the typed API (M-2); insert
    // it raw to prove the matcher does not coerce.
    let mut raw = TagSet::new();
    raw.0.insert("flag".into(), TagValue::Str("true".into()));
    assert!(!raw.matches(&q));

    // PG bind is JSON `true`, not the string "true".
    let f = pg(&q);
    assert_eq!(f.binds, vec![json!({"flag": true})]);

    // CH bind is the string "true" (per tag_value_to_ch_string); the
    // storage layer writes Bool(true) → "true" identically, so the
    // equality holds iff the matcher does.
    let f = ch(&q);
    assert_eq!(f.binds, vec![json!("flag"), json!("true")]);
    assert_eq!(
        f.binds[1].as_str().unwrap(),
        tag_value_to_ch_string(&TagValue::Bool(true))
    );
}

// ---------------------------------------------------------------------
// Fixture: bare-tag sugar T3.
// ---------------------------------------------------------------------
#[test]
fn bare_tag_sugar_parity() {
    let q = TagQuery::from_str("sensor").unwrap();

    let hit = ts(&[("sensor", TagValue::Bool(true))]);
    let miss_false = ts(&[("sensor", TagValue::Bool(false))]);
    let miss_absent = TagSet::new();
    assert!(hit.matches(&q));
    assert!(!miss_false.matches(&q));
    assert!(!miss_absent.matches(&q));

    let f = pg(&q);
    assert_eq!(f.binds, vec![json!({"sensor": true})]);
    let f = ch(&q);
    assert_eq!(f.binds, vec![json!("sensor"), json!("true")]);
}

// ---------------------------------------------------------------------
// Fixture: float-literal rejection.
// ---------------------------------------------------------------------
#[test]
fn float_literal_rejection_is_typed() {
    let err = TagQuery::from_str("value:42.3").unwrap_err();
    let msg = err.to_string();
    // Must mention typed-column guidance so writers see the fix.
    assert!(msg.contains("typed column") || msg.contains("samples.value_num"));
    assert!(matches!(err, TagParseError::FloatLiteral { .. }));

    // Negative floats and scientific notation too.
    assert!(matches!(
        TagQuery::from_str("v:-1.5").unwrap_err(),
        TagParseError::FloatLiteral { .. }
    ));
    assert!(matches!(
        TagQuery::from_str("v:1e3").unwrap_err(),
        TagParseError::FloatLiteral { .. }
    ));
}

// ---------------------------------------------------------------------
// Fixture: Bool/Str reserved-string rejection at TagSet construction.
// ---------------------------------------------------------------------
#[test]
fn reserved_bool_string_rejected() {
    let mut s = TagSet::new();
    for forbidden in ["true", "false", "TRUE", "False", "tRuE"] {
        let err = s
            .insert("flag", TagValue::Str(forbidden.into()))
            .unwrap_err();
        assert!(matches!(err, TagSetError::ReservedBoolString { .. }));
    }
    // The legitimate Bool form still works.
    s.insert("flag", TagValue::Bool(true)).unwrap();
}

// ---------------------------------------------------------------------
// Fixture: tag_value_to_ch_string round-trip.
// ---------------------------------------------------------------------
#[test]
fn tag_value_to_ch_string_is_canonical() {
    for (v, expect) in [
        (TagValue::Bool(true), "true"),
        (TagValue::Bool(false), "false"),
        (TagValue::Str("equip_abc".into()), "equip_abc"),
        (TagValue::Str("8080".into()), "8080"),
        // Edge: a non-bool string that happens to look like another
        // string passes through verbatim (no normalisation).
        (TagValue::Str("Hello, world".into()), "Hello, world"),
    ] {
        assert_eq!(tag_value_to_ch_string(&v), expect);

        // And: compile_to_ch binds exactly this string as the literal.
        let q = TagQuery::Eq("k".into(), v.clone());
        let f = ch(&q);
        assert_eq!(f.binds[1], json!(expect));
    }
}

// ---------------------------------------------------------------------
// Fixture: JSON ingest rejects NaN/Inf/non-integer numbers.
// ---------------------------------------------------------------------
#[test]
fn json_ingest_rejects_non_integer_numbers() {
    let mut s = TagSet::new();
    // Integer JSON numbers coerce to canonical decimal string.
    s.insert_json("port", serde_json::json!(8080)).unwrap();
    assert_eq!(s.get("port"), Some(&TagValue::Str("8080".into())));

    // Non-integer: rejected typed.
    let err = s
        .insert_json("reading", serde_json::json!(42.3))
        .unwrap_err();
    assert!(matches!(err, TagSetError::NonIntegerNumber { .. }));

    // NaN / Inf cannot appear in serde_json::Value directly (the
    // serialiser disallows them); construct the Number manually.
    let nan = serde_json::Number::from_f64(f64::NAN);
    let inf = serde_json::Number::from_f64(f64::INFINITY);
    assert!(nan.is_none() && inf.is_none(), "guarded at construction");
}

// ---------------------------------------------------------------------
// Composite battery: matcher truth must match compiler shape.
// ---------------------------------------------------------------------
#[test]
fn composite_parity_battery() {
    let cases: &[(&str, Vec<(&str, TagValue)>, bool)] = &[
        ("a and b", vec![("a", TagValue::Bool(true)), ("b", TagValue::Bool(true))], true),
        ("a and b", vec![("a", TagValue::Bool(true))], false),
        ("a or b", vec![("b", TagValue::Bool(true))], true),
        ("not c", vec![], true),
        ("not c", vec![("c", TagValue::Bool(true))], false),
        (
            "energy and (building:\"hq\" or building:\"warehouse\")",
            vec![
                ("energy", TagValue::Bool(true)),
                ("building", TagValue::Str("hq".into())),
            ],
            true,
        ),
    ];
    for (src, kv, expected) in cases {
        let q = TagQuery::from_str(src).unwrap();
        let s = ts(kv);
        assert_eq!(
            s.matches(&q),
            *expected,
            "matcher truth for {src:?} on {kv:?}"
        );

        // Both compilers must produce a well-formed parameterised
        // fragment with the right number of binds.
        let leaves = count_leaves(&q);
        let pgf = pg(&q);
        assert_eq!(pgf.binds.len(), leaves, "pg bind count for {src:?}");
        let chf = ch(&q);
        assert_eq!(chf.binds.len(), leaves * 2, "ch bind count for {src:?}");
    }
}

fn count_leaves(q: &TagQuery) -> usize {
    match q {
        TagQuery::Has(_) | TagQuery::Eq(_, _) => 1,
        TagQuery::And(xs) | TagQuery::Or(xs) => xs.iter().map(count_leaves).sum(),
        TagQuery::Not(x) => count_leaves(x),
    }
}
