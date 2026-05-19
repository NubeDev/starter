//! In-memory SQLite pool with no migrations applied. Callers run
//! their own migrations after.

use crate::pool::{connect, Pool};

/// Return a fresh in-memory SQLite [`Pool`].
///
/// Each call returns a different database. No migrations are applied
/// — register them with `migrate(...)` afterward.
pub async fn ephemeral() -> Pool {
    connect("sqlite::memory:")
        .await
        .expect("in-memory sqlite must connect")
}
