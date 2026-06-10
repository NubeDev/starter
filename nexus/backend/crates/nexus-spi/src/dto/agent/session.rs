//! `POST /api/v1/agents/:id/sessions` request — start a new session.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Open a session against an agent by sending its opening prompt. The agent's
/// system prompt (if any) is prepended by the server; the caller supplies only
/// the user message. The response carries the session id whose SSE feed streams
/// the run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CreateSessionRequest {
    /// The opening user message.
    pub prompt: String,
}

/// The response to opening a session: the session id (now `running`) plus a
/// short-lived signed token to connect its SSE feed at
/// `GET /api/v1/agents/sessions/{id}/events?token=…`. A browser `EventSource`
/// cannot set an auth header, so the token rides the query string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CreateSessionResponse {
    pub id: uuid::Uuid,
    pub status: String,
    /// Signed token for the SSE subscription.
    pub token: String,
}
