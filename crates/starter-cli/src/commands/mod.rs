//! Starter-shipped subcommands. One file per subcommand so an AI
//! editing `health` doesn't load the `admin create` flow.

#[allow(dead_code)] // template kept as docs; consumers wire their own
mod admin_create;
mod health;
mod openapi;
mod prefs;
#[cfg(feature = "prune")]
mod prune;

pub use health::Health;
pub use openapi::OpenApi;
pub use prefs::{run_with as run_prefs_with, Prefs};
#[cfg(feature = "prune")]
pub use prune::{run_with as run_prune_with, Prune};
