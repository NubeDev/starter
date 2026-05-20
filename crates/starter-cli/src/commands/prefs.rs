//! `starter prefs` — inspect and mutate user/org preferences via
//! the four `/v1/{me,orgs,units}/…` REST endpoints. Owned by
//! SCOPE.md "API surface".
//!
//! Subcommands:
//!
//! - `prefs get [--org WS] [--output table|json]` — fetch the
//!   resolved view; prints a two-column table by default or pretty
//!   JSON when `--output json`.
//! - `prefs set --field FIELD --value VALUE [--org WS]` — PATCH a
//!   single field. The literal string `"auto"` is the CLI's
//!   inheritance sentinel and is translated to JSON `null` in the
//!   request body before send (SCOPE.md R3 "revert to inherit").
//! - `prefs units` — fetch the public unit registry and print one
//!   line per quantity.

use std::io::Write;

use async_trait::async_trait;
use clap::{Arg, ArgMatches, Command as ClapCommand};
use serde_json::{json, Value};
use starter_client_rs::Client;

use crate::registry::{Command, CommandError};

/// `prefs` subcommand.
pub struct Prefs;

const DEFAULT_BASE: &str = "http://localhost:8080";

#[async_trait]
impl Command for Prefs {
    fn name(&self) -> &'static str {
        "prefs"
    }

    fn subcommand(&self) -> ClapCommand {
        ClapCommand::new(self.name())
            .about("Inspect and mutate user/org preferences")
            .arg_required_else_help(true)
            .subcommand(
                ClapCommand::new("get")
                    .about("Print the resolved preferences view")
                    .arg(base_url_arg())
                    .arg(bearer_arg())
                    .arg(
                        Arg::new("org")
                            .long("org")
                            .help("Workspace id; defaults to the active workspace"),
                    )
                    .arg(
                        Arg::new("output")
                            .long("output")
                            .value_parser(["table", "json"])
                            .default_value("table")
                            .help("Output format"),
                    ),
            )
            .subcommand(
                ClapCommand::new("set")
                    .about("PATCH a single preference field")
                    .arg(base_url_arg())
                    .arg(bearer_arg())
                    .arg(
                        Arg::new("org")
                            .long("org")
                            .help("Workspace id; defaults to the active workspace"),
                    )
                    .arg(
                        Arg::new("field")
                            .long("field")
                            .required(true)
                            .help("Preference field name (e.g. temperature_unit)"),
                    )
                    .arg(
                        Arg::new("value")
                            .long("value")
                            .required(true)
                            .help("Value to set; the literal `auto` reverts to inherit"),
                    ),
            )
            .subcommand(
                ClapCommand::new("units")
                    .about("List the public unit registry")
                    .arg(base_url_arg()),
            )
    }

    async fn run(&self, matches: &ArgMatches) -> Result<(), CommandError> {
        let mut buf: Vec<u8> = Vec::new();
        run_with(&mut buf, matches).await?;
        // Flush captured bytes to real stdout in one shot. Using a
        // buffered Vec keeps `run_with`'s `Send` bound trivially
        // satisfied (StdoutLock is not Send).
        use std::io::Write as _;
        std::io::stdout()
            .write_all(&buf)
            .map_err(|e| CommandError::UserFacing(format!("write failed: {e}")))?;
        Ok(())
    }
}

/// Test seam: dispatch the parsed `prefs` matches to one of the
/// subcommand runners and write to `out` instead of stdout. The
/// integration tests for this crate call this directly so they can
/// assert on captured bytes.
pub async fn run_with<W: Write + Send>(
    out: &mut W,
    matches: &ArgMatches,
) -> Result<(), CommandError> {
    match matches.subcommand() {
        Some(("get", m)) => run_get(out, m).await,
        Some(("set", m)) => run_set(out, m).await,
        Some(("units", m)) => run_units(out, m).await,
        _ => Err(CommandError::UserFacing(
            "prefs requires a subcommand (get | set | units)".into(),
        )),
    }
}

fn base_url_arg() -> Arg {
    Arg::new("base-url")
        .long("base-url")
        .env("STARTER_BASE_URL")
        .default_value(DEFAULT_BASE)
}

fn bearer_arg() -> Arg {
    Arg::new("bearer")
        .long("bearer")
        .env("STARTER_BEARER")
        .help("Bearer token attached to authenticated calls")
}

