//! `starter openapi` — fetch the server's OpenAPI document.
//! Useful for `pnpm codegen` workflows that need the doc on disk.

use async_trait::async_trait;
use clap::{Arg, ArgMatches, Command as ClapCommand};

use crate::registry::{Command, CommandError};

/// `openapi` subcommand.
pub struct OpenApi;

#[async_trait]
impl Command for OpenApi {
    fn name(&self) -> &'static str {
        "openapi"
    }

    fn subcommand(&self) -> ClapCommand {
        ClapCommand::new(self.name())
            .about("Fetch the server's OpenAPI document and print to stdout")
            .arg(Arg::new("base-url").long("base-url").env("STARTER_BASE_URL"))
    }

    async fn run(&self, _matches: &ArgMatches) -> Result<(), CommandError> {
        // TODO(ap): fetch via starter_client_rs::Client::openapi
        // and pretty-print to stdout.
        Ok(())
    }
}
