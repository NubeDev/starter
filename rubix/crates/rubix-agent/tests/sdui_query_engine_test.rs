//! Sibling integration coverage for `RubixQueryEngine` — exercises
//! the `source_id`-prefix dispatch without needing a live PG/CH
//! backend.

use rubix_agent::sdui::query_engine::RubixQueryEngine;
use serde_json::{json, Map, Value};
use starter_sdui_routes::query::{QueryEngine, QueryRequest, TableRow};

fn req(source: &str, page: usize, size: usize) -> QueryRequest {
    QueryRequest {
        source_id: source.into(),
        page,
        size,
        sort: None,
        filter: None,
    }
}

#[tokio::test]
async fn unknown_backend_returns_deterministic_empty_page() {
    let eng = RubixQueryEngine::new(None);
    let r = eng.query(&req("xxx:nope", 1, 50)).await.unwrap();
    assert!(r.data.is_empty());
    assert_eq!(r.meta.total, 0);
    assert_eq!(r.meta.pages, 1);
}

#[tokio::test]
async fn pg_and_ch_backends_return_empty_when_unwired() {
    let eng = RubixQueryEngine::new(None);
    for src in ["pg:flows_definitions", "ch:samples", "ch:raw_events"] {
        let r = eng.query(&req(src, 1, 50)).await.unwrap();
        assert!(r.data.is_empty(), "{src} returned non-empty rows");
    }
}

#[tokio::test]
async fn memory_backend_pages_through_seeded_rows() {
    let rows: Vec<TableRow> = (0..5)
        .map(|i| {
            let mut m = Map::new();
            m.insert("i".into(), Value::from(i));
            m
        })
        .collect();
    let eng = RubixQueryEngine::new(None).with_memory_source("demo", rows);

    let p1 = eng.query(&req("mem:demo", 1, 2)).await.unwrap();
    assert_eq!(p1.data.len(), 2);
    assert_eq!(p1.meta.total, 5);
    assert_eq!(p1.meta.pages, 3);
    assert_eq!(p1.data[0]["i"], json!(0));

    let p3 = eng.query(&req("mem:demo", 3, 2)).await.unwrap();
    assert_eq!(p3.data.len(), 1, "trailing partial page");
    assert_eq!(p3.data[0]["i"], json!(4));
}
