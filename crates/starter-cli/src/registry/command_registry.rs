//! `CommandRegistry` — collects [`super::Command`] impls and runs
//! the parsed subcommand.

use std::collections::HashMap;
use std::sync::Arc;

use super::command::{Command, CommandError};

/// Consumer-driven registry. Add starter commands with
/// [`Self::register_starter_defaults`]; add their own with
/// [`Self::register`].
#[derive(Default)]
pub struct CommandRegistry {
    commands: HashMap<&'static str, Arc<dyn Command>>,
}

impl CommandRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one command.
    pub fn register<C: Command>(mut self, cmd: C) -> Self {
        self.commands.insert(cmd.name(), Arc::new(cmd));
        self
    }

    /// Register the starter-shipped defaults (`health`, `openapi`,
    /// `admin create`, …).
    pub fn register_starter_defaults(self) -> Self {
        // TODO(ap): register `commands::health::Health`, etc., once
        // each command's body lands.
        self
    }

    /// Run the parsed clap matches against the registered commands.
    pub async fn dispatch(&self, _matches: &clap::ArgMatches) -> Result<(), CommandError> {
        // TODO(ap): look up by subcommand name and call `run`.
        Ok(())
    }
}
