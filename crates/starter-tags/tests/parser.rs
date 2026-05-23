use starter_tags::prelude::*;
use std::str::FromStr;

#[test]
fn bare_tag() {
    let q = TagQuery::from_str("sensor").unwrap();
    assert_eq!(q, TagQuery::Has("sensor".into()));
}

#[test]
fn eq_string() {
    let q = TagQuery::from_str("equipRef:\"equip_abc\"").unwrap();
    assert_eq!(
        q,
        TagQuery::Eq("equipRef".into(), TagValue::Str("equip_abc".into()))
    );
}

#[test]
fn eq_integer_compiles_to_str() {
    let q = TagQuery::from_str("port:8080").unwrap();
    assert_eq!(q, TagQuery::Eq("port".into(), TagValue::Str("8080".into())));
}

#[test]
fn eq_bool() {
    let q = TagQuery::from_str("flag:true").unwrap();
    assert_eq!(q, TagQuery::Eq("flag".into(), TagValue::Bool(true)));
}

#[test]
fn and_or_not_precedence() {
    let q = TagQuery::from_str("a and b or not c").unwrap();
    // 'and' binds tighter than 'or'; 'not' applies to its atom
    assert_eq!(
        q,
        TagQuery::Or(vec![
            TagQuery::And(vec![TagQuery::Has("a".into()), TagQuery::Has("b".into()),]),
            TagQuery::Not(Box::new(TagQuery::Has("c".into()))),
        ])
    );
}

#[test]
fn parenthesised() {
    let q = TagQuery::from_str("energy and (building:\"hq\" or building:\"warehouse\")").unwrap();
    if let TagQuery::And(xs) = &q {
        assert_eq!(xs.len(), 2);
    } else {
        panic!("expected And, got {q:?}");
    }
}

#[test]
fn dotted_key() {
    let q = TagQuery::from_str("energy.subkind:\"hvac\"").unwrap();
    assert_eq!(
        q,
        TagQuery::Eq("energy.subkind".into(), TagValue::Str("hvac".into()))
    );
}

#[test]
fn float_literal_rejected_with_typed_error() {
    let err = TagQuery::from_str("value:42.3").unwrap_err();
    match err {
        TagParseError::FloatLiteral { literal } => assert_eq!(literal, "42.3"),
        other => panic!("expected FloatLiteral, got {other:?}"),
    }
}

#[test]
fn empty_query_rejected() {
    assert!(matches!(
        TagQuery::from_str("   ").unwrap_err(),
        TagParseError::Empty
    ));
}

#[test]
fn round_trip_display_parse() {
    for src in [
        "sensor",
        "equipRef:\"equip_abc\"",
        "port:8080",
        "flag:true",
        "flag:false",
        "a and b",
        "a or b",
        "not a",
        "a and (b or c)",
    ] {
        let q = TagQuery::from_str(src).unwrap();
        let rendered = q.to_string();
        let q2 = TagQuery::from_str(&rendered).unwrap_or_else(|e| {
            panic!("re-parse {rendered:?} failed: {e}");
        });
        assert_eq!(q, q2, "round-trip mismatch from {src:?}");
    }
}
