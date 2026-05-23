//! Display→parse fixed-point. Parsing the canonical rendering of any
//! AST must yield an AST that re-renders to the same string.

use starter_tags::prelude::*;
use std::str::FromStr;

fn fp(src: &str) {
    let q = TagQuery::from_str(src).unwrap();
    let r1 = q.to_string();
    let q2 = TagQuery::from_str(&r1).unwrap();
    assert_eq!(q, q2, "AST changed on re-parse of {r1:?}");
    let r2 = q2.to_string();
    assert_eq!(r1, r2, "rendering not idempotent for {src:?}");
}

#[test]
fn fixed_points() {
    for src in [
        "sensor",
        "equipRef:\"equip_abc\"",
        "port:8080",
        "flag:true",
        "flag:false",
        "a and b and c",
        "a or b or c",
        "not a",
        "a and (b or not c)",
        "energy and (building:\"hq\" or building:\"warehouse\")",
        "energy.subkind:\"hvac\"",
    ] {
        fp(src);
    }
}
