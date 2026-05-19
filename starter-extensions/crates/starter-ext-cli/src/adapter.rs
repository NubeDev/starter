//! [`build_cli_commands`] — walks the registry, returns one
//! [`ExtensionSubcommand`] per `contributes.cli` entry across every
//! `Validated` extension.
//!
//! The consumer's `main()` then registers each one on its
//! [`starter_cli::CommandRegistry`]:
//!
//! ```ignore
//! let mut registry = CommandRegistry::new().register_starter_defaults();
//! for cmd in build_cli_commands(&host_registry, dispatcher, default_timeout)? {
//!     registry = registry.register(cmd);
//! }
//! ```
//!
//! Build-time concerns the adapter owns:
//!
//! - **Command-name collisions** between two extensions (or between an
//!   extension and a starter-shipped command-name reservation) are
//!   returned as [`BuildCliError::Collision`] before any command is
//!   produced — the operator sees one diagnostic with both ids
//!   instead of clap's "subcommand defined more than once" panic at
//!   `get_matches()` time.
//! - **Schema I/O failures** propagate as [`BuildCliError::SchemaIo`] /
//!   `SchemaJson`. Adapters stop at the first failing entry so the
//!   operator sees one root cause.
//! - **Manifest-relative paths** (`args_schema:`,
//!   `description_file:`) are resolved against each record's
//!   `bundle_dir` exactly like the REST adapter does.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use starter_ext_host::ExtensionRegistry;
use starter_ext_spi::{ContributeCli, ExtensionId, LifecycleState};

use crate::command::ExtensionSubcommand;
use crate::dispatcher::CliDispatcher;

/// Errors raised at adapter build time.
#[derive(Debug, thiserror::Error)]
pub enum BuildCliError {
    /// Two `contributes.cli` entries (in possibly different extensions)
    /// declared the same `command:` verb. The adapter never silently
    /// shadows; both ids surface in the diagnostic.
    #[error("command-name collision on `{command}` between {first:?} and {second:?}")]
    Collision {
        /// The colliding verb.
        command: String,
        /// First registrant ("<extension>:<contribute_id>").
        first: String,
        /// Second registrant ("<extension>:<contribute_id>").
        second: String,
    },

    /// `args_schema:` could not be read off disk.
    #[error("entry {entry:?}: reading args_schema {path:?}: {source}")]
    SchemaIo {
        /// "<extension>:<contribute_id>"
        entry: String,
        /// Manifest-relative schema path.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// `args_schema:` did not parse as JSON.
    #[error("entry {entry:?}: args_schema {path:?} is not valid JSON: {source}")]
    SchemaJson {
        /// "<extension>:<contribute_id>"
        entry: String,
        /// Manifest-relative schema path.
        path: String,
        /// Underlying parse error.
        #[source]
        source: serde_json::Error,
    },

    /// `description_file:` could not be read.
    #[error("entry {entry:?}: reading description_file {path:?}: {source}")]
    DescriptionIo {
        /// "<extension>:<contribute_id>"
        entry: String,
        /// Manifest-relative description path.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// Build one [`ExtensionSubcommand`] per `contributes.cli` entry.
///
/// `default_timeout` is the per-call timeout the adapter passes into
/// every dispatch unless the user overrides it via `--timeout-ms`. The
/// dispatcher is shared across every produced command (an `Arc<dyn …>`
/// makes that cheap).
pub fn build_cli_commands(
    registry: &ExtensionRegistry,
    dispatcher: Arc<dyn CliDispatcher>,
    default_timeout: Duration,
) -> Result<Vec<ExtensionSubcommand>, BuildCliError> {
    // First pass: detect collisions before we build anything so a
    // partial set of commands never leaks back to the caller.
    let mut planned: HashMap<String, String> = HashMap::new();
    let mut entries: Vec<(ExtensionId, std::path::PathBuf, ContributeCli)> = Vec::new();
    for record in registry.iter_validated() {
        let Some(manifest) = record.manifest.as_ref() else {
            continue;
        };
        if record.state != LifecycleState::Validated {
            continue;
        }
        let Some(extension_id) = record.id.as_ref() else {
            continue;
        };
        for cli in &manifest.contributes.cli {
            let entry_label = format!("{}:{}", extension_id.as_str(), cli.id);
            if let Some(prev) = planned.get(&cli.command) {
                return Err(BuildCliError::Collision {
                    command: cli.command.clone(),
                    first: prev.clone(),
                    second: entry_label,
                });
            }
            planned.insert(cli.command.clone(), entry_label);
            entries.push((extension_id.clone(), record.bundle_dir.clone(), cli.clone()));
        }
    }

    // Second pass: resolve schemas + descriptions and build the
    // commands.
    let mut out: Vec<ExtensionSubcommand> = Vec::with_capacity(entries.len());
    for (ext_id, bundle, cli) in entries {
        let entry_label = format!("{}:{}", ext_id.as_str(), cli.id);
        let schema_path = bundle.join(&cli.args_schema);
        let schema_bytes = std::fs::read(&schema_path).map_err(|source| {
            BuildCliError::SchemaIo {
                entry: entry_label.clone(),
                path: cli.args_schema.clone(),
                source,
            }
        })?;
        let schema: serde_json::Value =
            serde_json::from_slice(&schema_bytes).map_err(|source| BuildCliError::SchemaJson {
                entry: entry_label.clone(),
                path: cli.args_schema.clone(),
                source,
            })?;

        let desc_path = bundle.join(&cli.description_file);
        let description = std::fs::read_to_string(&desc_path).map_err(|source| {
            BuildCliError::DescriptionIo {
                entry: entry_label.clone(),
                path: cli.description_file.clone(),
                source,
            }
        })?;
        // Use the first non-empty line as the clap `about:` blurb;
        // the long help can come from the manifest's full file via
        // a future `long_about:` wiring.
        let about = description
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("")
            .trim()
            .to_owned();

        // `starter_cli::Command::name` returns `&'static str`. Leak
        // the manifest's owned `command` once — every CLI command is
        // registered exactly once at host startup, so the leak is
        // bounded by the number of contributed CLI verbs.
        let command_name: &'static str = Box::leak(cli.command.clone().into_boxed_str());

        out.push(ExtensionSubcommand::new(
            ext_id,
            cli.id,
            command_name,
            about,
            schema,
            cli.streaming,
            default_timeout,
            dispatcher.clone(),
        ));
    }
    Ok(out)
}
