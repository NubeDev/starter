//! Auth-route handler context. Holds the three stores plus the
//! cookie-naming configuration. Wrapped in `Arc` so the handlers
//! (which close over it) stay cheap to clone per request.

use std::sync::Arc;

use crate::store::{SessionStore, TokenStore, UserStore};

/// Shared state for the `/auth/*` handlers. Build once at startup and
/// hand to [`super::auth_router`].
#[derive(Clone)]
pub struct AuthState {
    /// User table.
    pub users: Arc<dyn UserStore>,
    /// Session table.
    pub sessions: Arc<dyn SessionStore>,
    /// Token table.
    pub tokens: Arc<dyn TokenStore>,
}

impl AuthState {
    /// Build the state from the three stores.
    pub fn new(
        users: Arc<dyn UserStore>,
        sessions: Arc<dyn SessionStore>,
        tokens: Arc<dyn TokenStore>,
    ) -> Self {
        Self {
            users,
            sessions,
            tokens,
        }
    }
}
