//! The `datasource` sink: write batches to any registered datasource by kind.
//!
//! One sink, any backend. Its config names a resolved datasource (`kind` +
//! connection material, decrypted by the caller through the audited envelope path
//! before the pipeline builds — `nexus-engine` never touches `nexus-store`), a
//! target `table`, and the batching policy (`batch_rows`/`batch_ms`). It batches
//! rows kind-agnostically ([`batch`]) and dispatches each full batch to a
//! [`DatasourceWriter`] chosen by kind ([`postgres_copy`], [`parquet_file`]). A
//! new kind is a new writer plus one arm in [`open_writer`] — the seam RW-07
//! extensions extend.
//!
//! Flush policy (roadmap §6 write-side backpressure): a batch flushes when the
//! buffer reaches `batch_rows` OR `batch_ms` has elapsed since the first buffered
//! batch, whichever first; `close` (clean end or cancellation) always flushes a
//! non-empty buffer so no row is stranded. The writer is opened lazily on the
//! first write so the registry builder stays synchronous (it returns no future)
//! and a flow that never receives a row opens no connection.

mod batch;
mod identifier;
mod parquet_file;
mod postgres_copy;
mod writer;

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use datafusion::arrow::array::RecordBatch;
use serde::Deserialize;
use serde_json::Value;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use self::batch::BatchAccumulator;
use self::parquet_file::ParquetFileWriter;
use self::postgres_copy::PostgresCopyWriter;
use self::writer::DatasourceWriter;
use crate::core::{EngineError, EngineResult, Sink};

pub use self::writer::DatasourceWriter as Writer;

/// Default rows-per-flush when the config omits `batch_rows`. Matches the engine
/// `max_batch_rows` default so a flow that never sets either still batches sanely.
const DEFAULT_BATCH_ROWS: usize = 8192;

/// Default flush window when the config omits `batch_ms`. One second bounds
/// end-to-end write latency for a low-rate device feed without thrashing.
const DEFAULT_BATCH_MS: u64 = 1000;

/// The resolved datasource sink config. `kind` selects the writer; the rest is
/// kind-specific connection material the caller resolved (decrypted) before build.
#[derive(Debug, Clone, Deserialize)]
struct DatasourceSinkConfig {
    /// Datasource kind — `"postgres"` (covers Timescale) or `"file"`.
    kind: String,
    /// Target table (postgres) — validated as a strict identifier by the writer.
    #[serde(default)]
    table: Option<String>,
    /// Resolved Postgres connection components (postgres kind only). The password
    /// is decrypted upstream through the audited envelope path and is never
    /// logged. Passed as discrete fields rather than a URI so a password with URL
    /// metacharacters needs no percent-encoding dance.
    #[serde(default)]
    conn: Option<PostgresConn>,
    /// Output directory for Parquet part-files (file kind only).
    #[serde(default)]
    dir: Option<String>,
    /// Part-file name prefix (file kind only).
    #[serde(default)]
    prefix: Option<String>,
    /// Rows per flush. Defaults to [`DEFAULT_BATCH_ROWS`].
    #[serde(default)]
    batch_rows: Option<usize>,
    /// Flush window in milliseconds. Defaults to [`DEFAULT_BATCH_MS`].
    #[serde(default)]
    batch_ms: Option<u64>,
}

/// Resolved Postgres connection components. Built by the caller from a decrypted
/// datasource record; the engine only opens the pool.
#[derive(Debug, Clone, Deserialize)]
struct PostgresConn {
    host: String,
    port: u16,
    database: String,
    user: String,
    /// Plaintext password, decrypted upstream. Never logged.
    password: String,
}

/// A sink that batches rows and writes them to a datasource via a kind-specific
/// [`DatasourceWriter`], opened on the first write.
pub struct DatasourceSink {
    accumulator: BatchAccumulator,
    config: DatasourceSinkConfig,
    /// `None` until the first write opens the kind's writer.
    writer: Option<Box<dyn DatasourceWriter>>,
}

