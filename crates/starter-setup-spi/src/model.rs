//! Net-new domain model (DOCS §4). The engine speaks `FlowBody` (a node
//! graph) and `RunId` (an execution); the setup layer adds the friendly
//! wrapper a non-author can launch and watch.

use std::fmt;

use serde::{Deserialize, Serialize};
use starter_flow::definition::body::FlowBody;
use starter_flow_spi::flow::RunId;

use crate::error::SetupError;

/// Reverse-DNS template identifier, e.g. `com.acme.add-device`.
///
/// A thin newtype over `String`; the value is validated as reverse-DNS
/// when it flows through a `FlowBody.flow_id`, so the wrapper itself is
/// permissive (the catalog stores it as an opaque key).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TemplateId(pub String);

impl TemplateId {
    /// Borrow the underlying id string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for TemplateId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl From<String> for TemplateId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl fmt::Display for TemplateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A minimal, ordering-aware semantic version (`major.minor.patch`).
///
/// The codebase has no existing `SemVer` type (verified), and templates
/// only need exact identity + a deterministic ordering for "latest", so
/// this is a small purpose-built type rather than a new dependency. It is
/// serialized as the canonical `"1.2.0"` string so YAML and stored rows
/// round-trip byte-identically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SemVer {
    /// Major version (ordered first).
    pub major: u64,
    /// Minor version.
    pub minor: u64,
    /// Patch version.
    pub patch: u64,
}

impl SemVer {
    /// Construct a version from its parts.
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parse a `"major.minor.patch"` string. Each component must be a
    /// non-negative integer; extra components or non-numeric parts are
    /// rejected.
    pub fn parse(s: &str) -> Result<Self, SetupError> {
        let mut it = s.split('.');
        let mut next = |s: &str| -> Result<u64, SetupError> {
            it.next()
                .ok_or_else(|| SetupError::InvalidVersion(s.to_owned()))?
                .parse::<u64>()
                .map_err(|_| SetupError::InvalidVersion(s.to_owned()))
        };
        let major = next(s)?;
        let minor = next(s)?;
        let patch = next(s)?;
        if it.next().is_some() {
            return Err(SetupError::InvalidVersion(s.to_owned()));
        }
        Ok(Self::new(major, minor, patch))
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Serialize for SemVer {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SemVer {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        SemVer::parse(&s).map_err(serde::de::Error::custom)
    }
}

/// Maps one launcher form field onto one flow entry slot (DOCS §6).
///
/// Bindings are objects (`{ field, slot }`), not the `a -> b`
/// pseudo-syntax of the first draft. `slot` is a `node.slot` reference
/// into the flow body's entry slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputBinding {
    /// The `input_schema` form field name supplying the value.
    pub field: String,
    /// The `node.slot` entry-slot reference the value is written to.
    pub slot: String,
}

/// Maps one terminal flow slot onto a field of the run's result (DOCS §6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputBinding {
    /// The terminal `node.slot` reference whose value becomes a result.
    pub slot: String,
    /// The result field name the slot value is exposed under.
    pub field: String,
}

/// Who may author vs run a template (DOCS §4/§10). The data-dependent
/// team check is a setup-layer Rust check, not an authz condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TemplateAccess {
    /// Owning tenant; `None` for the `__global__` extension catalog.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Teams allowed to run. **Empty = any team in the tenant.**
    #[serde(default)]
    pub allowed_teams: Vec<String>,
    /// Role required to run (coarse authz gate, e.g. `"writer"`).
    #[serde(default)]
    pub run_role: Option<String>,
}

impl TemplateAccess {
    /// The setup-layer team check (DOCS §10 step 2). Empty `allowed_teams`
    /// means any team in the tenant; otherwise the principal must share at
    /// least one team. This is the data-dependent part the authz condition
    /// engine cannot see.
    pub fn team_allows(&self, principal_teams: &[String]) -> bool {
        if self.allowed_teams.is_empty() {
            return true;
        }
        self.allowed_teams
            .iter()
            .any(|t| principal_teams.iter().any(|p| p == t))
    }
}

/// Provenance of a stored template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TemplateSource {
    /// Imported from a YAML file committed in git.
    Yaml {
        /// Source path (informational).
        path: String,
    },
    /// Created/published through the REST or MCP API.
    Api,
    /// Contributed by an extension's `block.yaml`.
    Extension {
        /// The contributing extension id.
        ext_id: String,
    },
}

