//! The `Tool` impl that bridges an extension's `contributes.tools[]`
//! entry to `starter_mcp::ToolRegistry`.
//!
//! One `ExtensionToolBinding` per declared tool. The binding owns:
//!
//! - a clone of the extension's id (so the dispatch closure can address
//!   the right entry in the `BuiltinTable`),
//! - the `Arc<BuiltinEntry>` view into the host's static dispatch table,
//! - the tool's manifest entry's static metadata (description + input
//!   schema, read once at load time per R7),
//! - a `CtxInner` (Phase 1 stub) the dispatch closure threads through.

use std::sync::Arc;

use async_trait::async_trait;

use starter_ext_metrics::Counters;
use starter_ext_sdk::builtin::BuiltinTable;
use starter_ext_sdk::ctx::CtxInner;
use starter_ext_spi::{ContributeTool, Error, ExtensionId};
use starter_spi::tool::{Tool, ToolDefinition};

/// Bump `tool_calls_total` on entry to a tool invocation, then
/// `tool_errors_total` if the call returned an error. A no-op when no
/// [`Counters`] handle is wired (`counters == None`).
fn record_tool_invocation<T>(counters: &Option<Arc<Counters>>, result: &starter_spi::Result<T>) {
    if let Some(c) = counters {
        c.record_tool_call();
        if result.is_err() {
            c.record_tool_error();
        }
    }
}

/// One adapter-mounted tool. Implements [`starter_spi::tool::Tool`] so
/// `starter_mcp::ToolRegistry::register` can take it directly.
///
/// SCOPE R7 — the `description` and `input_schema` fields are populated
/// at load time from the static files the manifest names; they are
/// *not* templated and the extension cannot mutate them between calls.
pub struct ExtensionToolBinding {
    /// The owning extension's id. Used as the lookup key into the
    /// `BuiltinTable` on every call.
    pub extension_id: ExtensionId,
    /// The tool id as it appears in `block.yaml` and on the MCP wire.
    pub tool_id: String,
    /// Cached description bytes (read from `description_file`).
    pub description: String,
    /// Cached input schema (parsed from `input_schema`).
    pub input_schema: serde_json::Value,
    /// Shared dispatch table. Cheap to clone (Arc-backed inside).
    pub builtins: Arc<BuiltinTable>,
    /// Shared Ctx — stubbed in Phase 1; real backends in later phases.
    pub ctx: CtxInner,
    /// Per-extension metrics counters, when a `MetricsRegistry` was wired
    /// at registration. `None` ⇒ no metrics overhead on the call path.
    pub counters: Option<Arc<Counters>>,
}

impl ExtensionToolBinding {
    /// Wrap one `ContributeTool` entry. Reads the description + schema
    /// files relative to `bundle_dir`. Failure to read either file is a
    /// load-time error — the adapter surfaces it before the host serves
    /// any traffic.
    pub fn build(
        extension_id: ExtensionId,
        bundle_dir: &std::path::Path,
        entry: &ContributeTool,
        builtins: Arc<BuiltinTable>,
        ctx: CtxInner,
    ) -> Result<Self, Error> {
        let description = std::fs::read_to_string(bundle_dir.join(&entry.description_file))
            .map_err(|e| {
                Error::manifest(format!(
                    "description_file {:?}: {}",
                    entry.description_file, e
                ))
            })?;
        let schema_bytes =
            std::fs::read_to_string(bundle_dir.join(&entry.input_schema)).map_err(|e| {
                Error::manifest(format!("input_schema {:?}: {}", entry.input_schema, e))
            })?;
        let input_schema: serde_json::Value = serde_json::from_str(&schema_bytes).map_err(|e| {
            Error::manifest(format!(
                "input_schema {:?} is not valid JSON: {}",
                entry.input_schema, e
            ))
        })?;
        Ok(Self {
            extension_id,
            tool_id: entry.id.clone(),
            description,
            input_schema,
            builtins,
            ctx,
            counters: None,
        })
    }

