//! The `AuthAuthenticator` — bridges both cookie and bearer paths
//! into a single [`starter_spi::auth::Authenticator`] impl. Route
//! guards never need to know which path the caller used.

mod impl_;

pub use impl_::AuthAuthenticator;
