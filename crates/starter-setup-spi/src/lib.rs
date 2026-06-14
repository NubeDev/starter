//! Setup / Automation Builder — service provider interface.
//!
//! Contracts-only crate (mirrors [`starter_flow_spi`]). It defines the
//! friendly **Template** / **SetupRun** domain that wraps the flow
//! engine's `FlowBody` / `RunId`, the [`TemplateStore`] and
//! [`SetupRunStore`] traits backends implement, and the YAML
//! [`TemplateEnvelope`] import/export shape.
//!
//! See `DOCS/setup-automation-builder.md` §4–§6. The net-new code is a
//! thin Template/Run domain on top of primitives that already exist; this
//! crate holds the *contracts* for that domain with zero runtime logic.

pub mod envelope;
pub mod error;
pub mod model;
pub mod store;

pub use error::{SetupError, SetupResult};
pub use model::{
    InputBinding, OutputBinding, Progress, SemVer, SetupRun, SetupRunStatus, Template,
    TemplateAccess, TemplateId, TemplateSource, TemplateSummary,
};
pub use store::{
    SetupRunFilter, SetupRunStore, TemplateFilter, TemplateStore, GLOBAL_TENANT_SENTINEL,
};

/// Reserved entry-slot names the run service seeds from the **verified**
/// `Principal` at `FlowRunner::start` (DOCS §9 "Trusted identity").
///
/// These are written from host-bound identity, never from client form
/// input. A template's [`InputBinding`]s must never target them — see
/// [`reserved::is_reserved`] and the import/run-time validation that
/// rejects bindings whose `slot` resolves to one of these names.
pub mod reserved {
    /// `Principal.subject` of the launcher.
    pub const CALLER_USER_ID: &str = "caller_user_id";
    /// `Principal.teams` (JSON array) of the launcher.
    pub const CALLER_TEAM_IDS: &str = "caller_team_ids";
    /// `Principal.tenant_id` of the launcher.
    pub const CALLER_TENANT_ID: &str = "caller_tenant_id";

    /// All reserved trusted-identity slot names.
    pub const ALL: [&str; 3] = [CALLER_USER_ID, CALLER_TEAM_IDS, CALLER_TENANT_ID];

    /// Whether `name` is a reserved trusted-identity slot name.
    ///
    /// Matching is on the **slot name** (the part after the node id in a
    /// `node.slot` reference), since the run service seeds identity onto
    /// every entry node under these names.
    pub fn is_reserved(name: &str) -> bool {
        ALL.contains(&name)
    }
}
