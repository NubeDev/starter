//! Build a memory-bounded DataFusion `SessionContext` and register one table per
//! federated source under its alias (`ds_<alias>`).
//!
//! DataFusion resolves each `ds_<alias>` name during planning against the tables
//! registered here — no SQL string-scraping decides which datasource a name
//! belongs to (the spec's "DataFusion's OWN catalog resolution" requirement).
//! Each authorised alias becomes exactly one registered table; a SQL reference to
//! a `ds_<alias>` that was not registered is a planning error, which is the
//! desired behaviour for an alias outside the request's authorised map.
//!
//! Input-side memory bound: the context's `RuntimeEnv` carries a `MemoryPool`
//! sized to `max_input_bytes`, so a join that would buffer two huge inputs before
//! producing one capped output row fails with a resource error instead of an OOM.

use std::sync::Arc;

use datafusion::execution::memory_pool::GreedyMemoryPool;
use datafusion::execution::runtime_env::RuntimeEnvBuilder;
use datafusion::prelude::{CsvReadOptions, ParquetReadOptions, SessionConfig, SessionContext};

use super::postgres_table::PostgresTableProvider;
use super::source::FederatedSource;
use crate::core::{EngineError, EngineResult};

/// The SQL table name a federated source is registered (and referenced) under.
/// One place owns the `ds_` prefix so registration and any error message agree.
pub fn table_name(alias: &str) -> String {
    format!("ds_{alias}")
}

/// Build a `SessionContext` bounded by `max_input_bytes` of working memory and
/// register every `(alias, source)` as the table `ds_<alias>`. `max_fetch_rows`
/// caps each remote SQL fetch. Returns the ready-to-query context.
pub async fn build(
    sources: &[(String, FederatedSource)],
    max_input_bytes: usize,
    max_fetch_rows: usize,
) -> EngineResult<SessionContext> {
    let runtime = RuntimeEnvBuilder::new()
        .with_memory_pool(Arc::new(GreedyMemoryPool::new(max_input_bytes)))
        .build()
        .map_err(|e| EngineError::Build(format!("federation runtime: {e}")))?;
    let ctx = SessionContext::new_with_config_rt(SessionConfig::new(), Arc::new(runtime));

    for (alias, source) in sources {
        register_one(&ctx, alias, source, max_fetch_rows).await?;
    }
    Ok(ctx)
}

/// Register one source as the table `ds_<alias>`.
async fn register_one(
    ctx: &SessionContext,
    alias: &str,
    source: &FederatedSource,
    max_fetch_rows: usize,
) -> EngineResult<()> {
    let name = table_name(alias);
    match source {
        FederatedSource::Postgres { conn, table } => {
            let provider = PostgresTableProvider::connect(conn, table, max_fetch_rows).await?;
            ctx.register_table(name.as_str(), Arc::new(provider))
                .map_err(|e| EngineError::Build(format!("federation register postgres: {e}")))?;
        }
        FederatedSource::Parquet { path } => {
            ctx.register_parquet(name.as_str(), path, ParquetReadOptions::default())
                .await
                .map_err(|e| EngineError::Build(format!("federation register parquet: {e}")))?;
        }
        FederatedSource::Csv { path, has_header } => {
            let options = CsvReadOptions::new().has_header(*has_header);
            ctx.register_csv(name.as_str(), path, options)
                .await
                .map_err(|e| EngineError::Build(format!("federation register csv: {e}")))?;
        }
    }
    Ok(())
}
