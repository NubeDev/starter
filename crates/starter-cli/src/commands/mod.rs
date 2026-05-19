//! Starter-shipped subcommands. One file per subcommand so an AI
//! editing `health` doesn't load the `admin create` flow.

#[allow(dead_code)] // template kept as docs; consumers wire their own
mod admin_create;
mod health;
mod openapi;

pub use health::Health;
pub use openapi::OpenApi;
