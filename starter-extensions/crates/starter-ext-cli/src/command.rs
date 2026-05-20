//! [`ExtensionSubcommand`] — one `starter_cli::Command` per
//! `contributes.cli` entry.
//!
//! Each instance carries:
//!
//! - the extension's id + the cli entry's id (the dispatcher routes by
//!   the pair),
//! - the manifest's `command:` name (the verb the user types),
//! - a pre-built [`clap::Command`] surface — args derived from the
//!   manifest's `args_schema`,
//! - the configured streaming mode (`none` vs `stdout`),
//! - the configured request timeout,
//! - an `Arc<dyn CliDispatcher>` shared with every other entry.
//!
//! The same instance handles SIGINT: at run time the adapter installs
//! a one-shot `tokio::signal::ctrl_c` watcher that fires the
//! streaming-response's [`crate::CancelHandle`]. A second `Ctrl-C`
//! tears down the process immediately.

use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use clap::{Arg, ArgAction, ArgMatches};
use futures::StreamExt;
use serde_json::Value;
use starter_cli::registry::CommandError;
use starter_cli::Command;
use starter_ext_spi::{CliStreaming, ExtensionId};

use crate::dispatcher::{CliDispatcher, DispatchError, StreamResponse};

/// One CLI subcommand surfaced from a `contributes.cli` entry.
pub struct ExtensionSubcommand {
    extension: ExtensionId,
    contribute_id: String,
    /// The verb on the command line. `'static` because
    /// [`starter_cli::Command::name`] requires it; the adapter leaks
    /// the manifest-supplied string into a static once at build time.
    command_name: &'static str,
    description: String,
    args_schema: Value,
    streaming: CliStreaming,
    request_timeout: Duration,
    dispatcher: Arc<dyn CliDispatcher>,
}

impl ExtensionSubcommand {
    /// Build a subcommand for one manifest entry.
    ///
    /// `command_name` must be a `'static str` (the adapter leaks the
    /// manifest's owned `String` once at build time via
    /// [`Box::leak`]) so it matches the trait surface; CLI commands
    /// are registered exactly once at host startup, so leaking is the
    /// right shape.
    pub fn new(
        extension: ExtensionId,
        contribute_id: String,
        command_name: &'static str,
        description: String,
        args_schema: Value,
        streaming: CliStreaming,
        request_timeout: Duration,
        dispatcher: Arc<dyn CliDispatcher>,
    ) -> Self {
        Self {
            extension,
            contribute_id,
            command_name,
            description,
            args_schema,
            streaming,
            request_timeout,
            dispatcher,
        }
    }

    /// Parse the args_schema's top-level `properties:` into a flat
    /// list of clap [`Arg`]s. Top-level `required: [..]` becomes
    /// `Arg::required(true)`. Anything more complex (oneOf, nested
    /// schemas, …) is exposed as `--input <JSON>` instead — the
    /// extension still sees the parsed JSON object on dispatch.
    fn args_from_schema(&self) -> Vec<Arg> {
        let mut args: Vec<Arg> = Vec::new();
        if let Some(obj) = self.args_schema.as_object() {
            let required: Vec<String> = obj
                .get("required")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            if let Some(props) = obj.get("properties").and_then(Value::as_object) {
                for (name, prop) in props {
                    let ty = prop
                        .as_object()
                        .and_then(|p| p.get("type"))
                        .and_then(Value::as_str)
                        .unwrap_or("string");
                    let desc = prop
                        .as_object()
                        .and_then(|p| p.get("description"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let mut arg = Arg::new(name.to_owned())
                        .long(name.to_owned())
                        .required(required.iter().any(|r| r == name))
                        .help(desc.to_owned());
                    // Boolean schemas become flags; everything else is
                    // `--name <value>`. Numbers and integers still take
                    // a string and the dispatcher parses — keeps the
                    // adapter's clap surface small.
                    if ty == "boolean" {
                        arg = arg.action(ArgAction::SetTrue);
                    } else {
                        arg = arg.action(ArgAction::Set);
                    }
                    args.push(arg);
                }
            }
        }
        // Universal escape hatch: `--input <JSON>` overrides every
        // individual flag with the raw JSON the extension expects. The
        // ergonomic flags are a convenience over the same input.
        args.push(
            Arg::new("input")
                .long("input")
                .value_name("JSON")
                .action(ArgAction::Set)
                .help("Override per-flag args with one raw JSON object."),
        );
        // Adapter-owned `--timeout` knob; applied on top of the
        // dispatcher's default. Honoured by every flavour.
        args.push(
            Arg::new("timeout-ms")
                .long("timeout-ms")
                .value_name("MS")
                .action(ArgAction::Set)
                .help("Override the adapter's request timeout (milliseconds)."),
        );
        args
    }

    /// Collect the parsed clap matches into one JSON object the
    /// dispatcher passes to the handler. `--input` wins over per-flag
    /// values; otherwise each property in the schema becomes one key.
    fn matches_to_json(&self, matches: &ArgMatches) -> Result<Value, CommandError> {
        if let Some(raw) = matches.get_one::<String>("input") {
            return serde_json::from_str(raw)
                .map_err(|e| CommandError::UserFacing(format!("--input is not valid JSON: {e}")));
        }
        let mut out = serde_json::Map::new();
        if let Some(props) = self
            .args_schema
            .as_object()
            .and_then(|o| o.get("properties"))
            .and_then(Value::as_object)
        {
            for (name, prop) in props {
                let ty = prop
                    .as_object()
                    .and_then(|p| p.get("type"))
                    .and_then(Value::as_str)
                    .unwrap_or("string");
                if ty == "boolean" {
                    if matches.get_flag(name) {
                        out.insert(name.clone(), Value::Bool(true));
                    }
                } else if let Some(v) = matches.get_one::<String>(name) {
                    let coerced = match ty {
                        "integer" => v.parse::<i64>().map(Value::from).map_err(|_| {
                            CommandError::UserFacing(format!("--{name} not an integer"))
                        })?,
                        "number" => v
                            .parse::<f64>()
                            .map(|f| {
                                serde_json::Number::from_f64(f)
                                    .map(Value::Number)
                                    .unwrap_or(Value::Null)
                            })
                            .map_err(|_| {
                                CommandError::UserFacing(format!("--{name} not a number"))
                            })?,
                        _ => Value::String(v.to_owned()),
                    };
                    out.insert(name.clone(), coerced);
                }
            }
        }
        Ok(Value::Object(out))
    }

    fn resolved_timeout(&self, matches: &ArgMatches) -> Duration {
        matches
            .get_one::<String>("timeout-ms")
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_millis)
            .unwrap_or(self.request_timeout)
    }
}

#[async_trait]
impl Command for ExtensionSubcommand {
    fn name(&self) -> &'static str {
        self.command_name
    }

