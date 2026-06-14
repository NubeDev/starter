//! `GET /api/v1/insights/functions` — the curated insight function catalog.
//!
//! A static, tenant-independent description of every function the Rhai insight
//! sandbox exposes, so the workbench can render a cheatsheet and drive
//! autocomplete that never drifts from the engine. The list is authored
//! alongside the engine's op registry and served verbatim.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// One curated insight function, as the UI's cheatsheet / autocomplete renders
/// it. Every field is display copy — the engine remains the enforcing authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct InsightFunctionDoc {
    /// The bare function name as called on a frame, e.g. `zscore`.
    pub name: String,
    /// The full signature with argument names and types, e.g.
    /// `zscore(col: string)`.
    pub signature: String,
    /// A one-line description of what the function does.
    pub summary: String,
    /// The bucket the UI groups it under: `select | filter | window | shape |
    /// resample | anomaly`.
    pub category: String,
    /// A runnable example call, e.g. `zscore("value")`.
    pub example: String,
}

/// The whole curated catalog — every function a script may call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct InsightFunctionCatalog {
    /// Every curated insight function the sandbox exposes.
    pub functions: Vec<InsightFunctionDoc>,
}
