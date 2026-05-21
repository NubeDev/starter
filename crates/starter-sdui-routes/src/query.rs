//! Table-source query engine — per SCOPE.md § R6 / S-D2.
//!
//! `GET /api/v1/ui/table` is the paginated source for
//! [`starter_ui_ir::Component::Table`]. The grammar on the wire is
//! **RSQL** (per Rubix) but the *engine* is host-provided behind
//! this trait — production consumers wire whatever backend their
//! data already lives in.
//!
//! Per **S-D2** (SCOPE.md § Decisions) v1 ships a trait + an
//! in-memory reference implementation that examples and tests rely
//! on. Production consumers wire their own backend against the
//! trait. Promotion to its own crate waits on the first consumer
//! that hits the in-memory engine's limits.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::error::SduiError;

/// One row in a table response. Free-form fields keyed by string
/// path — the renderer's `field` paths address into this map.
pub type TableRow = serde_json::Map<String, JsonValue>;

/// Validated query parameters for `GET /api/v1/ui/table`.
///
/// `source_id` and `filter` are opaque host concerns — RSQL parsing
/// and column whitelisting happen inside the host's
/// [`QueryEngine`] impl. The routes crate only enforces R8's
/// page-size cap before dispatch.
#[derive(Debug, Clone, Deserialize)]
pub struct QueryRequest {
    /// The `source_id` from the `Component::Table.source` block.
    pub source_id: String,
    /// 1-based page number.
    #[serde(default = "default_page")]
    pub page: usize,
    /// Rows per page. Capped server-side per R8 to
    /// [`crate::limits::MAX_TABLE_ROWS_PER_PAGE`].
    #[serde(default = "default_size")]
    pub size: usize,
    /// Sort spec — comma-separated, leading `-` for descending.
    #[serde(default)]
    pub sort: Option<String>,
    /// Extra RSQL filter merged with the source's base query.
    #[serde(default)]
    pub filter: Option<String>,
}

fn default_page() -> usize {
    1
}
fn default_size() -> usize {
    50
}

/// One page of [`TableRow`]s plus metadata for the client's pager.
#[derive(Debug, Clone, Serialize)]
pub struct QueryResponse {
    /// The rows for this page.
    pub data: Vec<TableRow>,
    /// Pagination metadata.
    pub meta: QueryMeta,
}

/// Pager metadata.
#[derive(Debug, Clone, Serialize)]
pub struct QueryMeta {
    /// Total rows across all pages.
    pub total: usize,
    /// Echo of the page that was returned (1-based).
    pub page: usize,
    /// Echo of the page size.
    pub size: usize,
    /// Total number of pages.
    pub pages: usize,
}

/// The host's RSQL query engine. Implementations parse the filter,
/// translate it to their backend, execute, and return one page.
#[async_trait]
pub trait QueryEngine: Send + Sync + 'static {
    /// Execute a query and return one page of rows. Errors that
    /// surface here are mapped to `400 Bad Request` by the route.
    async fn query(&self, req: &QueryRequest) -> Result<QueryResponse, SduiError>;
}

/// Trivial reference engine — holds rows in memory and applies a
/// page slice. Filter / sort are not implemented; this exists to
/// keep the example crates compiling and the route smoke-testable.
/// Production consumers ignore this and wire their own engine.
#[derive(Debug, Default, Clone)]
pub struct InMemoryQueryEngine {
    sources: std::collections::HashMap<String, Vec<TableRow>>,
}

impl InMemoryQueryEngine {
    /// Empty engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register the rows for one `source_id`.
    pub fn with(mut self, source_id: impl Into<String>, rows: Vec<TableRow>) -> Self {
        self.sources.insert(source_id.into(), rows);
        self
    }
}

#[async_trait]
impl QueryEngine for InMemoryQueryEngine {
    async fn query(&self, req: &QueryRequest) -> Result<QueryResponse, SduiError> {
        let rows = self
            .sources
            .get(&req.source_id)
            .cloned()
            .unwrap_or_default();
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
        Ok(QueryResponse {
            data: slice,
            meta: QueryMeta { total, page, size, pages },
        })
    }
}
