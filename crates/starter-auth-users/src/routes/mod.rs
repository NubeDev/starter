//! `/auth/*` routes. One file per endpoint so the handlers stay
//! easy to find by name.

mod login;
mod logout;
mod me;
mod router;
mod state;

pub use login::{LoginRequest, LoginResponse, CSRF_COOKIE};
pub use me::MeResponse;
pub use router::auth_router;
pub use state::AuthState;
