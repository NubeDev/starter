//! The `block.yaml` manifest schema.
//!
//! Per SCOPE.md **R3**: the manifest is the only source of truth for what an
//! extension provides. Parsed with `#[serde(deny_unknown_fields)]` so a typo
//! in a key is a load-time error, never a silent ignore. Per **R13** the
//! `contributes:` block reaches every transport the host exposes — `tools`,
//! `cli`, `rest`, `grpc`, `workers`, `ui` — through small adapter crates
//! that share this single schema.
//!
//! This module is *only* the schema. Reading the file, resolving the
//! `description_file:` / `*_schema:` paths against the bundle root, and
//! validating namespace ownership all live in `starter-ext-host`.

use serde::{Deserialize, Serialize};

use crate::{capability::Capability, ExtensionId};

/// The current manifest schema version.
///
/// SCOPE.md "Manifest schema versioning": new fields within a major are
/// additive forever; breaking changes bump this integer and the loader
/// supports the previous N versions.
pub const MANIFEST_VERSION: u32 = 1;

/// The deserialised `block.yaml`.
///
/// Field order follows the canonical layout shown in SCOPE.md
/// ("The manifest: `block.yaml`"). Optional fields default to absent so a
/// minimal manifest can omit them; required fields are non-`Option`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Manifest schema version. Must equal [`MANIFEST_VERSION`] (1) today;
    /// future hosts may accept a small window of older majors.
    pub v: u32,

    /// Reverse-DNS extension id. Owns every contributed identifier (R4).
    pub id: ExtensionId,

    /// Extension version (semver).
    pub version: semver::Version,

    /// Human-readable name surfaced in admin UIs.
    pub display_name: String,

    /// Path (relative to bundle root) to a static markdown file describing
    /// the extension. R7: never templated at runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_file: Option<String>,

    /// Author identifiers (email addresses, handles, …). Free-form;
    /// surfaced as-is to operators.
    #[serde(default)]
    pub authors: Vec<String>,

    /// Host interface dependencies. The host hard-fails the load if any
    /// required interface is missing or at an incompatible version (R6).
    #[serde(default)]
    pub requires: Vec<Require>,

    /// How this extension is packaged (R1: exactly one of builtin / wasm /
    /// process).
    pub runtime: Runtime,

    /// Process-flavour supervisor settings. Ignored for builtin / wasm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supervision: Option<Supervision>,

    /// Operator-supplied capability grants. The list of categories the
    /// extension *requires* lives in [`Manifest::requires`]; this map is
    /// the *values* (allowlists / scalars) the operator scopes them with.
    /// `http_out: []` is a legal "neutralised" grant (R6).
    #[serde(default)]
    pub capabilities: Vec<Capability>,

    /// Path (relative to bundle root) to a JSON Schema for the manifest's
    /// `config:` payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_schema: Option<String>,

    /// Operator-supplied configuration values; validated against
    /// `config_schema` at load time.
    #[serde(default)]
    pub config: serde_json::Value,

    /// Per-transport contributions (R13). Every adapter crate (`mcp`,
    /// `rest`, `cli`, `grpc`, `workers`, `ui`) reads its own field and
    /// wires the entries into its transport.
    #[serde(default)]
    pub contributes: Contributes,
}

/// `Manifest::requires` element — a host interface dependency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Require {
    /// Interface id (e.g. `starter.spi.tool`).
    pub id: String,
    /// Semver requirement (e.g. `"^1"`).
    pub version: semver::VersionReq,
}

/// Backwards-compatibility alias for code that historically named this
/// type. Kept so future renames inside the manifest module do not cascade
/// to consumer crates.
#[doc(hidden)]
pub type ManifestRequires = Vec<Require>;

/// How the extension is packaged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Runtime {
    /// Packaging flavour.
    pub kind: RuntimeKind,
    /// For `process`: path (relative to bundle root) to the spawned binary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bin: Option<String>,
    /// For `builtin`: the crate name linked at host build time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crate_name: Option<String>,
    /// For `wasm`: path (relative to bundle root) to the `.wasm` artefact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artefact: Option<String>,
}

/// One of the three packaging flavours (SCOPE R1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeKind {
    /// Statically linked into the host at host build time.
    Builtin,
    /// WASI-p2 component instantiated by `starter-ext-wasm`.
    Wasm,
    /// Child process spawned by `starter-ext-supervisor` and addressed via
    /// stdio JSON-RPC.
    Process,
}

