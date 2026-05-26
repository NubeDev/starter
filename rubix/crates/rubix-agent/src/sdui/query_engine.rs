//! `RubixQueryEngine` — rubix's impl of
//! [`starter_sdui_routes::QueryEngine`] for `GET /api/v1/ui/table`.
//!
//! Stage 3 of `rubix/docs/proposal/warehouse-engine-swap.md` removed
//! the `ch:` ClickHouse backend prefix. Remaining backends:
//!
//! | Prefix | Backend | Notes |
//! |--------|---------|-------|
//! | `pg:<table>` | Postgres via the agent's [`Pool`] | dimensions |
//! | `mem:<id>`  | In-memory fixture | reserved for tests and demos |
//! | anything else | — | returns an empty page |

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Map, Value as JsonValue};
use starter_sdui_routes::{
    query::{QueryEngine, QueryMeta, QueryRequest, QueryResponse, TableRow},
    SduiError,
};
use starter_store_postgres::pool::Pool;
use tracing::warn;

enum Backend<'a> {
    Postgres(&'a str),
    Memory(&'a str),
    Unknown,
}

fn parse_backend(source_id: &str) -> Backend<'_> {
    if let Some(rest) = source_id.strip_prefix("pg:") {
        return Backend::Postgres(rest);
    }
    if let Some(rest) = source_id.strip_prefix("mem:") {
        return Backend::Memory(rest);
    }
    Backend::Unknown
}

#[derive(Clone)]
pub struct RubixQueryEngine {
    pg: Option<Pool>,
    memory: Arc<std::collections::HashMap<String, Vec<TableRow>>>,
}

impl std::fmt::Debug for RubixQueryEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RubixQueryEngine")
            .field("pg", &self.pg.is_some())
            .field("memory_sources", &self.memory.len())
            .finish()
    }
}

impl RubixQueryEngine {
    pub fn new(pg: Option<Pool>) -> Self {
        Self {
            pg,
            memory: Arc::new(std::collections::HashMap::new()),
        }
    }

    pub fn with_memory_source(mut self, id: impl Into<String>, rows: Vec<TableRow>) -> Self {
        let map = Arc::make_mut(&mut self.memory);
        map.insert(id.into(), rows);
        self
    }

    async fn query_pg(&self, _table: &str, req: &QueryRequest) -> Result<QueryResponse, SduiError> {
        if self.pg.is_none() {
            warn!(target: "rubix.sdui", source = %req.source_id, "pg backend not wired; returning empty page");
        }
        Ok(empty_page(req))
    }

    fn query_memory(&self, id: &str, req: &QueryRequest) -> Result<QueryResponse, SduiError> {
        let rows = self.memory.get(id).cloned().unwrap_or_default();
        Ok(page_slice(rows, req))
    }
}

#[async_trait]
impl QueryEngine for RubixQueryEngine {
    async fn query(&self, req: &QueryRequest) -> Result<QueryResponse, SduiError> {
        match parse_backend(&req.source_id) {
            Backend::Postgres(t) => self.query_pg(t, req).await,
            Backend::Memory(id) => self.query_memory(id, req),
            Backend::Unknown => {
                warn!(
                    target: "rubix.sdui",
                    source = %req.source_id,
                    "unknown backend prefix; returning empty page",
                );
                Ok(empty_page(req))
            }
        }
    }
}

fn empty_page(req: &QueryRequest) -> QueryResponse {
    page_slice(Vec::<TableRow>::new(), req)
}

fn page_slice(rows: Vec<TableRow>, req: &QueryRequest) -> QueryResponse {
    let total = rows.len();
    let size = req.size.max(1);
    let pages = total.div_ceil(size).max(1);
    let page = req.page.max(1);
    let start = (page - 1) * size;
    let end = (start + size).min(total);
    let slice = if start < end {
        rows[start..end].to_vec()
    } else {
        Vec::new()
    };
    QueryResponse {
        data: slice,
        meta: QueryMeta {
            total,
            page,
            size,
            pages,
        },
    }
}

#[allow(dead_code)]
fn json_obj(pairs: &[(&str, JsonValue)]) -> TableRow {
    let mut m: Map<String, JsonValue> = Map::new();
    for (k, v) in pairs {
        m.insert((*k).to_string(), v.clone());
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(source: &str) -> QueryRequest {
        QueryRequest {
            source_id: source.into(),
            page: 1,
            size: 50,
            sort: None,
            filter: None,
        }
    }

    #[tokio::test]
    async fn unknown_prefix_returns_empty_page() {
        let eng = RubixQueryEngine::new(None);
        let r = eng.query(&req("nope:42")).await.unwrap();
        assert!(r.data.is_empty());
        assert_eq!(r.meta.total, 0);
        assert_eq!(r.meta.page, 1);
        assert_eq!(r.meta.pages, 1);
    }

    #[tokio::test]
    async fn pg_backend_returns_empty_when_unwired() {
        let eng = RubixQueryEngine::new(None);
        assert!(eng
            .query(&req("pg:flows_definitions"))
            .await
            .unwrap()
            .data
            .is_empty());
    }

    #[tokio::test]
    async fn memory_backend_returns_seeded_rows_paged() {
        let rows: Vec<TableRow> = (0..7)
            .map(|i| json_obj(&[("i", JsonValue::from(i))]))
            .collect();
        let eng = RubixQueryEngine::new(None).with_memory_source("demo", rows);

        let r = eng
            .query(&QueryRequest {
                source_id: "mem:demo".into(),
                page: 2,
                size: 3,
                sort: None,
                filter: None,
            })
            .await
            .unwrap();
        assert_eq!(r.data.len(), 3);
        assert_eq!(r.meta.total, 7);
        assert_eq!(r.meta.pages, 3);
        assert_eq!(r.meta.page, 2);
        assert_eq!(r.data[0]["i"], JsonValue::from(3));
    }

    #[test]
    fn backend_prefixes_are_parsed_exhaustively() {
        assert!(matches!(parse_backend("pg:a"), Backend::Postgres("a")));
        assert!(matches!(parse_backend("mem:a"), Backend::Memory("a")));
        assert!(matches!(parse_backend("ch:a"), Backend::Unknown));
        assert!(matches!(parse_backend("a"), Backend::Unknown));
    }
}
