//! RFC 7807-style problem document. The single error shape every
//! starter-server endpoint returns when a request fails.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Machine-readable error body.
///
/// Mirrors RFC 7807 loosely. The `type` field is a stable string
/// identifier (`not_found`, `invalid_input`, …) that callers can
/// switch on; the HTTP status is set by the transport.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Problem {
    /// Stable identifier for the error class. Matches lower-case
    /// snake-case of [`crate::error::Error`] variants.
    #[serde(rename = "type")]
    pub kind: String,

    /// Short human title for the problem.
    pub title: String,

    /// Optional detailed explanation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}
