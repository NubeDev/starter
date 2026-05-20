//! `/auth/*` routes. One file per endpoint so the handlers stay
//! easy to find by name.

pub(crate) mod login;
pub(crate) mod logout;
pub(crate) mod me;
pub(crate) mod signup;
mod router;
mod state;

pub use login::{LoginRequest, LoginResponse, PasswordNotSetResponse, CSRF_COOKIE};
pub use me::MeResponse;
pub use router::auth_router;
pub use signup::{SignupError, SignupRequest, SignupResponse};
pub use state::AuthState;
