//! `starter openapi` — fetch the server's OpenAPI document.
//! Useful for `pnpm codegen` workflows that need the doc on disk.

use async_trait::async_trait;
use clap::{Arg, ArgMatches, Command as ClapCommand};
use starter_client_rs::Client;

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
            .arg(
                Arg::new("base-url")
                    .long("base-url")
                    .env("STARTER_BASE_URL")
                    .default_value("http://localhost:8080"),
            )
    }

    async fn run(&self, matches: &ArgMatches) -> Result<(), CommandError> {
        let base = matches
            .get_one::<String>("base-url")
            .map(String::as_str)
            .unwrap_or("http://localhost:8080");
        let client = Client::new(base.to_string(), None, None)
            .map_err(|e| CommandError::UserFacing(format!("client init failed: {e}")))?;
        let doc = client
            .openapi()
            .await
            .map_err(|e| CommandError::UserFacing(format!("request failed: {e}")))?;
        let body = serde_json::to_string_pretty(&doc)
            .map_err(|e| CommandError::UserFacing(format!("serialize failed: {e}")))?;
        println!("{body}");
        Ok(())
    }
}