/// Process-flavour supervisor settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Supervision {
    /// Restart policy. Defaults to `on_crash` if omitted.
    #[serde(default)]
    pub restart: RestartPolicy,
    /// Intensity cap: max restarts within [`Self::within_seconds`].
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,
    /// Intensity cap window.
    #[serde(default = "default_within_seconds")]
    pub within_seconds: u32,
    /// Exponential backoff parameters.
    #[serde(default)]
    pub backoff: Backoff,
    /// Health-check cadence and timeout.
    #[serde(default)]
    pub health: HealthConfig,
    /// Reserved for v0.2 supervisor groups (SCOPE.md "Decisions made:
    /// supervisor restart policy"). The supervisor ignores this in v0.1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// Grace window between `SIGTERM` and `SIGKILL` on shutdown.
    #[serde(default = "default_shutdown_grace_ms")]
    pub shutdown_grace_ms: u32,
}

/// Restart policy (SCOPE R9). Names are deliberately *not* the Erlang/OTP
/// vocabulary (`permanent | transient | temporary`) — same semantics, but
/// the names read naturally to a reader who has not done supervisor work
/// in Erlang.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    /// Restart on any exit (clean or crash). OTP `permanent`.
    Always,
    /// Restart only on abnormal exit. OTP `transient`. **Default.**
    #[default]
    OnCrash,
    /// Never restart. OTP `temporary`.
    Never,
}

/// Exponential-backoff settings for the supervisor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Backoff {
    /// First wait, in milliseconds.
    pub initial_ms: u32,
    /// Cap, in milliseconds.
    pub max_ms: u32,
    /// Add jitter on top of the exponential schedule.
    #[serde(default = "default_true")]
    pub jitter: bool,
}

impl Default for Backoff {
    fn default() -> Self {
        Self {
            initial_ms: 200,
            max_ms: 30_000,
            jitter: true,
        }
    }
}

/// Health-check cadence and timeout for process-flavour extensions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthConfig {
    /// Interval between `health` pings.
    pub interval_ms: u32,
    /// Maximum time the child has to acknowledge a `health` ping before
    /// the supervisor treats the silence as a crash.
    pub timeout_ms: u32,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            interval_ms: 5_000,
            timeout_ms: 2_000,
        }
    }
}

// ---------------------------------------------------------------------------
// contributes:
// ---------------------------------------------------------------------------

/// Per-transport contributions (SCOPE R13).
///
/// Every field is optional and defaults to empty — a minimal extension that
/// only contributes tools omits the other sections entirely. Each list is a
/// vector of small typed entries shared with the adapter crate that owns
/// that transport.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contributes {
    /// MCP-style tools. Consumed by `starter-ext-mcp-adapter`.
    #[serde(default)]
    pub tools: Vec<ContributeTool>,
    /// CLI subcommands. Consumed by `starter-ext-cli-adapter`.
    #[serde(default)]
    pub cli: Vec<ContributeCli>,
    /// REST routes. Consumed by `starter-ext-rest-adapter`.
    #[serde(default)]
    pub rest: Vec<ContributeRest>,
    /// gRPC RPCs. Consumed by `starter-ext-grpc-adapter`.
    #[serde(default)]
    pub grpc: Vec<ContributeGrpc>,
    /// Periodic worker jobs. Consumed by `starter-ext-workers-adapter`.
    #[serde(default)]
    pub workers: Vec<ContributeWorker>,
    /// UI Module-Federation entry. At most one block per extension (a
    /// single `remoteEntry.js` exposing one or more modules).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui: Option<ContributeUi>,
}

/// Per-entry auth gate (SCOPE post-R13 "per-entry auth shape").
///
/// Optional on every contribute entry. Adapters wrap the handler in the
/// `Authenticator` middleware configured for these values; the extension
/// never sees a request that did not pass the gate. Omitting both fields
/// means "inherit the adapter's default".
///
/// `Role` / `Scope` types live in `starter-spi` — the same vocabulary
/// consumer code uses to gate its own routes. Keeping the manifest field as
/// `String` (rather than the typed `starter_spi::auth::Role` enum) means
/// new role names defined by a consumer's `auth` layer flow through the
/// manifest without a `starter-ext-spi` version bump; the adapter parses
/// them against its known role set at load time.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthGate {
    /// Minimum role required (e.g. `"admin"`, `"user"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_role: Option<String>,
    /// Required scope (e.g. `"extension:weather"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_scope: Option<String>,
}

/// One MCP-style tool the extension provides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributeTool {
    /// Tool id; must be the extension id or a dotted descendant (R4).
    pub id: String,
    /// Path (relative to bundle root) to the tool's JSON Schema for input.
    pub input_schema: String,
    /// Path (relative to bundle root) to the tool's JSON Schema for output.
    pub output_schema: String,
    /// Path (relative to bundle root) to the tool's static markdown
    /// description. R7: never templated.
    pub description_file: String,
    /// Optional per-entry auth gate.
    #[serde(default)]
    pub auth: AuthGate,
}

