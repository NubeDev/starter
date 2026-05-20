//! Starter-shipped subcommands. One file per subcommand so an AI
//! editing `health` doesn't load the `admin create` flow.

#[allow(dead_code)] // template kept as docs; consumers wire their own
mod admin_create;
mod health;
mod openapi;
mod prefs;

pub use health::Health;
pub use openapi::OpenApi;
pub use prefs::{run_with as run_prefs_with, Prefs};