impl DatasourceSink {
    /// Build from resolved config. Pure setup — no connection or file is opened
    /// here; the writer opens on the first batch, so building stays synchronous
    /// and side-effect-free like every other registry node.
    pub fn from_config(config: &Value) -> EngineResult<Self> {
        let cfg: DatasourceSinkConfig = serde_json::from_value(config.clone())
            .map_err(|e| EngineError::Build(format!("invalid datasource sink config: {e}")))?;
        let rows = cfg.batch_rows.unwrap_or(DEFAULT_BATCH_ROWS);
        let window = Duration::from_millis(cfg.batch_ms.unwrap_or(DEFAULT_BATCH_MS));
        Ok(Self {
            accumulator: BatchAccumulator::new(rows, window),
            config: cfg,
            writer: None,
        })
    }

    /// Construct directly from an accumulator policy and an already-open writer —
    /// the seam tests (and future in-process callers) use to skip JSON parsing and
    /// live connections.
    pub fn with_writer(rows: usize, window: Duration, writer: Box<dyn DatasourceWriter>) -> Self {
        Self {
            accumulator: BatchAccumulator::new(rows, window),
            // A placeholder config: this path never re-opens a writer.
            config: DatasourceSinkConfig {
                kind: String::new(),
                table: None,
                conn: None,
                dir: None,
                prefix: None,
                batch_rows: Some(rows),
                batch_ms: None,
            },
            writer: Some(writer),
        }
    }

    /// Drain the buffer and write the combined batch through the (lazily opened)
    /// writer, if anything is buffered.
    async fn flush(&mut self) -> EngineResult<()> {
        let Some(batch) = self.accumulator.drain()? else {
            return Ok(());
        };
        if self.writer.is_none() {
            self.writer = Some(open_writer(&self.config).await?);
        }
        self.writer
            .as_mut()
            .expect("writer opened above")
            .write_batch(&batch)
            .await
    }
}

#[async_trait]
impl Sink for DatasourceSink {
    async fn write(&mut self, batch: &RecordBatch) -> EngineResult<()> {
        // A new batch arriving is the moment to apply both thresholds: the
        // pipeline drives writes at the channel's pace, so the time bound is
        // enforced here as an upper bound on buffer age. Clone is a cheap
        // Arc-bump of the columns, not a data copy.
        self.accumulator.push(batch.clone());
        if self.accumulator.rows_due() || self.accumulator.time_due() {
            self.flush().await?;
        }
        Ok(())
    }

    async fn close(&mut self) -> EngineResult<()> {
        // Always drain on close so a partial final buffer is never lost, then
        // finalize the writer (e.g. close the open Parquet footer) if it opened.
        self.flush().await?;
        if let Some(writer) = self.writer.as_mut() {
            writer.flush().await?;
        }
        Ok(())
    }
}

/// Dispatch on the resolved kind to open the right writer. A new datasource kind
/// is a new arm here plus its writer module; nothing else in the sink changes.
async fn open_writer(cfg: &DatasourceSinkConfig) -> EngineResult<Box<dyn DatasourceWriter>> {
    match cfg.kind.as_str() {
        "postgres" => {
            let conn = cfg.conn.as_ref().ok_or_else(|| {
                EngineError::Build("postgres datasource sink requires resolved conn fields".into())
            })?;
            let table = cfg.table.as_deref().ok_or_else(|| {
                EngineError::Build("postgres datasource sink requires a table".into())
            })?;
            let opts = PgConnectOptions::new()
                .host(&conn.host)
                .port(conn.port)
                .database(&conn.database)
                .username(&conn.user)
                .password(&conn.password);
            // A small bounded pool: a sink is one long-lived writer, not a
            // fan-out, so a couple of connections absorb COPY back-to-back.
            let pool = PgPoolOptions::new()
                .max_connections(2)
                .connect_with(opts)
                .await
                .map_err(|e| EngineError::Sink(format!("datasource connect failed: {e}")))?;
            Ok(Box::new(PostgresCopyWriter::new(pool, table)?))
        }
        "file" => {
            let dir = cfg
                .dir
                .as_deref()
                .ok_or_else(|| EngineError::Build("file datasource sink requires a dir".into()))?;
            let prefix = cfg.prefix.clone().unwrap_or_else(|| "part".to_string());
            let rows = cfg.batch_rows.unwrap_or(DEFAULT_BATCH_ROWS);
            Ok(Box::new(ParquetFileWriter::new(
                PathBuf::from(dir),
                prefix,
                rows,
            )))
        }
        other => Err(EngineError::Build(format!(
            "datasource sink: unknown kind {other:?}"
        ))),
    }
}
