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
pub mod id;
pub mod paging;
pub mod secrets;
pub mod sort;
pub mod tool;

pub use error::{Error, Result};
pub use id::Id;
pub use paging::{Cursor, Page};
