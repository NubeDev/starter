//! Per-user query history: record runs, list recent, star a favourite.
//!
//! A thin recent-history ledger, RLS-isolated per tenant like the rest of the
//! control plane (see [`crate::tenant_tx`]). Every function runs inside a
//! tenant-bound transaction so a tenant only ever touches its own rows. Durable
//! change history is WS-12's changelog, not this table — history here is bounded
//! per user on write.

mod list;
mod record;
mod row;
mod star;

use starter_spi::Error;

pub use list::list_recent;
pub use record::record_run;
pub use row::{NewQueryRun, QueryHistoryRow};
pub use star::set_starred;

/// The number of rows kept per user; older rows are trimmed on each record so
/// the table stays a recent-history ledger rather than an unbounded log.
pub const RETENTION_PER_USER: i64 = 200;

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
