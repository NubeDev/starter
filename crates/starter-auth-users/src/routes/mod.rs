//! `/auth/*` routes. One file per endpoint so the handlers stay
//! easy to find by name.

pub(crate) mod login;
pub(crate) mod logout;
pub(crate) mod me;
mod router;
mod state;

pub use login::{LoginRequest, LoginResponse, CSRF_COOKIE};
pub use me::MeResponse;
pub use router::auth_router;
pub use state::AuthState;