/// One CLI subcommand the extension provides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributeCli {
    /// Command id; must be the extension id or a dotted descendant (R4).
    pub id: String,
    /// The subcommand name as it appears on the command line
    /// (e.g. `weather-current`). The adapter validates that this name does
    /// not collide with any host-owned command.
    pub command: String,
    /// Path (relative to bundle root) to the static help text. R7.
    pub description_file: String,
    /// Path (relative to bundle root) to a JSON Schema describing the
    /// command's flag set. The adapter generates a `clap` parser from it.
    pub args_schema: String,
    /// Optional per-entry auth gate.
    #[serde(default)]
    pub auth: AuthGate,
}

/// One REST route the extension provides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributeRest {
    /// Route id; must be the extension id or a dotted descendant (R4).
    pub id: String,
    /// HTTP method, uppercase (`GET`, `POST`, `PUT`, …).
    pub method: String,
    /// Path template, mounted under the adapter's namespace prefix. May
    /// contain `{param}` segments.
    pub path: String,
    /// Path (relative to bundle root) to the route's static markdown
    /// description. R7.
    pub description_file: String,
    /// Optional path to a JSON Schema for the request body / query.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_schema: Option<String>,
    /// Optional path to a JSON Schema for the response body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<String>,
    /// `true` if the response is a server-sent event stream. Adapters use
    /// the streaming sub-protocol (`stream.event` / `stream.end` / …) to
    /// drive it.
    #[serde(default)]
    pub streaming: bool,
    /// Optional per-entry auth gate.
    #[serde(default)]
    pub auth: AuthGate,
}

/// One gRPC RPC the extension provides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributeGrpc {
    /// RPC id; must be the extension id or a dotted descendant (R4).
    pub id: String,
    /// gRPC service name (e.g. `weather.v1.Weather`).
    pub service: String,
    /// gRPC method name (e.g. `Current`).
    pub method: String,
    /// Path (relative to bundle root) to the `.proto` file describing the
    /// service. The adapter loads it at host build time.
    pub proto: String,
    /// Path (relative to bundle root) to the RPC's static markdown
    /// description. R7.
    pub description_file: String,
    /// `true` if the response is a server-streaming RPC. Adapter maps onto
    /// the streaming sub-protocol.
    #[serde(default)]
    pub streaming: bool,
    /// Optional per-entry auth gate.
    #[serde(default)]
    pub auth: AuthGate,
}

/// One periodic worker the extension provides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributeWorker {
    /// Worker id; must be the extension id or a dotted descendant (R4).
    pub id: String,
    /// Cron expression (5- or 6-field). The adapter parses it.
    pub cron: String,
    /// Path (relative to bundle root) to the worker's static markdown
    /// description. R7.
    pub description_file: String,
    /// Optional cap on concurrent executions of this worker (default 1).
    #[serde(default = "default_one")]
    pub concurrency: u32,
    /// Optional per-entry auth gate (workers run under a system principal
    /// if omitted; the gate is for operator-triggered runs).
    #[serde(default)]
    pub auth: AuthGate,
}

/// The extension's UI block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributeUi {
    /// Path (relative to bundle root) to the Module-Federation entry
    /// (`remoteEntry.js`). Served by `starter-ext-server` at
    /// `/extensions/<id>/ui/*`.
    pub entry: String,
    /// Modules the remote exposes. Each maps onto a named slot.
    pub exposes: Vec<ContributeUiExpose>,
}

/// One module a UI extension exposes to the host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributeUiExpose {
    /// Component name surfaced to the Module-Federation runtime
    /// (e.g. `WeatherPanel`).
    pub name: String,
    /// Remote module path the host loads (e.g. `"./Panel"`).
    pub module: String,
    /// Host slot id the component mounts into (`sidebar`, `header`, …).
    pub slot: String,
    /// Optional per-entry auth gate.
    #[serde(default)]
    pub auth: AuthGate,
}

// ---------------------------------------------------------------------------
// serde default helpers
// ---------------------------------------------------------------------------

