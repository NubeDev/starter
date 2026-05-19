//! Request-id extension type. The transport-side middleware that
//! attaches one per request lives in `starter-server` so the concrete
//! `http::Request` body type is pinned in one place — this crate
//! deliberately stays HTTP-framework-agnostic.

use uuid::Uuid;

/// Wrapper around a request-scoped UUID. The starter-server middleware
/// inserts one of these as a request extension on each incoming
/// request; handlers extract it via `axum::Extension<RequestId>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestId(pub Uuid);

impl RequestId {
    /// Generate a new random v4 id.
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

/// Standard `X-Request-Id` header name. Used by both the inbound
/// adopt-or-generate logic and the outbound echo header.
pub const HEADER_NAME: &str = "x-request-id";
