//! Request-edge concerns: stream-token auth now; principal/tenant/authz wiring
//! join as the identity milestone lands.

pub mod stream_token;

pub use stream_token::{StreamClaims, StreamTokenSigner, TokenError};