    /// Attach a metrics-counter handle so `invoke` bumps
    /// `tool_calls_total` / `tool_errors_total`. Builder-style so the
    /// registration helpers can wire it only when a registry is present.
    #[must_use]
    pub fn with_counters(mut self, counters: Option<Arc<Counters>>) -> Self {
        self.counters = counters;
        self
    }
}

#[async_trait]
impl Tool for ExtensionToolBinding {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.tool_id.clone(),
            description: self.description.clone(),
            input_schema: self.input_schema.clone(),
        }
    }

    async fn invoke(&self, input: serde_json::Value) -> starter_spi::Result<serde_json::Value> {
        let entry =
            self.builtins
                .get(&self.extension_id)
                .ok_or_else(|| starter_spi::Error::NotFound {
                    what: format!(
                        "starter-ext-mcp: extension {:?} is not in the BuiltinTable — was \
                     `register_static_table!` called for its crate?",
                        self.extension_id.as_str()
                    ),
                })?;
        let result = entry
            .dispatch(&self.tool_id, &self.ctx, input)
            .map_err(|e| map_ext_error(e, self.extension_id.as_str()));
        record_tool_invocation(&self.counters, &result);
        result
    }
}

/// Convert a kernel `Error` into the `starter_spi` `Error` the MCP
/// transport surfaces. Kept here (not in `starter-ext-spi`) so the
/// kernel does not depend on `starter-spi`'s error categories.
///
/// `subject` is the extension id of the binding that produced the
/// error — passed in so a `Transport` failure can be re-shaped as a
/// recoverable `Unavailable` with a non-empty `subject` that the
/// HTTP layer turns into a restart link. Bindings whose error has
/// no specific subject (e.g. the builtin-flavour binding) pass an
/// empty string and the resulting `Unavailable.subject` stays
/// `Some("")`, which the transport will skip.
fn map_ext_error(e: Error, subject: &str) -> starter_spi::Error {
    use starter_spi::Error as SE;
    match e {
        Error::Validation(m) => SE::Invalid { message: m },
        Error::Capability(_) => SE::Forbidden,
        // Transport errors from a supervised process are the most
        // common recoverable failure mode: the child crashed and the
        // restart budget might still allow a respawn, or the operator
        // can force one. Surfacing this as `Unavailable` (rather than
        // `Internal`) lets transports map it to HTTP 503 and surface
        // a restart affordance keyed on the extension id.
        Error::Transport(m) => {
            SE::unavailable_subject("extension.supervisor_unavailable", subject, m)
        }
        other => SE::Internal {
            source: Box::new(other),
        },
    }
}

// ---------------------------------------------------------------------------
// ProcessExtensionToolBinding — process-flavour MCP tool wrapper
// ---------------------------------------------------------------------------

/// MCP `Tool` impl that dispatches into a process-flavour extension via
/// its [`starter_ext_supervisor::SupervisorHandle`].
///
/// Mirrors [`ExtensionToolBinding`] for the process flavour: same
/// manifest-driven definition (description + input schema read once at
/// load time, R7), but `invoke` routes through
/// [`starter_ext_supervisor::SupervisorHandle::call`] with the
/// JSON-RPC method `tools/<tool_id>` — matching the loop generated by
/// `starter_ext_sdk::register_process_main!` in the child binary.
pub struct ProcessExtensionToolBinding {
    /// The owning extension's id (used only in error messages; the
    /// supervisor handle already addresses one extension).
    pub extension_id: starter_ext_spi::ExtensionId,
    /// The tool id as it appears in `block.yaml` and on the MCP wire.
    pub tool_id: String,
    /// Cached description bytes.
    pub description: String,
    /// Cached input schema.
    pub input_schema: serde_json::Value,
    /// Handle to the supervised child process.
    pub handle: Arc<starter_ext_supervisor::SupervisorHandle>,
    /// Per-call request timeout.
    pub request_timeout: std::time::Duration,
    /// Per-extension metrics counters, when wired. `None` ⇒ no overhead.
    pub counters: Option<Arc<Counters>>,
}

