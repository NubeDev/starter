//! Body of the `/health` endpoint every starter-server ships.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Server health snapshot. Returned by `GET /health`.
///
/// `status = "ok"` is the only success value. Anything else is a
/// degraded state the caller should treat as failure even if the
/// HTTP status is 200.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Health {
    /// Coarse health string. `"ok"` or a degraded reason.
    pub status: String,

    /// Build version string (`CARGO_PKG_VERSION` of the consumer's
    /// binary).
    pub version: String,

    /// Uptime in seconds since process start.
    pub uptime_seconds: u64,
}
