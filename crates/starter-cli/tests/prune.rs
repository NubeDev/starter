//! Integration tests for the `starter prune` subcommand.
//!
//! Uses a fake [`starter_changelog::Prune`] backend so the test
//! exercises arg parsing, request construction, and report
//! rendering without touching SQLite.

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use starter_changelog::{Prune as PruneBackend, PruneReport, PruneRequest};
use starter_cli::commands::{run_prune_with, Prune};
use starter_cli::{Command, CommandRegistry};
use starter_spi::Result;

#[derive(Default)]
struct FakePrune {
    calls: Mutex<Vec<PruneRequest>>,
    rows: u64,
}

impl FakePrune {
    fn new(rows: u64) -> Arc<Self> {
        Arc::new(Self {
            calls: Mutex::new(Vec::new()),
            rows,
        })
    }
}

#[async_trait]
impl PruneBackend for FakePrune {
    async fn prune(&self, req: &PruneRequest) -> Result<PruneReport> {
        self.calls.lock().unwrap().push(req.clone());
        Ok(PruneReport { rows: self.rows })
    }
}

fn root_command(registry: &CommandRegistry) -> clap::Command {
    clap::Command::new("starter").subcommands(registry.subcommands())
}

#[tokio::test]
async fn prune_dispatches_with_explicit_before() {
    let backend = FakePrune::new(7);
    let registry = CommandRegistry::new().register(Prune::new(backend.clone()));

    let matches = root_command(&registry).get_matches_from(vec![
        "starter",
        "prune",
        "--before",
        "2026-01-01T00:00:00Z",
        "--resource-kind",
        "note",
        "--dry-run",
        "--output",
        "json",
    ]);
    registry.dispatch(&matches).await.expect("dispatch ok");

    let calls = backend.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    let req = &calls[0];
    assert_eq!(req.before.to_rfc3339(), "2026-01-01T00:00:00+00:00");
    assert_eq!(req.resource_kind.as_deref(), Some("note"));
    assert!(req.dry_run);
}

#[tokio::test]
async fn prune_older_than_days_translates_to_recent_cutoff() {
    let backend = FakePrune::new(0);
    let cmd = Prune::new(backend.clone());

    // Build matches against the same clap surface the registry exposes.
    let registry = CommandRegistry::new().register(Prune::new(backend.clone()));
    let matches = root_command(&registry).get_matches_from(vec![
        "starter",
        "prune",
        "--older-than-days",
        "30",
    ]);
    let (_, sub) = matches.subcommand().unwrap();

    let before_call = Utc::now();
    let mut out = Vec::new();
    run_prune_with(&mut out, &*FakePrune::new(0), sub)
        .await
        .expect("run ok");
    // Re-dispatch through the registered command to keep the fake's
    // call log populated for the assertion below.
    cmd.run(sub).await.expect("run ok");

    let calls = backend.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    let req = &calls[0];
    let expected = before_call - Duration::days(30);
    // Allow a small window for clock drift between Utc::now() calls.
    let delta = (req.before - expected).num_seconds().abs();
    assert!(delta < 5, "cutoff drifted by {delta}s");
    assert!(!req.dry_run);
    assert!(req.resource_kind.is_none());

    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("deleted 0 rows"), "text output: {text}");
}

#[tokio::test]
async fn prune_requires_one_of_before_or_older_than_days() {
    let backend = FakePrune::new(0);
    let registry = CommandRegistry::new().register(Prune::new(backend));
    let matches = root_command(&registry).get_matches_from(vec!["starter", "prune"]);
    let err = registry.dispatch(&matches).await.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("--before") && msg.contains("--older-than-days"),
        "unexpected error: {msg}"
    );
}

#[tokio::test]
async fn prune_text_output_mentions_cutoff_and_count() {
    let backend = FakePrune::new(42);
    let registry = CommandRegistry::new().register(Prune::new(backend.clone()));
    let matches = root_command(&registry).get_matches_from(vec![
        "starter",
        "prune",
        "--before",
        "2025-06-01T00:00:00Z",
    ]);
    let (_, sub) = matches.subcommand().unwrap();

    let mut out = Vec::new();
    run_prune_with(&mut out, &*backend, sub).await.expect("run ok");
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("deleted 42 rows"), "text output: {text}");
    assert!(text.contains("2025-06-01"), "text output: {text}");
}
