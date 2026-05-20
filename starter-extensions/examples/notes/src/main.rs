//! `starter-notes` — host binary that loads the bundled `com.nube.notes`
//! extension via the real `starter-ext-host` loader and surfaces every
//! contribution into the corresponding adapter.
//!
//! Subcommands:
//!
//! - `serve` — bind axum on `127.0.0.1:8080`, mounting:
//!   - `POST /tools/<id>` and `POST/GET /notes` (REST adapter)
//!   - `GET  /extensions[/:id…]` admin slice (incl. UI bundle serving)
//! - `notes-add`, `notes-list` — extension-contributed CLI subcommands
//!   surfaced through `starter-ext-cli` into `starter-cli`'s standard
//!   `CommandRegistry`. Indistinguishable from `health` / `openapi` to
//!   the user.
//! - `health`, `openapi` — `starter-cli` defaults.
//!
//! **What's not here on purpose:**
//!
//! - No auth wiring. The example demonstrates the substrate; gating
//!   would use `with_principal` + `with_role` (per the `AuthGate`
//!   manifest field) — see `starter-server::auth`. Skipped to keep
//!   the binary small.
//! - No gRPC. `contributes.grpc` parses but no adapter ships in v0.1.
//! - No persistence. The store is process-global; restarting the
//!   server loses notes. Real products would put state behind a
//!   future `kv:` capability.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use clap::Command;
use starter_cli::CommandRegistry;
use starter_ext_cli::{
    build_cli_commands, BuiltinCliDispatcher, BuiltinCliRegistry, DEFAULT_REQUEST_TIMEOUT,
};
use starter_ext_host::{ExtensionRegistry, Loader};
use starter_ext_mcp::register_tools;
use starter_ext_server::{
    rest_router, router as admin_router, BuiltinRestDispatcher, ExtensionAdmin, RestRouterOptions,
};
use starter_ext_spi::ExtensionId;
use starter_mcp::ToolRegistry;

const NOTES_EXT_ID: &str = "com.nube.notes";
const DEFAULT_BIND: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> std::process::ExitCode {
    // ---- 1. Discover + validate the bundled extension -------------------
    let bundle_root = match locate_bundle_root() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("starter-notes: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let records = Loader::scan(&bundle_root).validate_all();
    let mut registry = ExtensionRegistry::new();
    let outcome = Loader::commit(records, &mut registry);
    registry.seal();
    if outcome.validated == 0 {
        eprintln!(
            "starter-notes: no extensions validated under {} (failed: {})",
            bundle_root.display(),
            outcome.failed
        );
        return std::process::ExitCode::FAILURE;
    }
    let registry = Arc::new(registry);
    tracing::info!(
        validated = outcome.validated,
        failed = outcome.failed,
        "extensions loaded"
    );

    // ---- 2. Build the cross-surface BuiltinTable -------------------------
    // One closure handles every contribute id (tools + REST) — see
    // `starter_notes_ext::build_builtin_table`. Wrapped in `Arc` for
    // sharing with both the MCP and REST adapters.
    let builtins = Arc::new(starter_notes_ext::build_builtin_table());

    // ---- 3. MCP tools ----------------------------------------------------
    let (tools, mcp_outcome, mcp_err) = register_tools(&registry, &builtins, ToolRegistry::new());
    if let Err(e) = mcp_err {
        eprintln!("starter-notes: mcp register: {e}");
        return std::process::ExitCode::FAILURE;
    }
    tracing::info!(registered = mcp_outcome.tools_registered, "mcp tools wired");
    let _tools = tools; // The example's `serve` doesn't expose MCP over
                        // HTTP yet — wiring `starter-mcp`'s transport
                        // would mean adding `starter-mcp-http` to the
                        // deps. Tools are loaded and queryable; serving
                        // them is left as the next slice.

    // ---- 4. CLI registry -------------------------------------------------
    let ext_id = ExtensionId::new(NOTES_EXT_ID).expect("known id");
    let cli_registry = BuiltinCliRegistry::new()
        .register(
            ext_id.clone(),
            "com.nube.notes.cli_add",
            starter_notes_ext::cli_add,
        )
        .register(
            ext_id.clone(),
            "com.nube.notes.cli_list",
            starter_notes_ext::cli_list,
        );
    let cli_dispatcher = Arc::new(BuiltinCliDispatcher::new(Arc::new(cli_registry)));
    let ext_cmds = match build_cli_commands(&registry, cli_dispatcher, DEFAULT_REQUEST_TIMEOUT) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("starter-notes: cli build: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let mut cmd_registry = CommandRegistry::new().register_starter_defaults();
    for c in ext_cmds {
        cmd_registry = cmd_registry.register(c);
    }

    // ---- 5. Top-level clap surface ---------------------------------------
    let root = Command::new("starter-notes")
        .about("Notes demo — one extension, four surfaces (MCP/REST/CLI/UI), loaded by the real starter-ext-host loader.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("serve").about(format!(
            "Bind axum on {DEFAULT_BIND} and mount the REST + admin/UI routers"
        )))
        .subcommands(cmd_registry.subcommands());

    let matches = root.get_matches();
    match matches.subcommand() {
        Some(("serve", _)) => match run_serve(registry, builtins).await {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("starter-notes: serve: {e}");
                std::process::ExitCode::FAILURE
            }
        },
        _ => match cmd_registry.dispatch(&matches).await {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("starter-notes: {e}");
                std::process::ExitCode::FAILURE
            }
        },
    }
}

/// Bind axum on the configured address; merge the REST adapter and the
/// admin slice (which serves `/extensions/:id/ui/*` for the UI panel).
async fn run_serve(
    registry: Arc<ExtensionRegistry>,
    builtins: Arc<starter_ext_sdk::builtin::BuiltinTable>,
) -> Result<(), String> {
    let rest_dispatcher = Arc::new(BuiltinRestDispatcher::new(builtins, registry.clone()));
    let rest = rest_router::<()>(
        registry.clone(),
        rest_dispatcher,
        RestRouterOptions::default(),
    )
    .map_err(|e| format!("rest router: {e}"))?;
    let admin = ExtensionAdmin::builder(registry).build();
    let admin_rt: Router<()> = admin_router(admin);

    let app = Router::new().merge(rest).merge(admin_rt);

    let addr: SocketAddr = DEFAULT_BIND.parse().expect("DEFAULT_BIND parses");
    tracing::info!(%addr, "starter-notes listening");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("bind {addr}: {e}"))?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("ctrl-c received, shutting down");
        })
        .await
        .map_err(|e| format!("serve: {e}"))
}

/// CARGO_MANIFEST_DIR is `examples/notes/`; pointing the loader at its
/// parent (`examples/`) lets it discover this bundle alongside any
/// sibling examples. Per-bundle `block.yaml` failures are isolated, so
/// extra siblings are noise — not breakage.
fn locate_bundle_root() -> Result<PathBuf, String> {
    let dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(dir)
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "examples/ parent missing".into())
}
