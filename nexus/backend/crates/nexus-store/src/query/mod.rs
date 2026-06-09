//! The guarded one-shot query path against a datasource Postgres.

mod row_json;
mod run;

use std::time::Duration;

pub use run::run_query;

/// The server-enforced safety bounds applied to every datasource query. None of
/// these are expressible by the caller — they are set from server config and the
/// datasource's policy, never from the request body.
#[derive(Debug, Clone, Copy)]
pub struct QueryGuards {
    /// Postgres `statement_timeout` for the query transaction.
    pub statement_timeout: Duration,
    /// Maximum rows to return before truncating.
    pub max_rows: u64,
    /// Maximum serialized bytes to return before truncating.
    pub max_bytes: u64,
}
