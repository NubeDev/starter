//! `POST /streams` — register a live subscription and mint its SSE token.
//!
//! Native `EventSource` cannot send an `Authorization` header, so the SSE route
//! is authenticated by a short-lived signed token rather than the REST Bearer.
//! This Bearer-authed call returns that token plus the URL to connect to.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Body for creating a live stream: the datasource to read and the SQL shaping
/// applied to each batch before it is pushed to subscribers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CreateStreamRequest {
    /// Datasource whose live input feeds this stream.
    pub datasource_id: Uuid,
    /// SQL applied per batch (DataFusion pipeline) to shape the live rows.
    pub sql: String,
}

/// Result of creating a stream. The token is single-use-per-connection,
/// short-lived, and scoped to exactly this stream — it authorizes the SSE
/// subscription and nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CreateStreamResponse {
    /// Immutable stream id.
    pub id: Uuid,
    /// Signed token to pass as `?token=` when opening the SSE connection.
    pub token: String,
    /// Relative URL to connect an `EventSource` to, with the token embedded.
    pub subscribe_url: String,
    /// Token lifetime in seconds; the client must connect before it expires.
    pub expires_in_secs: u64,
}
