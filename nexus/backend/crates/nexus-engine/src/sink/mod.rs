//! Pipeline sinks: the custom ArkFlow output sinks and the caps they enforce,
//! plus their native ports onto the RW-01 [`crate::core::Sink`] trait (additive
//! while RW-02 runs; the ArkFlow versions stay until RW-03 cuts over).

pub mod broadcast_store;
pub mod cap;
pub mod collector;
pub mod collector_sink;
pub mod drop_sink;
pub mod pg_insert;
pub mod postgres;
pub mod postgres_sink;
pub mod sse;
pub mod sse_sink;
pub mod stdout;
pub mod store;

pub use collector_sink::CollectorSink;
pub use drop_sink::DropSink;
pub use postgres_sink::PostgresSink;
pub use sse_sink::SseSink;
pub use stdout::StdoutSink;
