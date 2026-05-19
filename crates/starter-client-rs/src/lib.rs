//! # starter-client-rs
//!
//! Reqwest-based HTTP client mirroring `starter-server`'s routes.
//! Used by consumer CLI binaries and any Rust caller of a
//! starter-based server. **Zero `starter-server` dep** — only
//! `starter-spi` types cross the boundary.
//!
//! - [`client`] — `Client` struct + builder.
//! - [`endpoints`] — one file per endpoint family (health, auth, …).
//! - [`error`] — client error type.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod client;
pub mod endpoints;
pub mod error;

pub use client::Client;
pub use error::ClientError;
