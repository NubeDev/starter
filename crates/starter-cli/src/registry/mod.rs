//! `Command` trait + `CommandRegistry`.

mod command;
mod command_registry;

pub use command::{Command, CommandError};
pub use command_registry::CommandRegistry;
