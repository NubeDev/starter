//! `rubix.flow_ops.lint` — request/response DTOs and tool descriptor.
//!
//! Read-only verb: parses a YAML body through
//! `rubix_flows::yaml::RubixFlowYaml` (and the downstream
//! `rubix_flows::convert` pass) and returns a structured list of
//! [`LintDiagnostic`]s with line/column information when available.
//! No state is written. See
//! [docs/design/flows/](../../../../docs/design/flows/README.md).

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.flow_ops.lint`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FlowLintRequest {
    /// Raw YAML body to validate.
    pub body_yaml: String,
}

/// One structured lint error.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LintDiagnostic {
    /// Human-readable error message (already pre-rendered).
    pub message: String,
    /// 1-based line, when the underlying parser can locate it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// 1-based column, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FlowLintResponse {
    /// `rubix.flow.linted` when `errors` is empty,
    /// `rubix.flow.lint.found_errors` otherwise.
    pub summary: Diagnostic,
    /// Empty when the YAML is acceptable.
    pub errors: Vec<LintDiagnostic>,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "flows.read";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Validate a rubix flow YAML body and surface structured errors with line numbers.",
    when_to_use: concat!(
        "Use as a pre-check before rubix.flow_ops.deploy, or when the ",
        "operator is iterating on a draft and asks \"does this parse?\"."
    ),
    when_not_to_use: concat!(
        "Do not use to deploy — lint is read-only. Do not rely on lint ",
        "alone to gate a production push; deploy runs the same checks ",
        "and is the authoritative path."
    ),
    example: concat!(
        "Input:  { \"body_yaml\": \"id: com.example.broken\\nnodes: []\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.flow.lint.found_errors\", ",
        "\"params\": { \"count\": 1 } }, \"errors\": [ { \"message\": ",
        "\"flow `…`: must declare at least one node\" } ] }"
    ),
    siblings: &[SiblingTool {
        id: "rubix.flow_ops.deploy",
        wins_when: "the caller has already linted and wants to write the revision.",
    }],
};
