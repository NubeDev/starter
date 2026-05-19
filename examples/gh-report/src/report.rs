//! `report` subcommand — the domain-specific entry point.
//!
//! Today this is a placeholder that prints a stubbed JSON body so the
//! consumer-domain layout pattern is locked. To turn this into a real
//! tool: replace [`generate`] with an `octocrab`-backed call against
//! the GitHub API and surface the auth token via `SecretStore`.

use async_trait::async_trait;
use clap::{Arg, ArgMatches, Command as ClapCommand};
use serde_json::json;
use starter_cli::registry::{Command, CommandError};

/// `report` subcommand.
pub struct Report;

#[async_trait]
impl Command for Report {
    fn name(&self) -> &'static str {
        "report"
    }

    fn subcommand(&self) -> ClapCommand {
        ClapCommand::new(self.name())
            .about("Generate a GitHub activity report (skeleton).")
            .arg(
                Arg::new("repo")
                    .long("repo")
                    .required(true)
                    .help("owner/name slug, e.g. NubeDev/starter"),
            )
            .arg(
                Arg::new("since")
                    .long("since")
                    .help("ISO-8601 datetime to limit the report window"),
            )
    }

    async fn run(&self, matches: &ArgMatches) -> Result<(), CommandError> {
        let repo = matches
            .get_one::<String>("repo")
            .map(String::as_str)
            .ok_or_else(|| CommandError::UserFacing("missing --repo".into()))?;
        let since = matches.get_one::<String>("since").map(String::as_str);

        let body = generate(repo, since).await?;
        let pretty = serde_json::to_string_pretty(&body)
            .map_err(|e| CommandError::UserFacing(format!("serialize: {e}")))?;
        println!("{pretty}");
        Ok(())
    }
}

/// Stubbed report builder. Replace with a real GitHub-API call.
async fn generate(repo: &str, since: Option<&str>) -> Result<serde_json::Value, CommandError> {
    Ok(json!({
        "repo": repo,
        "since": since,
        "stub": true,
        "note": "Replace report::generate with an octocrab-backed implementation."
    }))
}
