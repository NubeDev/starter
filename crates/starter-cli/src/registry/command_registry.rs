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

    /// Register the starter-shipped defaults. Today this is `health`
    /// and `openapi`; both speak to a running starter-server via
    /// `starter-client-rs`. The `admin create` bootstrap path is
    /// deferred — it lands with the `starter-auth-users` crate body.
    pub fn register_starter_defaults(self) -> Self {
        use crate::commands::{Health, OpenApi, Prefs};
        self.register(Health).register(OpenApi).register(Prefs)
    }

    /// Iterate the registered clap subcommands in name order. The
    /// binary's `main()` calls this to attach them to its root
    /// `clap::Command`.
    pub fn subcommands(&self) -> Vec<clap::Command> {
        let mut names: Vec<&&'static str> = self.commands.keys().collect();
        names.sort();
        names
            .into_iter()
            .map(|n| self.commands[n].subcommand())
            .collect()
    }

    /// Run the parsed clap matches against the registered commands.
    ///
    /// Looks up the active subcommand by name; returns
    /// `CommandError::UserFacing` when no subcommand was given or the
    /// name is unknown.
    pub async fn dispatch(&self, matches: &clap::ArgMatches) -> Result<(), CommandError> {
        let (name, sub_matches) = matches
            .subcommand()
            .ok_or_else(|| CommandError::UserFacing("no subcommand given".into()))?;
        let cmd = self
            .commands
            .get(name)
            .ok_or_else(|| CommandError::UserFacing(format!("unknown subcommand: {name}")))?;
        cmd.run(sub_matches).await
    }
}