    fn subcommand(&self) -> clap::Command {
        clap::Command::new(self.command_name)
            .about(self.description.clone())
            .args(self.args_from_schema())
    }

    async fn run(&self, matches: &ArgMatches) -> Result<(), CommandError> {
        let input = self.matches_to_json(matches)?;
        let timeout = self.resolved_timeout(matches);

        match self.streaming {
            CliStreaming::None => {
                let v = self
                    .dispatcher
                    .dispatch(&self.extension, &self.contribute_id, input, timeout)
                    .await
                    .map_err(dispatch_to_cmd_err)?;
                // Pretty-print the response for human consumption.
                // Pipelines that need stable framing should select
                // `streaming: stdout`.
                let pretty = serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string());
                println!("{pretty}");
                Ok(())
            }
            CliStreaming::Stdout => {
                let response = self
                    .dispatcher
                    .dispatch_stream(&self.extension, &self.contribute_id, input, timeout)
                    .await
                    .map_err(dispatch_to_cmd_err)?;
                run_streaming(response).await
            }
        }
    }
}

/// Translate a [`DispatchError`] into the `CommandError` shape the
/// `starter-cli` binary surfaces. Substrate failures bubble up as
/// `Other` so the binary's outer error handler can log them with a
/// backtrace; everything else is a user-facing message.
fn dispatch_to_cmd_err(e: DispatchError) -> CommandError {
    match e {
        DispatchError::Substrate(m) => {
            CommandError::Other(std::io::Error::new(std::io::ErrorKind::Other, m).into())
        }
        other => CommandError::UserFacing(other.to_string()),
    }
}

/// Pump a [`StreamResponse`] onto stdout one event per line. Installs
/// a `Ctrl-C` watcher that fires the response's [`crate::CancelHandle`]
/// on the first signal; a second signal exits the process immediately
/// (so a wedged extension can't trap the user).
async fn run_streaming(mut response: StreamResponse) -> Result<(), CommandError> {
    // We `take` the cancel out of the response so it can outlive
    // `response` itself for one branch of the select while the body
    // keeps the stream end. The `events` field still moves into the
    // streaming loop below.
    let cancel = std::mem::replace(
        &mut response.cancel,
        crate::dispatcher::CancelHandle::noop(),
    );
    let cancel = Arc::new(cancel);

    let cancel_for_signal = cancel.clone();
    let signal_task = tokio::spawn(async move {
        // First Ctrl-C → fire the cancel handle (kernel maps to
        // `stream.cancel`). Second Ctrl-C → exit hard.
        if tokio::signal::ctrl_c().await.is_ok() {
            cancel_for_signal.fire();
        }
        if tokio::signal::ctrl_c().await.is_ok() {
            // Belt-and-braces hard exit; the stream loop should have
            // wound down by now but we don't want a wedged extension
            // to keep the binary alive.
            std::process::exit(130); // 128 + SIGINT
        }
    });

    let mut events = response.events;
    let stdout = std::io::stdout();
    let mut exit_err: Option<CommandError> = None;
    while let Some(item) = events.next().await {
        match item {
            Ok(ev) => {
                let line =
                    serde_json::to_string(&ev.payload).unwrap_or_else(|_| ev.payload.to_string());
                // Re-acquire the lock per event so the lock guard
                // never spans an `await`. Cheap; stdout is already
                // line-buffered on a tty.
                let mut handle = stdout.lock();
                if writeln!(handle, "{line}").is_err() {
                    // Pipe closed (e.g. `| head -n 1`). Treat as a
                    // user-initiated cancel and stop pumping.
                    cancel.fire();
                    break;
                }
                let _ = handle.flush();
            }
            Err(e) => {
                exit_err = Some(CommandError::UserFacing(format!("stream error: {e}")));
                break;
            }
        }
    }
    // Drop the rest of the response (and its cancel guard) and let
    // the signal task race to completion. `response.events` already
    // moved out; the remaining fields drop at end-of-scope.
    drop(events);
    signal_task.abort();
    let _ = response; // suppress unused warning; partial drop happens on return.
    match exit_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
