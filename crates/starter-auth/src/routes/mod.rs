//! `/auth/*` routes. One file per endpoint so the handlers stay
//! easy to find by name.

mod login;
mod logout;
mod me;
mod router;

pub use router::auth_router;
