//! Connect a `Pool` from a SQLite URL.

use sqlx::{sqlite::SqlitePoolOptions, Executor, SqlitePool};

use super::wrapper::Pool;

/// Connect to a SQLite database.
///
/// `url` follows sqlx's convention: `sqlite::memory:` for in-memory,
/// `sqlite:./data.db` for a file. Returns a ready-to-use [`Pool`].
///
/// Per `DOCS/flow/scope/SCOPE.md` Phase 3 D-F3.8 the pool applies a
/// fixed set of pragmas on every fresh connection so the 24/7
/// supervisory-runtime durability posture holds workspace-wide
/// (not just for `starter-flow` consumers). The pragmas are safe
/// defaults for every existing consumer:
///
/// - `journal_mode = WAL` — survives process crashes at WAL-fsync
///   boundaries (no-op on `:memory:`, which returns `memory`).
/// - `synchronous = NORMAL` — the SQLite documented sweet spot for
///   write-heavy long-running workloads (the per-tick checkpoint
///   batch in [Phase 3] amortises commit cost; `FULL` would halve
///   throughput for the same crash-safety guarantee).
/// - `busy_timeout = 5000` — five-second internal retry on writer
///   contention before surfacing `SQLITE_BUSY`.
/// - `foreign_keys = ON` — SQLite's default-off FK enforcement is a
///   long-standing footgun; turning it on at connection init matches
///   every other workspace store crate's expectation.
pub async fn connect(url: &str) -> Result<Pool, sqlx::Error> {
    let inner: SqlitePool = SqlitePoolOptions::new()
        .max_connections(8)
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                // Order matters slightly: WAL first so subsequent
                // pragmas observe the WAL journal. Each PRAGMA is
                // its own statement so an `:memory:` database
                // silently ignoring `journal_mode = WAL` doesn't
                // shadow the rest.
                conn.execute("PRAGMA journal_mode = WAL").await?;
                conn.execute("PRAGMA synchronous = NORMAL").await?;
                conn.execute("PRAGMA busy_timeout = 5000").await?;
                conn.execute("PRAGMA foreign_keys = ON").await?;
                Ok(())
            })
        })
        .connect(url)
        .await?;
    Ok(Pool::from_sqlx(inner))
}
