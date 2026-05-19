//! `starter health` — hits `GET /health` and prints the result.

use async_trait::async_trait;
use clap::{ArgMatches, Command as ClapCommand};

use crate::registry::{Command, CommandError};

/// `health` subcommand.
pub struct Health;

#[async_trait]
impl Command for Health {
    fn name(&self) -> &'static str {
        "health"
    }

    fn subcommand(&self) -> ClapCommand {
        ClapCommand::new(self.name()).about("Check the server's /health endpoint")
    }

    async fn run(&self, _matches: &ArgMatches) -> Result<(), CommandError> {
        // TODO(ap): build a `starter_client_rs::Client` from a
        // base-url flag and print the JSON-decoded body.
        Ok(())
    }
}
