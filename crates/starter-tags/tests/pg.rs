use serde_json::json;
use starter_tags::prelude::*;
use std::str::FromStr;

fn pg(q: &TagQuery) -> SqlFragment {
    compile_to_pg(
        q,
        PgCompileOptions {
            column: "tags",
            first_bind: 1,
        },
    )
}

#[test]
fn has_emits_containment_against_bool_true() {
    let f = pg(&TagQuery::from_str("sensor").unwrap());
    assert_eq!(f.sql, "(tags @> $1::jsonb)");
    assert_eq!(f.binds, vec![json!({"sensor": true})]);
}

#[test]
fn eq_string_uses_containment_only() {
    let f = pg(&TagQuery::from_str("equipRef:\"equip_abc\"").unwrap());
    assert_eq!(f.sql, "(tags @> $1::jsonb)");
    assert_eq!(f.binds, vec![json!({"equipRef": "equip_abc"})]);
}

#[test]
fn no_jsonb_extract_or_array_ops_anywhere() {
    let q = TagQuery::from_str("a and b or not c").unwrap();
    let f = pg(&q);
    for forbidden in ["->>", "->", "jsonb_path", "?|", "?&", "@?", "@@"] {
        assert!(
            !f.sql.contains(forbidden),
            "T8a violation: PG fragment contains forbidden operator {forbidden:?}: {}",
            f.sql
        );
    }
    // exactly one containment per leaf (3 leaves → 3 $-binds)
    assert_eq!(f.binds.len(), 3);
}

#[test]
fn and_or_not_shape() {
    let q = TagQuery::from_str("a and (b or not c)").unwrap();
    let f = pg(&q);
    assert!(f.sql.contains(" AND "));
    assert!(f.sql.contains(" OR "));
    assert!(f.sql.contains("NOT "));
}
