//! Change tail — backend-neutral subscription to newly recorded rows.
//!
//! Postgres backends implement this with `LISTEN/NOTIFY`; SQLite with
//! polling. See SCOPE §"Storage shape" final paragraph.

use async_trait::async_trait;

use starter_spi::changelog::Change;
use starter_spi::Result;

/// A live stream of newly committed changes.
#[async_trait]
pub trait ChangeTail: Send + Sync {
    /// Subscribe. The returned receiver yields rows in commit order.
    /// Backends define their own buffering / lag semantics.
    async fn subscribe(&self) -> Result<tokio::sync::mpsc::Receiver<Change>>;
}
