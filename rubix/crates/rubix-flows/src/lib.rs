//! Rubix bundled flows.
//!
//! Six flow YAMLs, one per goal, each rooted at an `ai-agent` node.
//! Embedded via `include_dir!` and fed into the host's `FlowRegistry`
//! at boot. Every bundled flow auto-surfaces as an MCP tool through
//! starter's `FlowAsTool` — no per-flow MCP wiring needed. See
//! [docs/design/flows/](../../docs/design/flows/README.md).

use include_dir::{include_dir, Dir};

pub mod convert;
pub mod error;
pub mod load;
pub mod yaml;

pub use convert::{
    convert, AI_AGENT_KIND_ID, AI_AGENT_KIND_YAML, DEFAULT_OUTPUT_SLOT, DEFAULT_SEED_SLOT,
    NODE_ID_PREFIX,
};
pub use error::LoadError;
pub use load::{into_arcs, load_all};
pub use yaml::{parse_yaml, RubixFlowYaml, RubixLinkYaml, RubixNodeYaml, ALLOWED_TOOLS_KEY};

/// All bundled rubix flows, embedded at compile time.
pub static BUNDLED: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/flows");

/// All bundled rubix SDUI dashboard pages, embedded at compile
/// time. Each `<slug>.json` under `dashboards/` is seeded into
/// `dashboards_definitions` on first boot by
/// `rubix_agent::boot::dashboards_seed`.
pub static BUNDLED_DASHBOARDS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/dashboards");

/// Return the embedded flow bundle. The agent binary wraps this in
/// the starter-flow loader; rubix-flows itself does not depend on
/// starter-flow to keep this crate tiny — content crates ship
/// content, not behaviour.
pub fn bundled() -> &'static Dir<'static> {
    &BUNDLED
}
