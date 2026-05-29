//! Project per-kind counts for the cheap overview endpoint.
//!
//! Counts are derived by running each projector and counting the
//! emitted rows. The projectors are pure walks of in-memory data,
//! so this is fast even for callers that hit `/admin/overview`
//! frequently.

use rubix_spi::dto::admin::{RegistryKind, RegistryOverview};

use super::state::AdminState;

/// Build the [`RegistryOverview`] from the supplied state.
pub fn overview(state: &AdminState) -> RegistryOverview {
    RegistryOverview::from_fn(|kind| count(kind, state) as u32)
}

fn count(kind: RegistryKind, state: &AdminState) -> usize {
    match kind {
        RegistryKind::Tool => state.tools.len(),
        RegistryKind::Node => state.node_behaviors.len(),
        RegistryKind::Rule => state.rules.as_ref().map(|r| r.len()).unwrap_or(0),
        RegistryKind::Template => state.templates.as_ref().map(|t| t.len()).unwrap_or(0),
        RegistryKind::Table => super::tables::table_items(state.extensions.as_ref()).len(),
        RegistryKind::Skill => super::skills::skill_items().len(),
        RegistryKind::Extension => state
            .extensions
            .as_ref()
            .map(|e| e.list().len())
            .unwrap_or(0),
    }
}