impl ProcessExtensionToolBinding {
    /// Wrap one `ContributeTool` entry for a process-flavour extension.
    pub fn build(
        extension_id: starter_ext_spi::ExtensionId,
        bundle_dir: &std::path::Path,
        entry: &ContributeTool,
        handle: Arc<starter_ext_supervisor::SupervisorHandle>,
        request_timeout: std::time::Duration,
    ) -> Result<Self, Error> {
        let description = std::fs::read_to_string(bundle_dir.join(&entry.description_file))
            .map_err(|e| {
                Error::manifest(format!(
                    "description_file {:?}: {}",
                    entry.description_file, e
                ))
            })?;
        let schema_bytes =
            std::fs::read_to_string(bundle_dir.join(&entry.input_schema)).map_err(|e| {
                Error::manifest(format!("input_schema {:?}: {}", entry.input_schema, e))
            })?;
        let input_schema: serde_json::Value = serde_json::from_str(&schema_bytes).map_err(|e| {
            Error::manifest(format!(
                "input_schema {:?} is not valid JSON: {}",
                entry.input_schema, e
            ))
        })?;
        Ok(Self {
            extension_id,
            tool_id: entry.id.clone(),
            description,
            input_schema,
            handle,
            request_timeout,
            counters: None,
        })
    }

    /// Attach a metrics-counter handle so `invoke` bumps
    /// `tool_calls_total` / `tool_errors_total`.
    #[must_use]
    pub fn with_counters(mut self, counters: Option<Arc<Counters>>) -> Self {
        self.counters = counters;
        self
    }
}

#[async_trait]
impl Tool for ProcessExtensionToolBinding {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.tool_id.clone(),
            description: self.description.clone(),
            input_schema: self.input_schema.clone(),
        }
    }

    async fn invoke(&self, input: serde_json::Value) -> starter_spi::Result<serde_json::Value> {
        let method = format!("tools/{}", self.tool_id);
        // Use the host-set caller scope (rubix-agent's tools router
        // stamps it from `Principal`) so the child's `ctx.caller()`
        // resolves to a real tenant frame. Absent ⇒ system frame,
        // which is the correct fail-closed behaviour for
        // unauthenticated callers.
        let res = match starter_ext_supervisor::caller_local::current() {
            Some(caller) => {
                self.handle
                    .call_as(&method, input, caller, self.request_timeout)
                    .await
            }
            None => self.handle.call(&method, input, self.request_timeout).await,
        };
        let result = res.map_err(|e| map_ext_error(e, self.extension_id.as_str()));
        record_tool_invocation(&self.counters, &result);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_ext_error_validation_becomes_invalid() {
        let mapped = map_ext_error(Error::Validation("bad input".into()), "com.acme.x");
        match mapped {
            starter_spi::Error::Invalid { message } => assert_eq!(message, "bad input"),
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[test]
    fn map_ext_error_capability_becomes_forbidden() {
        let mapped = map_ext_error(Error::Capability("no http_out".into()), "com.acme.x");
        assert!(matches!(mapped, starter_spi::Error::Forbidden));
    }

    #[test]
    fn map_ext_error_transport_becomes_unavailable_with_subject() {
        // Transport failures from a supervised process are the
        // recoverable variant — must surface as `Unavailable` with
        // the supplied subject so the HTTP layer can synthesise a
        // restart URL keyed on the extension id.
        let mapped = map_ext_error(
            Error::Transport("supervisor task is no longer running".into()),
            "com.rubix.example",
        );
        match mapped {
            starter_spi::Error::Unavailable {
                code,
                subject,
                message,
            } => {
                assert_eq!(code, "extension.supervisor_unavailable");
                assert_eq!(subject.as_deref(), Some("com.rubix.example"));
                assert_eq!(message, "supervisor task is no longer running");
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn map_ext_error_other_becomes_internal() {
        let mapped = map_ext_error(Error::manifest("missing field"), "com.acme.x");
        assert!(matches!(mapped, starter_spi::Error::Internal { .. }));
    }
}
