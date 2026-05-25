//! `RubixQueryEngine` — rubix's impl of
//! [`starter_sdui_routes::QueryEngine`] for `GET /api/v1/ui/table`.
//!
//! Per `03-host-glue.md`, the table source `source_id` carries a
//! backend prefix that routes the query to the right data plane:
//!
//! | Prefix | Backend | Notes |
//! |--------|---------|-------|
//! | `ch:<table>` | ClickHouse via [`ChClient`] | history rows, time-series |
//! | `pg:<table>` | Postgres via the agent's [`Pool`] | dimensions |
//! | `mem:<id>`  | In-memory fixture | reserved for tests and demos |
//! | anything else | — | returns an empty result set with a `feature not yet implemented` diagnostic-friendly empty page |
//!
//! v1 keeps both backends behind a deliberately small surface: the
//! engine forwards the validated [`QueryRequest`] to the matching
//! backend, applies the global page-size cap (already enforced by
//! the route via [`starter_sdui_routes::limits`]), and returns a
//! deterministic empty page when the prefix is unknown. RSQL parsing
//! / column whitelisting / fancy aggregations live in follow-up
//! stages — the seam is here so static `Component::Table` sources
//! can render without panicking.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Map, Value as JsonValue};
use starter_sdui_routes::{
    query::{QueryEngine, QueryMeta, QueryRequest, QueryResponse, TableRow},
    SduiError,
};
use starter_store_clickhouse::ChClient;
use starter_store_postgres::pool::Pool;
use tracing::warn;

/// Backend selector parsed from `source_id`.
enum Backend<'a> {
    Clickhouse(&'a str),
    Postgres(&'a str),
    Memory(&'a str),
    Unknown,
}

fn parse_backend(source_id: &str) -> Backend<'_> {
    if let Some(rest) = source_id.strip_prefix("ch:") {
        return Backend::Clickhouse(rest);
    }
    if let Some(rest) = source_id.strip_prefix("pg:") {
        return Backend::Postgres(rest);
    }
    if let Some(rest) = source_id.strip_prefix("mem:") {
        return Backend::Memory(rest);
    }
    Backend::Unknown
}

/// Cheap-to-clone query engine.
#[derive(Clone)]
pub struct RubixQueryEngine {
    pg: Option<Pool>,
    ch: Option<Arc<ChClient>>,
    memory: Arc<std::collections::HashMap<String, Vec<TableRow>>>,
}

impl std::fmt::Debug for RubixQueryEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RubixQueryEngine")
            .field("pg", &self.pg.is_some())
            .field("ch", &self.ch.is_some())
            .field("memory_sources", &self.memory.len())
            .finish()
    }
}

impl RubixQueryEngine {
    /// Build with the optional production backends. Either may be
    /// `None` on the laptop boot path; matching prefixes then resolve
    /// to an empty page.
    pub fn new(pg: Option<Pool>, ch: Option<Arc<ChClient>>) -> Self {
        Self {
            pg,
            ch,
            memory: Arc::new(std::collections::HashMap::new()),
        }
    }

    /// Seed the in-memory backend (`mem:<id>`) for demos and tests.
    pub fn with_memory_source(
        mut self,
        id: impl Into<String>,
        rows: Vec<TableRow>,
    ) -> Self {
        let map = Arc::make_mut(&mut self.memory);
        map.insert(id.into(), rows);
        self
    }

    async fn query_pg(
        &self,
        _table: &str,
        req: &QueryRequest,
    ) -> Result<QueryResponse, SduiError> {
        // v1: the table is not parsed against any column whitelist
        // and RSQL → SQL translation has not landed (Phase B.4). We
        // return a deterministic empty page so static Table sources
        // pointing at `pg:<table>` render their "no rows" empty
        // state instead of 500ing.
        if self.pg.is_none() {
            warn!(target: "rubix.sdui", source = %req.source_id, "pg backend not wired; returning empty page");
        }
        Ok(empty_page(req))
    }

    async fn query_ch(
        &self,
        _table: &str,
        req: &QueryRequest,
    ) -> Result<QueryResponse, SduiError> {
        // Mirror of `query_pg` — ClickHouse translation lands with
        // the RSQL → CH aggregator (Phase B.4).
        if self.ch.is_none() {
            warn!(target: "rubix.sdui", source = %req.source_id, "ch backend not wired; returning empty page");
        }
        Ok(empty_page(req))
    }

    fn query_memory(
        &self,
        id: &str,
        req: &QueryRequest,
    ) -> Result<QueryResponse, SduiError> {
        let rows = self.memory.get(id).cloned().unwrap_or_default();
        Ok(page_slice(rows, req))
    }
}

#[async_trait]
impl QueryEngine for RubixQueryEngine {
    async fn query(&self, req: &QueryRequest) -> Result<QueryResponse, SduiError> {
        match parse_backend(&req.source_id) {
            Backend::Clickhouse(t) => self.query_ch(t, req).await,
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
        let eng = RubixQueryEngine::new(None, None);
        let r = eng.query(&req("nope:42")).await.unwrap();
        assert!(r.data.is_empty());
        assert_eq!(r.meta.total, 0);
        assert_eq!(r.meta.page, 1);
        assert_eq!(r.meta.pages, 1);
    }

    #[tokio::test]
    async fn pg_and_ch_backends_return_empty_when_unwired() {
        let eng = RubixQueryEngine::new(None, None);
        assert!(eng.query(&req("pg:flows_definitions")).await.unwrap().data.is_empty());
        assert!(eng.query(&req("ch:samples")).await.unwrap().data.is_empty());
    }

    #[tokio::test]
    async fn memory_backend_returns_seeded_rows_paged() {
        let rows: Vec<TableRow> = (0..7)
            .map(|i| json_obj(&[("i", JsonValue::from(i))]))
            .collect();
        let eng = RubixQueryEngine::new(None, None).with_memory_source("demo", rows);

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
        assert!(matches!(parse_backend("ch:a"), Backend::Clickhouse("a")));
        assert!(matches!(parse_backend("pg:a"), Backend::Postgres("a")));
        assert!(matches!(parse_backend("mem:a"), Backend::Memory("a")));
        assert!(matches!(parse_backend("a"), Backend::Unknown));
    }
}
