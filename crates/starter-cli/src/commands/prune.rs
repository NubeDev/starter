//! `starter prune` — drive a [`starter_changelog::Prune`] backend
//! from the CLI.
//!
//! SCOPE §"Open questions" item 2: starter does not auto-TTL.
//! Consumers schedule this subcommand (cron, k8s `CronJob`, …) and
//! supply the concrete backend at registration time:
//!
//! ```ignore
//! use std::sync::Arc;
//! use starter_cli::{commands::Prune, CommandRegistry};
//! # async fn build_backend() -> Arc<dyn starter_changelog::Prune> { unimplemented!() }
//! # async fn demo() {
//! let backend = build_backend().await;
//! let registry = CommandRegistry::new()
//!     .register_starter_defaults()
//!     .register(Prune::new(backend));
//! # let _ = registry; }
//! ```
//!
//! Args:
//!
//! - `--before <RFC3339>` — delete rows with `at < before`.
//! - `--older-than-days <N>` — convenience: `before = now - N days`.
//!   Mutually exclusive with `--before`; exactly one is required.
//! - `--resource-kind <kind>` — narrow to one `resource_kind`.
//! - `--dry-run` — report matches without deleting.
//! - `--output <text|json>` — default `text`.
//!
//! R3: the subcommand is transport — it parses, calls
//! [`starter_changelog::Prune::prune`], and prints. No retention
//! policy lives here.

use std::io::Write;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use clap::{Arg, ArgAction, ArgMatches, Command as ClapCommand};
use starter_changelog::{Prune as PruneBackend, PruneRequest};

use crate::registry::{Command, CommandError};

/// `prune` subcommand.
pub struct Prune {
    backend: Arc<dyn PruneBackend>,
}

impl Prune {
    /// Wrap a [`starter_changelog::Prune`] backend.
    pub fn new(backend: Arc<dyn PruneBackend>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl Command for Prune {
    fn name(&self) -> &'static str {
        "prune"
    }

    fn subcommand(&self) -> ClapCommand {
        ClapCommand::new(self.name())
            .about("Delete changelog rows older than a cutoff")
            .arg(
                Arg::new("before")
                    .long("before")
                    .value_name("RFC3339")
                    .help("Delete rows with at < BEFORE (e.g. 2026-01-01T00:00:00Z)"),
            )
            .arg(
                Arg::new("older-than-days")
                    .long("older-than-days")
                    .value_name("N")
                    .conflicts_with("before")
                    .value_parser(clap::value_parser!(i64).range(1..))
                    .help("Convenience: cutoff = now - N days"),
            )
            .arg(
                Arg::new("resource-kind")
                    .long("resource-kind")
                    .value_name("KIND")
                    .help("Narrow to a single resource_kind"),
            )
            .arg(
                Arg::new("dry-run")
                    .long("dry-run")
                    .action(ArgAction::SetTrue)
                    .help("Count matching rows but do not delete"),
            )
            .arg(
                Arg::new("output")
                    .long("output")
                    .value_parser(["text", "json"])
                    .default_value("text"),
            )
    }

    async fn run(&self, matches: &ArgMatches) -> Result<(), CommandError> {
        let mut buf: Vec<u8> = Vec::new();
        run_with(&mut buf, self.backend.as_ref(), matches).await?;
        std::io::stdout()
            .write_all(&buf)
            .map_err(|e| CommandError::UserFacing(format!("write failed: {e}")))?;
        Ok(())
    }
}

/// Test seam: parse matches, call the backend, write the report to
/// `out`. Integration tests dispatch through this directly with a
/// fake [`starter_changelog::Prune`] impl so they can assert on
/// captured bytes without spinning up SQLite.
pub async fn run_with<W: Write + Send>(
    out: &mut W,
    backend: &dyn PruneBackend,
    matches: &ArgMatches,
) -> Result<(), CommandError> {
    let req = parse_request(matches)?;
    let report = backend
        .prune(&req)
        .await
        .map_err(|e| CommandError::UserFacing(format!("prune failed: {e}")))?;

    let output = matches
        .get_one::<String>("output")
        .map(String::as_str)
        .unwrap_or("text");
    if output == "json" {
        let body = serde_json::json!({
            "rows": report.rows,
            "dry_run": req.dry_run,
            "before": req.before.to_rfc3339(),
            "resource_kind": req.resource_kind,
        });
        writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(&body).expect("json")
        )
        .map_err(io_err)?;
    } else {
        let verb = if req.dry_run {
            "would delete"
        } else {
            "deleted"
        };
        let kind = req
            .resource_kind
            .as_deref()
            .map(|k| format!(" (kind={k})"))
            .unwrap_or_default();
        writeln!(
            out,
            "{verb} {} rows older than {}{kind}",
            report.rows,
            req.before.to_rfc3339(),
        )
        .map_err(io_err)?;
    }
    Ok(())
}

fn parse_request(matches: &ArgMatches) -> Result<PruneRequest, CommandError> {
    let before = match (
        matches.get_one::<String>("before"),
        matches.get_one::<i64>("older-than-days"),
    ) {
        (Some(s), None) => DateTime::parse_from_rfc3339(s)
            .map_err(|e| {
                CommandError::UserFacing(format!("--before is not a valid RFC3339 datetime: {e}"))
            })?
            .with_timezone(&Utc),
        (None, Some(days)) => Utc::now() - Duration::days(*days),
        (Some(_), Some(_)) => {
            // clap's `conflicts_with` should already block this, but
            // belt-and-braces — keep the error surface stable.
            return Err(CommandError::UserFacing(
                "--before and --older-than-days are mutually exclusive".into(),
            ));
        }
        (None, None) => {
            return Err(CommandError::UserFacing(
                "one of --before or --older-than-days is required".into(),
            ));
        }
    };
    Ok(PruneRequest {
        before,
        resource_kind: matches.get_one::<String>("resource-kind").cloned(),
        dry_run: matches.get_flag("dry-run"),
    })
}

fn io_err(e: std::io::Error) -> CommandError {
    CommandError::UserFacing(format!("write failed: {e}"))
}
