//! Consumer-owned CLI commands. Implement `starter_cli::Command` and
//! register them into the same `CommandRegistry` that picks up
//! `starter`'s built-in `health` / `openapi`. Starter doesn't need to
//! know they exist — the registry is open.

use async_trait::async_trait;
use clap::{Arg, ArgMatches, Command as ClapCommand};
use serde_json::Value;
use starter_cli::registry::{Command, CommandError};

const DEFAULT_BASE: &str = "http://localhost:8080";

fn bearer_args(c: ClapCommand) -> ClapCommand {
    c.arg(
        Arg::new("base-url")
            .long("base-url")
            .env("NOTES_BASE_URL")
            .default_value(DEFAULT_BASE),
    )
    .arg(
        Arg::new("token")
            .long("token")
            .env("NOTES_TOKEN")
            .required(true)
            .help("Bearer token issued by `notes claim`"),
    )
}

fn http() -> reqwest::Client {
    reqwest::Client::builder().build().expect("build reqwest client")
}

/// `notes add "body"` — POSTs a new note.
pub struct NoteAdd;

#[async_trait]
impl Command for NoteAdd {
    fn name(&self) -> &'static str {
        "add"
    }

    fn subcommand(&self) -> ClapCommand {
        bearer_args(
            ClapCommand::new(self.name())
                .about("Create a note")
                .arg(Arg::new("body").required(true).help("Note body")),
        )
    }

    async fn run(&self, matches: &ArgMatches) -> Result<(), CommandError> {
        let base = matches.get_one::<String>("base-url").map(String::as_str).unwrap_or(DEFAULT_BASE);
        let token = matches.get_one::<String>("token").expect("clap requires token");
        let body = matches.get_one::<String>("body").expect("clap requires body");
        let res = http()
            .post(format!("{base}/notes"))
            .bearer_auth(token)
            .json(&serde_json::json!({ "body": body }))
            .send()
            .await
            .map_err(|e| CommandError::UserFacing(format!("request failed: {e}")))?;
        if !res.status().is_success() {
            return Err(CommandError::UserFacing(format!("server returned {}", res.status())));
        }
        let v: Value = res.json().await.map_err(|e| CommandError::UserFacing(e.to_string()))?;
        println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
        Ok(())
    }
}

/// `notes list` — GETs every note.
pub struct NoteList;

#[async_trait]
impl Command for NoteList {
    fn name(&self) -> &'static str {
        "list"
    }

    fn subcommand(&self) -> ClapCommand {
        bearer_args(ClapCommand::new(self.name()).about("List all notes"))
    }

    async fn run(&self, matches: &ArgMatches) -> Result<(), CommandError> {
        let base = matches.get_one::<String>("base-url").map(String::as_str).unwrap_or(DEFAULT_BASE);
        let token = matches.get_one::<String>("token").expect("clap requires token");
        let res = http()
            .get(format!("{base}/notes"))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| CommandError::UserFacing(format!("request failed: {e}")))?;
        if !res.status().is_success() {
            return Err(CommandError::UserFacing(format!("server returned {}", res.status())));
        }
        let v: Value = res.json().await.map_err(|e| CommandError::UserFacing(e.to_string()))?;
        println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
        Ok(())
    }
}
