//! Starter-shipped subcommands. One file per subcommand so an AI
//! editing `health` doesn't load the `admin create` flow.

mod admin_create;
mod health;
mod openapi;

pub use admin_create::AdminCreate;
pub use health::Health;
pub use openapi::OpenApi;
