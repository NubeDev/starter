//! Pipeline sinks over the [`crate::core::Sink`] trait, and the caps they
//! enforce: a bounded `collector` for one-shot queries, an `sse` broadcast for
//! live streams, a `postgres` writer for ingestion flows, the `datasource` sink
//! that writes batched rows to any datasource by kind, plus `drop`/`stdout`.
//! `store`/`broadcast_store` are the run-id registries the collector and sse
//! sinks resolve their buffer/channel through; `pg_insert` is the shared
//! bound-parameter row insert.

pub mod broadcast_store;
pub mod cap;
pub mod collector_sink;
pub mod datasource;
pub mod drop_sink;
pub mod pg_insert;
pub mod postgres_sink;
pub mod sse_sink;
pub mod stdout;
pub mod store;

pub use collector_sink::CollectorSink;
pub use datasource::DatasourceSink;
pub use drop_sink::DropSink;
pub use postgres_sink::PostgresSink;
pub use sse_sink::SseSink;
pub use stdout::StdoutSink;
