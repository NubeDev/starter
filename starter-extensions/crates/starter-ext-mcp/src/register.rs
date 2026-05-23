//! The single public entry point.
//!
//! Walks every `Validated` extension in the registry and, for each
//! `contributes.tools[]` entry, builds an [`ExtensionToolBinding`] and
//! adds it to the supplied [`starter_mcp::ToolRegistry`]. Returns the
//! mutated registry plus a [`RegisterOutcome`] summarising what wired
//! up — adapters in later phases will use the same shape so a
//! consumer's "loaded N tools across M extensions" log line is uniform.
//!
//! Errors during wiring (a missing `BuiltinTable` entry for an
//! extension whose manifest declared tools; a description / schema file
//! that fails to read) are aggregated into a single
//! [`RegisterError::Collected`] — the adapter does *not* short-circuit
//! on the first failure, matching the kernel's per-extension isolation
//! discipline.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use starter_ext_host::ExtensionRegistry;
use starter_ext_sdk::builtin::BuiltinTable;
use starter_ext_spi::{Error, ExtensionId, PermissionGate, RuntimeKind};
use starter_mcp::ToolRegistry;
use starter_spi::authz::PolicyEngine;

use crate::{
    authzed::AuthzedToolBinding,
    ctx_stub::make_stub_ctx,
    tool_wrapper::{ExtensionToolBinding, ProcessExtensionToolBinding},
};

/// Helper: wrap `tool` in an [`AuthzedToolBinding`] when both
/// `engine` and `gate` are present, otherwise return it untouched
/// boxed into the registry's `Arc<dyn Tool>` shape. Keeps the
/// "permission == None → zero overhead" property symmetric with
/// the REST adapter (SCOPE-EXT R15).
fn register_with_optional_gate<T>(
    tools: ToolRegistry,
    tool: T,
    engine: &Option<Arc<dyn PolicyEngine>>,
    gate: Option<&PermissionGate>,
) -> ToolRegistry
where
    T: starter_spi::tool::Tool + Send + Sync + 'static,
{
    match (engine, gate) {
        (Some(engine), Some(gate)) => {
            tools.register(AuthzedToolBinding::new(tool, engine.clone(), gate.clone()))
        }
        _ => tools.register(tool),
    }
}

/// Summary of what `register_tools` did. Surfaced so a consumer can log
/// the outcome without re-scanning the registry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RegisterOutcome {
    /// Number of extensions whose tools were considered (the count of
    /// `Validated` records in the registry).
    pub extensions_seen: usize,
    /// Number of tools successfully wired into `ToolRegistry`.
    pub tools_registered: usize,
    /// Number of tools that were skipped because the extension's flavour
    /// is not handled by this adapter (process / wasm — they get their
    /// own adapter wiring in later phases).
    pub tools_skipped_non_builtin: usize,
}

/// Per-tool wiring failure. Collected and surfaced through
/// [`RegisterError::Collected`].
#[derive(Debug, thiserror::Error)]
#[error("extension {extension:?}, tool {tool:?}: {source}")]
pub struct ToolBindingFailure {
    /// The owning extension id.
    pub extension: String,
    /// The failing tool id.
    pub tool: String,
    /// The underlying kernel error.
    #[source]
    pub source: Error,
}

/// Aggregate failure surfaced by [`register_tools`]. Holds one entry per
/// tool that failed to wire — analogous to `Loader::validate_all`'s
/// per-extension isolation but at the adapter level.
#[derive(Debug, thiserror::Error)]
pub enum RegisterError {
    /// One or more tool bindings failed to wire. The rest of the
    /// registry is *still* registered into the `ToolRegistry`; this
    /// error is informational. Phase 1 callers can choose to abort or
    /// continue; later adapter phases will likely surface this through
    /// `GET /extensions/<id>/events`.
    #[error("starter-ext-mcp: {} tool binding(s) failed to wire", .0.len())]
    Collected(Vec<ToolBindingFailure>),
}

/// Register every builtin extension's `contributes.tools[]` entries
/// into `tools`.
///
/// Returns the (now-populated) `ToolRegistry` plus a `RegisterOutcome`.
/// If any individual tool binding failed, the wire-able tools are still
/// registered and the failures are returned as an
/// `Err(RegisterError::Collected)` — analogous to how `Loader::commit`
/// isolates a bad bundle without bringing the rest down.
pub fn register_tools(
    registry: &ExtensionRegistry,
    builtins: &Arc<BuiltinTable>,
    tools: ToolRegistry,
) -> (ToolRegistry, RegisterOutcome, Result<(), RegisterError>) {
    register_tools_with_engine(registry, builtins, None, tools)
}