fn build_client(matches: &ArgMatches) -> Result<Client, CommandError> {
    let base = matches
        .get_one::<String>("base-url")
        .map(String::as_str)
        .unwrap_or(DEFAULT_BASE);
    // `bearer` is only declared on the `get` and `set` subcommands;
    // on `units` it's absent (no auth needed). `try_get_one` returns
    // `Err` when the id isn't declared on this subcommand, which we
    // treat as "no bearer".
    let bearer = matches
        .try_get_one::<String>("bearer")
        .ok()
        .flatten()
        .cloned();
    Client::new(base.to_string(), bearer, None)
        .map_err(|e| CommandError::UserFacing(format!("client init failed: {e}")))
}

async fn run_get<W: Write>(out: &mut W, matches: &ArgMatches) -> Result<(), CommandError> {
    let client = build_client(matches)?;
    let org = matches.get_one::<String>("org").map(String::as_str);
    let resolved = client
        .get_my_preferences(org)
        .await
        .map_err(|e| CommandError::UserFacing(format!("request failed: {e}")))?;
    let output = matches
        .get_one::<String>("output")
        .map(String::as_str)
        .unwrap_or("table");
    if output == "json" {
        let body = serde_json::to_string_pretty(&resolved)
            .map_err(|e| CommandError::UserFacing(format!("serialize failed: {e}")))?;
        writeln!(out, "{body}").map_err(io_err)?;
    } else {
        print_resolved_table(out, &resolved)?;
    }
    Ok(())
}

async fn run_set<W: Write>(out: &mut W, matches: &ArgMatches) -> Result<(), CommandError> {
    let client = build_client(matches)?;
    let org = matches.get_one::<String>("org").map(String::as_str);
    let field = matches
        .get_one::<String>("field")
        .ok_or_else(|| CommandError::UserFacing("--field required".into()))?;
    let value = matches
        .get_one::<String>("value")
        .ok_or_else(|| CommandError::UserFacing("--value required".into()))?;

    // CLI sentinel: literal "auto" -> JSON null (revert to inherit).
    // Anything else is forwarded as a JSON string; the server's
    // typed deserialiser handles validation against the closed enum
    // / unit registries.
    let body_value: Value = if value == "auto" {
        Value::Null
    } else {
        Value::String(value.clone())
    };
    let body = json!({ field.as_str(): body_value });

    // `PreferencesPatch` collapses `null` and `missing` into the
    // same `Option::None`, so the typed entry point can't carry
    // "revert to inherit". Use the raw passthrough — the server's
    // PATCH handler reads the body as `serde_json::Value` and
    // distinguishes the two cases explicitly per the route docs.
    client
        .patch_my_preferences_raw(org, body)
        .await
        .map_err(|e| CommandError::UserFacing(format!("request failed: {e}")))?;
    writeln!(out, "ok").map_err(io_err)?;
    Ok(())
}

async fn run_units<W: Write>(out: &mut W, matches: &ArgMatches) -> Result<(), CommandError> {
    let client = build_client(matches)?;
    let units = client
        .get_units()
        .await
        .map_err(|e| CommandError::UserFacing(format!("request failed: {e}")))?;
    for q in &units.quantities {
        writeln!(
            out,
            "{:14}  canonical={:14}  allowed=[{}]",
            q.quantity,
            q.canonical,
            q.allowed.join(", ")
        )
        .map_err(io_err)?;
    }
    Ok(())
}

fn io_err(e: std::io::Error) -> CommandError {
    CommandError::UserFacing(format!("write failed: {e}"))
}

fn print_resolved_table<W: Write>(
    out: &mut W,
    resolved: &starter_spi::preferences::ResolvedPreferences,
) -> Result<(), CommandError> {
    // Round-trip through serde_json to render as a stable
    // key/value table without hand-listing every field. Field
    // order matches the ResolvedPreferences struct declaration.
    let value = serde_json::to_value(resolved)
        .map_err(|e| CommandError::UserFacing(format!("serialize failed: {e}")))?;
    let Value::Object(map) = value else {
        return Err(CommandError::UserFacing(
            "ResolvedPreferences did not serialise as an object".into(),
        ));
    };
    let width = map.keys().map(|k| k.len()).max().unwrap_or(0);
    for (k, v) in &map {
        let display = match v {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        writeln!(out, "{:<width$}  {}", k, display, width = width).map_err(io_err)?;
    }
    Ok(())
}
