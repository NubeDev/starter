//! Provider implementations.
//!
//! One file per provider; each file is a single
//! [`crate::OAuthProvider`] impl plus its compile-time endpoint /
//! scope constants. Per Hard rule R6 / R7, this is the only place
//! provider-specific code lives — routes, state store, identity
//! table, and linking logic stay provider-agnostic.

pub mod github;

pub use github::GitHubProvider;
