//! # starter-spi
//!
//! Contracts crate. Every other starter crate depends on this one;
//! this one depends on nothing internal. See [`SCOPE.md`] rules R2 and R7.
//!
//! The body of every public item lives in its own file under
//! `src/<concept>/`. This file is a re-export barrel — keep it that way.
//! See [`HOW-TO-ADD-CODE.md`] rule "one responsibility per file".
//!
//! [`SCOPE.md`]: ../../SCOPE.md
//! [`HOW-TO-ADD-CODE.md`]: ../../SCOPE.md

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod ai;
pub mod auth;
pub mod dto;
pub mod error;
pub mod filter;
pub mod i18n;
pub mod id;
pub mod paging;
pub mod preferences;
pub mod secrets;
pub mod service;
pub mod sort;
pub mod tool;
pub mod units;

pub use error::{Error, Result};
pub use id::Id;
pub use paging::{Cursor, Page};

/// `secrecy::SecretString` re-exported at the crate root.
///
/// SCOPE rule R5: provider crates take already-resolved secrets as
/// `SecretString` and **do not depend on `secrecy` directly**. They
/// reach for `starter_spi::SecretString` instead. This is the only
/// place in the workspace `secrecy` is named as a direct dependency.
pub use secrecy::SecretString;

/// `secrecy::ExposeSecret` re-exported at the crate root.
///
/// R5 forbids provider crates from naming `secrecy` directly, but they
/// still have to read the plaintext out of a [`SecretString`] when
/// calling the third-party API. Importing
/// `starter_spi::ExposeSecret` and calling `.expose_secret()` is the
/// sanctioned path; the trait lives here so the `secrecy` crate name
/// never appears in any provider crate's `Cargo.toml`.
pub use secrecy::ExposeSecret;
