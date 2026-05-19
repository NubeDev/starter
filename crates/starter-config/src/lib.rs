//! # starter-config
//!
//! Layered config loader. The consumer brings their own
//! `serde::Deserialize` struct; this crate composes default → file →
//! env in a fixed order and hands back the populated value.
//!
//! No HTTP, no DB, no domain types. Generic over the consumer's
//! shape. See SCOPE.md "What each crate / package owns".

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod error;
pub mod loader;
pub mod source;

pub use error::ConfigError;
pub use loader::Loader;
