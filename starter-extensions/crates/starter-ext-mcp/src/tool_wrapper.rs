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

use starter_ext_sdk::builtin::BuiltinTable;
use starter_ext_sdk::ctx::CtxInner;
use starter_ext_spi::{ContributeTool, Error, ExtensionId};
use starter_spi::tool::{Tool, ToolDefinition};

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
        let schema_bytes = std::fs::read_to_string(bundle_dir.join(&entry.input_schema))
            .map_err(|e| {
                Error::manifest(format!(
                    "input_schema {:?}: {}",
                    entry.input_schema, e
                ))
            })?;
        let input_schema: serde_json::Value =
            serde_json::from_str(&schema_bytes).map_err(|e| {
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
        })
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

    async fn invoke(
        &self,
        input: serde_json::Value,
    ) -> starter_spi::Result<serde_json::Value> {
        let entry = self.builtins.get(&self.extension_id).ok_or_else(|| {
            starter_spi::Error::NotFound {
                what: format!(
                    "starter-ext-mcp: extension {:?} is not in the BuiltinTable — was \
                     `register_static_table!` called for its crate?",
                    self.extension_id.as_str()
                ),
            }
        })?;
        let result = entry.dispatch(&self.tool_id, &self.ctx, input);
        result.map_err(map_ext_error)
    }
}

/// Convert a kernel `Error` into the `starter_spi` `Error` the MCP
/// transport surfaces. Kept here (not in `starter-ext-spi`) so the
/// kernel does not depend on `starter-spi`'s error categories.
fn map_ext_error(e: Error) -> starter_spi::Error {
    use starter_spi::Error as SE;
    match e {
        Error::Validation(m) => SE::Invalid { message: m },
        Error::Capability(_) => SE::Forbidden,
        other => SE::Internal {
            source: Box::new(other),
        },
    }
}
