use serde_json::json;
use starter_tags::prelude::*;
use std::str::FromStr;

fn ch(q: &TagQuery) -> SqlFragment {
    compile_to_ch(
        q,
        ChCompileOptions {
            column: "tags",
            first_bind: 1,
        },
    )
}

#[test]
fn has_emits_map_equality_against_true_string() {
    let f = ch(&TagQuery::from_str("sensor").unwrap());
    assert_eq!(f.sql, "(tags[$1] = $2)");
    assert_eq!(f.binds, vec![json!("sensor"), json!("true")]);
}

#[test]
fn eq_str_uses_map_equality_only() {
    let f = ch(&TagQuery::from_str("port:8080").unwrap());
    assert_eq!(f.sql, "(tags[$1] = $2)");
    assert_eq!(f.binds, vec![json!("port"), json!("8080")]);
}

#[test]
fn no_json_extract_no_like_no_mapcontains() {
    let q = TagQuery::from_str("a and b or not c").unwrap();
    let f = ch(&q);
    for forbidden in ["JSONExtract", "LIKE", "ILIKE", "mapContains", "tagsValue"] {
        assert!(
            !f.sql.contains(forbidden),
            "T8b violation: CH fragment contains forbidden operator {forbidden:?}: {}",
            f.sql
        );
    }
    // 3 leaves × 2 binds (key + value) each = 6
    assert_eq!(f.binds.len(), 6);
}

#[test]
fn tag_value_to_ch_string_matches_bind() {
    let q = TagQuery::Eq("flag".into(), TagValue::Bool(true));
    let f = ch(&q);
    assert_eq!(
        f.binds[1],
        json!(tag_value_to_ch_string(&TagValue::Bool(true)))
    );
}
