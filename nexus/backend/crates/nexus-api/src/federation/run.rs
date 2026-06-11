//! Run a resolved federated query and shape its outcome into a `QueryResponse`.
//!
//! The federation engine ([`nexus_engine::FederatedQuery`]) executes the
//! cross-datasource SQL under the same caps the single-datasource path enforces,
//! derived here from the server [`QueryGuards`] so a federated query cannot
//! exceed the row/byte/wall-clock bounds a push-down query is held to.
//!
//! Macros/variables: a federated statement runs as written against the registered
//! `ds_<alias>` tables. The push-down `$__timeFilter`/`$var` binder produces
//! Postgres `$N` placeholders for sqlx, which DataFusion cannot consume, so it is
//! deliberately not applied here — a federated query is a literal read-only join,
//! and the engine rejects DDL/DML/statements so the literal SQL is still safe.

use std::time::Duration;

use nexus_engine::{Caps, FederatedQuery, FederatedSource};
use nexus_spi::dto::query::{QueryRequest, QueryResponse};
use nexus_store::QueryGuards;
use starter_spi::Error;

/// Run the request's `sql` over `sources`, bounded by `guards`, and map the
/// engine outcome to the wire response. The columns/rows/stats shapes are
/// identical to the push-down path's response, so a panel renders a federated
/// result with no special-casing.
pub async fn run_federated(
    req: &QueryRequest,
    sources: Vec<(String, FederatedSource)>,
    guards: QueryGuards,
) -> Result<QueryResponse, Error> {
    let query = FederatedQuery {
        sql: req.sql.clone(),
        sources,
    };
    let outcome = query.run(caps_from(guards)).await.map_err(|e| Error::Invalid {
        message: e.to_string(),
    })?;
    Ok(QueryResponse {
        columns: outcome.columns,
        rows: outcome.rows,
        stats: outcome.stats,
    })
}

/// Translate the server guards into engine caps. The mapping is one-to-one:
/// `statement_timeout` → wall-clock, `max_rows`/`max_bytes` → the output caps.
fn caps_from(guards: QueryGuards) -> Caps {
    Caps::new(
        guards.max_rows,
        guards.max_bytes,
        Duration::from_millis(guards.statement_timeout.as_millis().max(1) as u64),
    )
}
