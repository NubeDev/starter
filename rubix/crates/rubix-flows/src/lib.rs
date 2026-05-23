//! Rubix bundled flows.
//!
//! Six flow YAMLs, one per goal, each rooted at an `ai-agent` node.
//! Embedded via `include_dir!` and fed into the host's `FlowRegistry`
//! at boot. Every bundled flow auto-surfaces as an MCP tool through
//! starter's `FlowAsTool` — no per-flow MCP wiring needed. See
//! [docs/design/flows/](../../docs/design/flows/README.md).

use include_dir::{include_dir, Dir};

/// All bundled rubix flows, embedded at compile time.
pub static BUNDLED: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/flows");

/// Return the embedded flow bundle. The agent binary wraps this in
/// the starter-flow loader; rubix-flows itself does not depend on
/// starter-flow to keep this crate tiny — content crates ship
/// content, not behaviour.
pub fn bundled() -> &'static Dir<'static> {
    &BUNDLED
}
