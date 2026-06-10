//! `POST /api/v1/ingest/{flow_id}` success response.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Acknowledges that a push was enqueued onto a flow's bounded channel. The rows
/// are accepted, not yet written — the flow's sink writes them asynchronously, so
/// this is a `202`-style accept, not a write confirmation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct IngestAccepted {
    /// How many JSON documents the push contributed (one for a single object, the
    /// element count for an array).
    pub accepted: u64,
}