/// Same as [`register_tools`] but threads an optional
/// [`PolicyEngine`] through every tool whose manifest declared
/// `auth.permission`. SCOPE-EXT §5 (Phase 7d.2):
/// `engine.check((resource, action))` runs before the tool body;
/// a deny short-circuits with `Error::Forbidden` and lands in
/// `starter_authz_decisions` with `surface = "mcp"`.
pub fn register_tools_with_engine(
    registry: &ExtensionRegistry,
    builtins: &Arc<BuiltinTable>,
    engine: Option<Arc<dyn PolicyEngine>>,
    mut tools: ToolRegistry,
) -> (ToolRegistry, RegisterOutcome, Result<(), RegisterError>) {
    let mut outcome = RegisterOutcome::default();
    let mut failures: Vec<ToolBindingFailure> = Vec::new();

    for record in registry.iter_validated() {
        outcome.extensions_seen += 1;
        let Some(manifest) = record.manifest.as_ref() else {
            continue;
        };
        // Phase 1 only wires builtin-flavour extensions. Other flavours
        // get their own adapter wiring (supervisor for process, wasm
        // host for WASI components) in later phases.
        if manifest.runtime.kind != RuntimeKind::Builtin {
            outcome.tools_skipped_non_builtin += manifest.contributes.tools.len();
            continue;
        }
        let Some(extension_id) = record.id.clone() else {
            continue;
        };
        // The builtin must have called `register_static_table!` so the
        // dispatch table knows about its id. Adapter validation (R13):
        // missing entry is a wiring failure surfaced per-tool, not a
        // panic.
        if builtins.get(&extension_id).is_none() {
            for tool_entry in &manifest.contributes.tools {
                failures.push(ToolBindingFailure {
                    extension: extension_id.as_str().to_owned(),
                    tool: tool_entry.id.clone(),
                    source: Error::validation(format!(
                        "extension {:?} declares builtin runtime but its crate did not call \
                         `register_static_table!` (BuiltinTable lookup missed)",
                        extension_id.as_str()
                    )),
                });
            }
            continue;
        }

        let ctx = make_stub_ctx();
        for tool_entry in &manifest.contributes.tools {
            match ExtensionToolBinding::build(
                extension_id.clone(),
                &record.bundle_dir,
                tool_entry,
                builtins.clone(),
                ctx.clone(),
            ) {
                Ok(binding) => {
                    let gate = tool_entry.auth.permission.as_ref();
                    tools = register_with_optional_gate(tools, binding, &engine, gate);
                    outcome.tools_registered += 1;
                }
                Err(e) => failures.push(ToolBindingFailure {
                    extension: extension_id.as_str().to_owned(),
                    tool: tool_entry.id.clone(),
                    source: e,
                }),
            }
        }
    }

    let err = if failures.is_empty() {
        Ok(())
    } else {
        Err(RegisterError::Collected(failures))
    };
    (tools, outcome, err)
}

/// Register every process-flavour extension's `contributes.tools[]`
/// entries into `tools`, dispatching through their supervisor handles.
///
/// Companion to [`register_tools`] for the process flavour: walks every
/// `Validated` record whose manifest declares `runtime.kind: process`,
/// looks up its [`starter_ext_supervisor::SupervisorHandle`] in
/// `handles`, and wraps each declared tool in a
/// [`ProcessExtensionToolBinding`] that issues a JSON-RPC
/// `tools/<id>` request via [`starter_ext_supervisor::SupervisorHandle::call`]
/// on every MCP invocation.
///
/// Per-tool failures (description / schema file unreadable, handle
/// missing) are aggregated into [`RegisterError::Collected`] — the
/// adapter does *not* short-circuit on the first failure, matching the
/// kernel's per-extension isolation discipline.
pub fn register_process_tools(
    registry: &ExtensionRegistry,
    handles: &HashMap<ExtensionId, Arc<starter_ext_supervisor::SupervisorHandle>>,
    request_timeout: Duration,
    tools: ToolRegistry,
) -> (ToolRegistry, RegisterOutcome, Result<(), RegisterError>) {
    register_process_tools_with_engine(registry, handles, None, request_timeout, tools)
}

/// Process-flavour companion to [`register_tools_with_engine`].
pub fn register_process_tools_with_engine(
    registry: &ExtensionRegistry,
    handles: &HashMap<ExtensionId, Arc<starter_ext_supervisor::SupervisorHandle>>,
    engine: Option<Arc<dyn PolicyEngine>>,
    request_timeout: Duration,
    mut tools: ToolRegistry,
) -> (ToolRegistry, RegisterOutcome, Result<(), RegisterError>) {
    let mut outcome = RegisterOutcome::default();
    let mut failures: Vec<ToolBindingFailure> = Vec::new();

    for record in registry.iter_validated() {
        outcome.extensions_seen += 1;
        let Some(manifest) = record.manifest.as_ref() else {
            continue;
        };
        if manifest.runtime.kind != RuntimeKind::Process {
            // Builtin/wasm extensions are not this function's
            // responsibility — `register_tools` (builtin) and a future
            // `register_wasm_tools` cover those.
            outcome.tools_skipped_non_builtin += manifest.contributes.tools.len();
            continue;
        }
        let Some(extension_id) = record.id.clone() else {
            continue;
        };
        let Some(handle) = handles.get(&extension_id) else {
            for tool_entry in &manifest.contributes.tools {
                failures.push(ToolBindingFailure {
                    extension: extension_id.as_str().to_owned(),
                    tool: tool_entry.id.clone(),
                    source: Error::validation(format!(
                        "extension {:?} declares process runtime but no supervisor handle \
                         was supplied to register_process_tools",
                        extension_id.as_str()
                    )),
                });
            }
            continue;
        };

        for tool_entry in &manifest.contributes.tools {
            match ProcessExtensionToolBinding::build(
                extension_id.clone(),
                &record.bundle_dir,
                tool_entry,
                handle.clone(),
                request_timeout,
            ) {
                Ok(binding) => {
                    let gate = tool_entry.auth.permission.as_ref();
                    tools = register_with_optional_gate(tools, binding, &engine, gate);
                    outcome.tools_registered += 1;
                }
                Err(e) => failures.push(ToolBindingFailure {
                    extension: extension_id.as_str().to_owned(),
                    tool: tool_entry.id.clone(),
                    source: e,
                }),
            }
        }
    }

    let err = if failures.is_empty() {
        Ok(())
    } else {
        Err(RegisterError::Collected(failures))
    };
    (tools, outcome, err)
}
