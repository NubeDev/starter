//! The `Command` trait. Each registered command supplies its own
//! clap subcommand and runs against a shared context.

use async_trait::async_trait;
use clap::ArgMatches;

/// A registered CLI subcommand.
///
/// Implementors define their clap surface in [`Self::subcommand`] and
/// their behaviour in [`Self::run`]. The registry handles parsing
/// + dispatch.
#[async_trait]
pub trait Command: Send + Sync + 'static {
    /// Subcommand name. Becomes the verb on the command line
    /// (`starter <name> …`).
    fn name(&self) -> &'static str;

    /// Build the clap subcommand surface (args, help text, etc.).
    fn subcommand(&self) -> clap::Command;

    /// Execute the parsed subcommand.
    async fn run(&self, matches: &ArgMatches) -> Result<(), CommandError>;
}

/// Failures bubbled up by a `Command::run`.
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    /// The command's preconditions weren't met (file missing,
    /// server unreachable, etc.).
    #[error("{0}")]
    UserFacing(String),

    /// Anything else.
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
}