/// A published, parameterized automation a user can launch (DOCS §4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Template {
    /// Reverse-DNS id, e.g. `com.acme.add-device`.
    pub id: TemplateId,
    /// Immutable-once-published version.
    pub version: SemVer,
    /// Human-readable name shown in the launcher nav.
    pub display_name: String,
    /// Longer description.
    #[serde(default)]
    pub description: String,
    /// Optional icon name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Nav grouping.
    #[serde(default)]
    pub category: String,
    /// The launcher form, as JSON Schema. Validated server-side before a
    /// run starts.
    pub input_schema: serde_json::Value,
    /// The node graph — the "100 steps". This IS a `starter-flow`
    /// `FlowBody`.
    pub flow_body: FlowBody,
    /// How seeded form inputs map onto the flow's entry slots.
    #[serde(default)]
    pub input_bindings: Vec<InputBinding>,
    /// Which terminal slots become the run's result.
    #[serde(default)]
    pub output_bindings: Vec<OutputBinding>,
    /// Who may author vs run this.
    #[serde(default)]
    pub access: TemplateAccess,
    /// Where this template came from.
    pub source: TemplateSource,
}

/// Lightweight projection for list/nav views (DOCS §4 traits).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateSummary {
    /// Template id.
    pub id: TemplateId,
    /// Template version.
    pub version: SemVer,
    /// Display name.
    pub display_name: String,
    /// Nav category.
    pub category: String,
    /// Owning tenant (or `None` for the global catalog).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// Lifecycle of a setup run as the friendly index sees it (DOCS §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SetupRunStatus {
    /// Recorded but not yet started ticking.
    Pending,
    /// Actively executing.
    Running,
    /// Terminal failure — see [`SetupRun::failed_node`] / `resumable`
    /// (DOCS §8b).
    Failed,
    /// Terminal success.
    Completed,
    /// Cancelled by a user.
    Cancelled,
}

impl SetupRunStatus {
    /// Whether this is a terminal state.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            SetupRunStatus::Failed | SetupRunStatus::Completed | SetupRunStatus::Cancelled
        )
    }

    /// Stored string form (stable across DB rows + SSE).
    pub fn as_str(self) -> &'static str {
        match self {
            SetupRunStatus::Pending => "Pending",
            SetupRunStatus::Running => "Running",
            SetupRunStatus::Failed => "Failed",
            SetupRunStatus::Completed => "Completed",
            SetupRunStatus::Cancelled => "Cancelled",
        }
    }

    /// Parse the stored string form.
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "Pending" => SetupRunStatus::Pending,
            "Running" => SetupRunStatus::Running,
            "Failed" => SetupRunStatus::Failed,
            "Completed" => SetupRunStatus::Completed,
            "Cancelled" => SetupRunStatus::Cancelled,
            _ => return None,
        })
    }
}

/// Streamed progress snapshot (DOCS §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Progress {
    /// Steps completed.
    pub done: usize,
    /// Total steps (flow node count).
    pub total: usize,
    /// The node id currently executing, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_step: Option<String>,
}

/// A launch of a [`Template`]: a thin index row over a flow `RunId` so
/// runs can be listed, authorized, and resumed by template/owner/tenant
/// without touching the engine's internal run tables (DOCS §4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetupRun {
    /// The flow engine's run id (FK into the flow `runs` table).
    pub run_id: RunId,
    /// Template launched.
    pub template_id: TemplateId,
    /// Template version pinned at launch (DOCS Q2).
    pub template_version: SemVer,
    /// `Principal.subject` of the launcher.
    pub owner: String,
    /// Launcher tenant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Launcher team (first team, informational).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
    /// Current status.
    pub status: SetupRunStatus,
    /// Progress snapshot.
    pub progress: Progress,
    /// The node id the run halted on (DOCS §8b cursor), when failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed_node: Option<String>,
    /// Whether a failed run may be resumed from its cursor (DOCS §8b).
    #[serde(default)]
    pub resumable: bool,
    /// Creation timestamp (RFC3339).
    pub created_at: String,
    /// Terminal timestamp (RFC3339), if finished.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
}
