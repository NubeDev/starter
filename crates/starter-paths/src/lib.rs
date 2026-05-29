//! `starter-paths` — OS-aware application data-root resolver.
//!
//! One job: tell every other crate where durable on-disk state lives,
//! so the workspace stops inventing its own answer in each consumer.
//! Resolution honours an explicit override, then `$<APP>_DATA_ROOT`,
//! then the OS convention (`$XDG_DATA_HOME/<app>/` on Linux,
//! `~/Library/Application Support/<app>/` on macOS,
//! `%LOCALAPPDATA%/<app>/` on Windows).
//!
//! No domain types, no HTTP, no DB — same posture as `starter-config`.
//! Consumers depend on [`Paths`] and ask for the subdir they need.
//!
//! See `rubix/docs/scope/extensions/data-root-and-safe-uninstall.md`
//! for the design.

mod error;
mod paths;
mod resolve;

pub use error::PathsError;
pub use paths::Paths;
