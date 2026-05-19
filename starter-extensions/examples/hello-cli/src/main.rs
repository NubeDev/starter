//! Host-side wiring for the `hello-cli` example.
//!
//! Demonstrates the SCOPE.md "Phase 6" smoke shape: a single
//! `contributes.cli` entry surfaced through the CLI adapter into the
//! same [`starter_cli::CommandRegistry`] that hosts the
//! `starter`-shipped defaults (`health`, `openapi`, …). From the
//! user's perspective the extension's `hellocli-greet` /
//! `hellocli-tick` verbs are indistinguishable from the built-ins.
//!
//! The binary works against a vendored bundle inside the example
//! directory itself; an operator's binary would instead point
//! `Loader::scan` at the configured extensions root (see
//! `starter-config`'s `EXTENSIONS_DIR` resolution).

use std::path::PathBuf;
use std::sync::Arc;

use clap::Command;
use starter_cli::CommandRegistry;
use starter_ext_cli::{
    build_cli_commands, BuiltinCliDispatcher, BuiltinCliRegistry, DEFAULT_REQUEST_TIMEOUT,
};
use starter_ext_host::{ExtensionRegistry, Loader};
use starter_ext_spi::ExtensionId;

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    let bundle_root = match locate_bundle_root() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("hello-cli: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };

    // 1. Load + validate the extension bundle from disk.
    let records = Loader::scan(&bundle_root).validate_all();
    let mut registry = ExtensionRegistry::new();
    let outcome = Loader::commit(records, &mut registry);
    registry.seal();
    if outcome.validated == 0 {
        eprintln!(
            "hello-cli: no extensions validated under {} (failed: {})",
            bundle_root.display(),
            outcome.failed
        );
        return std::process::ExitCode::FAILURE;
    }

    // 2. Build the per-extension cli-handler registry (the proc-macro's
    //    dispatch path covers `contributes.tools`; cli handlers
    //    register here separately in v0.1).
    let ext_id = ExtensionId::new("com.acme.hellocli").expect("known id parses");
    let cli_registry = BuiltinCliRegistry::new()
        .register(ext_id.clone(), "com.acme.hellocli.greet", hello_cli::greet)
        .register_streaming(ext_id.clone(), "com.acme.hellocli.tick", hello_cli::tick);
    let dispatcher = Arc::new(BuiltinCliDispatcher::new(Arc::new(cli_registry)));

    // 3. Surface every contributes.cli entry as a clap subcommand
    //    inside the standard starter CommandRegistry.
    let ext_cmds = match build_cli_commands(&registry, dispatcher, DEFAULT_REQUEST_TIMEOUT) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("hello-cli: cli adapter build failed: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let mut cmd_registry = CommandRegistry::new().register_starter_defaults();
    for c in ext_cmds {
        cmd_registry = cmd_registry.register(c);
    }

    // 4. Standard clap dispatch loop.
    let root = Command::new("hello-cli")
        .about("Demo binary mixing starter-shipped and extension-contributed subcommands.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommands(cmd_registry.subcommands());
    let matches = root.get_matches();
    match cmd_registry.dispatch(&matches).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("hello-cli: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// Locate the directory of validated bundles. The example ships its own
/// bundle alongside the binary; an operator's binary would point at
/// `$XDG_DATA_HOME/<binary>/extensions/` per SCOPE 0's bundle path
/// decision.
fn locate_bundle_root() -> Result<PathBuf, String> {
    // CARGO_MANIFEST_DIR is `examples/hello-cli/`; the loader wants a
    // parent that contains one subdir *per* extension. Stage the
    // example into a tempdir-style layout by pointing at the
    // example's parent and filtering to the single known dir.
    //
    // To keep the example self-contained without a tempdir, we treat
    // CARGO_MANIFEST_DIR's parent (`examples/`) as the bundle root —
    // the loader will see `hello-builtin/`, `hello-cli/`, etc., and
    // skip the ones that fail to parse for unrelated reasons.
    // Operators would instead point at the configured root.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Ok(PathBuf::from(manifest_dir)
        .parent()
        .ok_or("examples/ parent missing")?
        .to_path_buf())
}
