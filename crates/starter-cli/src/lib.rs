//! # starter-cli
//!
//! Library of clap building blocks. The consumer's binary depends
//! on this crate + their own domain crates, and assembles its own
//! CLI by registering both starter-provided and consumer-provided
//! `Command` impls on a `CommandRegistry`.
//!
//! **There is no `main.rs` in this crate** — it's a library.
//!
//! - [`commands`] — starter-shipped subcommands (`health`, `openapi`,
//!   `admin`, …).
//! - [`registry`] — `Command` trait + `CommandRegistry`.
//! - [`prompt`] — interactive prompts (password input, confirmation).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod commands;
pub mod prompt;
pub mod registry;

pub use registry::{Command, CommandRegistry};
