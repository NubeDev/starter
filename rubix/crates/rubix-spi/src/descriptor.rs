//! Tool descriptor authoring contract.
//!
//! Every rubix tool ships a `ToolDescriptor` with five fields. The
//! descriptor steers both the bundled rubix agent and external MCP
//! clients — the bar is the same. Empty or one-line descriptors fail
//! review. See [docs/design/mcp-ux/](../../docs/design/mcp-ux/README.md)
//! for the worked-example bar and the calibration test (≥80% reviewer
//! agreement on tool choice).

use serde::Serialize;

/// Static metadata describing a rubix tool to LLMs and operators.
///
/// All five fields are mandatory at review time. The `siblings` field
/// is the disambiguation field — with ~25 rubix tools across six
/// goals, naming near-neighbour tools and saying why *this* one wins
/// is what keeps the agent from picking the wrong action.
///
/// Descriptors are compiled into the binary as `&'static` data so
/// they cannot be tampered with at runtime (anti-prompt-injection
/// parity with the host's skill-bundle trust model). Therefore
/// Serialize only — they never round-trip from the wire.
#[derive(Debug, Clone, Serialize)]
pub struct ToolDescriptor {
    /// One sentence, plain English. What the tool does.
    pub purpose: &'static str,
    /// Concrete trigger conditions. When the agent should pick this.
    pub when_to_use: &'static str,
    /// The most common misuse. When the agent should NOT pick this.
    pub when_not_to_use: &'static str,
    /// One realistic input + output, ≤10 lines total.
    pub example: &'static str,
    /// Tool ids most likely to be confused with this one, with a phrase
    /// explaining when *this* tool wins. Empty allowed only when the
    /// tool truly has no near-neighbours (rare).
    pub siblings: &'static [SiblingTool],
}

/// One near-neighbour tool, named with the reason this tool is the
/// right pick over it. Both fields are required when present.
#[derive(Debug, Clone, Serialize)]
pub struct SiblingTool {
    /// The other tool's id (e.g. "rubix.flow.validate").
    pub id: &'static str,
    /// Why this tool wins over the sibling, in one phrase.
    pub wins_when: &'static str,
}
