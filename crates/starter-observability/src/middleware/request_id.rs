//! Request-id middleware. Generates a v4 UUID per request (or
//! adopts an incoming `X-Request-Id` header if present) and attaches
//! it as a request extension so downstream handlers + tracing spans
//! can reference it.

use uuid::Uuid;

/// Wrapper around a request-scoped UUID. Inserted as an extension
/// on the request so handlers can extract it.
#[derive(Debug, Clone, Copy)]
pub struct RequestId(pub Uuid);

impl RequestId {
    /// Generate a new id.
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Build a tower `Layer` that attaches a [`RequestId`] to each
/// incoming request.
///
/// The actual layer construction lands with the server crate so we
/// can pin the concrete service shape without committing to a
/// specific http body type from this crate.
pub fn request_id_layer() {
    // TODO(ap): implement once starter-server's service type is
    // chosen (likely tower::ServiceBuilder over axum's BoxBody).
    // Stub kept so the public surface is reserved.
}