fn default_max_restarts() -> u32 {
    5
}
fn default_within_seconds() -> u32 {
    60
}
fn default_shutdown_grace_ms() -> u32 {
    5_000
}
fn default_true() -> bool {
    true
}
fn default_one() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The canonical manifest from SCOPE.md "The manifest: `block.yaml`"
    /// must parse cleanly with `deny_unknown_fields`.
    #[test]
    fn parses_scope_example_manifest() {
        let yaml = r#"
v: 1
id: com.acme.weather
version: 0.1.0
display_name: "Weather"
description_file: docs/README.md
authors: ["ap@nube-io.com"]

requires:
  - { id: starter.spi.tool,    version: "^1" }
  - { id: starter.spi.secrets, version: "^1" }

runtime:
  kind: process
  bin: dist/weather-driver

supervision:
  restart: always
  max_restarts: 5
  within_seconds: 60
  backoff: { initial_ms: 200, max_ms: 30000, jitter: true }
  health:  { interval_ms: 5000, timeout_ms: 2000 }
  shutdown_grace_ms: 5000

capabilities:
  - kind: secrets
    prefixes: ["weather:*"]
  - kind: http_out
    authorities: ["api.weather.gov"]
  - kind: fs
    paths: []
  - kind: wall_clock
    granted: true

config_schema: schemas/config.json
config: {}

contributes:
  tools:
    - id: com.acme.weather.current
      input_schema:  schemas/current_in.json
      output_schema: schemas/current_out.json
      description_file: docs/tools/current.md
  ui:
    entry: ui/remoteEntry.js
    exposes:
      - { name: WeatherPanel, module: "./Panel", slot: sidebar }
"#;
        let m: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(m.v, MANIFEST_VERSION);
        assert_eq!(m.id.as_str(), "com.acme.weather");
        assert_eq!(m.runtime.kind, RuntimeKind::Process);
        assert_eq!(m.contributes.tools.len(), 1);
        assert!(m.contributes.ui.is_some());
        assert_eq!(m.supervision.unwrap().restart, RestartPolicy::Always);
    }

    #[test]
    fn deny_unknown_fields_top_level() {
        let yaml = r#"
v: 1
id: com.acme.weather
version: 0.1.0
display_name: "Weather"
runtime: { kind: builtin, crate_name: weather }
nope_unknown_top_level: true
"#;
        let err = serde_yaml::from_str::<Manifest>(yaml).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("nope_unknown_top_level") || msg.contains("unknown field"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn deny_unknown_fields_inside_contributes() {
        let yaml = r#"
v: 1
id: com.acme.weather
version: 0.1.0
display_name: "Weather"
runtime: { kind: builtin, crate_name: weather }
contributes:
  tools:
    - id: com.acme.weather.x
      input_schema: a.json
      output_schema: b.json
      description_file: c.md
      bogus_field: 1
"#;
        assert!(serde_yaml::from_str::<Manifest>(yaml).is_err());
    }

    #[test]
    fn minimal_manifest_only_required_fields() {
        let yaml = r#"
v: 1
id: com.acme.minimal
version: 0.0.1
display_name: "Minimal"
runtime: { kind: builtin, crate_name: minimal }
"#;
        let m: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert!(m.contributes.tools.is_empty());
        assert!(m.contributes.cli.is_empty());
        assert!(m.contributes.rest.is_empty());
        assert!(m.contributes.grpc.is_empty());
        assert!(m.contributes.workers.is_empty());
        assert!(m.contributes.ui.is_none());
        assert!(m.capabilities.is_empty());
        assert!(m.supervision.is_none());
    }

    #[test]
    fn contributes_covers_every_transport() {
        let yaml = r#"
v: 1
id: com.acme.all
version: 0.0.1
display_name: "All"
runtime: { kind: process, bin: ./drv }
contributes:
  tools:
    - id: com.acme.all.t1
      input_schema: a.json
      output_schema: b.json
      description_file: c.md
  cli:
    - id: com.acme.all.cli1
      command: all-do
      description_file: c.md
      args_schema: a.json
  rest:
    - id: com.acme.all.r1
      method: GET
      path: /things/{id}
      description_file: c.md
  grpc:
    - id: com.acme.all.g1
      service: acme.v1.All
      method: Do
      proto: proto/all.proto
      description_file: c.md
  workers:
    - id: com.acme.all.w1
      cron: "*/5 * * * *"
      description_file: c.md
  ui:
    entry: ui/remoteEntry.js
    exposes:
      - { name: P, module: ./P, slot: sidebar }
"#;
        let m: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(m.contributes.tools.len(), 1);
        assert_eq!(m.contributes.cli.len(), 1);
        assert_eq!(m.contributes.rest.len(), 1);
        assert_eq!(m.contributes.grpc.len(), 1);
        assert_eq!(m.contributes.workers.len(), 1);
        assert!(m.contributes.ui.is_some());
    }

    #[test]
    fn per_entry_auth_gate_parses() {
        let yaml = r#"
v: 1
id: com.acme.gated
version: 0.0.1
display_name: "Gated"
runtime: { kind: builtin, crate_name: gated }
contributes:
  rest:
    - id: com.acme.gated.admin
      method: POST
      path: /admin/do
      description_file: c.md
      auth: { require_role: admin }
"#;
        let m: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            m.contributes.rest[0].auth.require_role.as_deref(),
            Some("admin")
        );
    }
}
