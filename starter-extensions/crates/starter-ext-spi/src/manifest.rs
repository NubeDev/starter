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
    /// Accepts either typed `{id, version}` or bare-string surface form
    /// — see [`Require`].
    #[serde(default, deserialize_with = "deserialize_requires")]
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
///
/// Two surface forms accepted on the wire (R6 + `examples/notes/user-pref.md`
/// § Stage 5):
///
/// ```yaml
/// requires:
///   - { id: starter.spi.tool, version: "^1" }   # typed form, Rust SPI
///   - "@nube/starter-ui-core/preferences"        # singleton form, JS surface
/// ```
///
/// The bare-string form maps to `version: "*"` because singleton ids
/// declare a capability, not a Rust crate semver bound — the actual
/// version negotiation happens host-side via
/// `ExtensionHostManager.checkSingletons` against
/// `RemoteFactory.singletons` (which carries the real version pin).
/// Listing the id in `requires:` is the manifest's way of saying "the
/// host must provide this singleton at load time" so the operator can
/// see the dependency without parsing the JS bundle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Require {
    /// Interface id (e.g. `starter.spi.tool`,
    /// `@nube/starter-ui-core/preferences`).
    pub id: String,
    /// Semver requirement (e.g. `"^1"`). Defaults to `"*"` so the
    /// bare-string surface form (see [`Require`]) deserialises without
    /// the caller having to repeat the unbounded marker.
    #[serde(default = "any_version_req")]
    pub version: semver::VersionReq,
}

fn any_version_req() -> semver::VersionReq {
    semver::VersionReq::STAR
}

/// Surface-form for [`Require`] — accepts a bare string or the typed
/// struct. Normalised into [`Require`] post-deserialisation so the rest
/// of the loader works against one shape.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
enum RequireWire {
    Bare(String),
    Typed(RequireTyped),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequireTyped {
    id: String,
    #[serde(default = "any_version_req")]
    version: semver::VersionReq,
}

impl From<RequireWire> for Require {
    fn from(w: RequireWire) -> Self {
        match w {
            RequireWire::Bare(id) => Self {
                id,
                version: any_version_req(),
            },
            RequireWire::Typed(t) => Self {
                id: t.id,
                version: t.version,
            },
        }
    }
}

/// Custom deserializer that normalises `Vec<RequireWire>` into
/// `Vec<Require>`. Used by [`Manifest::requires`] so callers see one
/// shape regardless of which surface form the YAML used.
fn deserialize_requires<'de, D>(deserializer: D) -> Result<Vec<Require>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let wire = <Vec<RequireWire> as Deserialize>::deserialize(deserializer)?;
    Ok(wire.into_iter().map(Require::from).collect())
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
    /// i18n catalog declarations (D-NP.7, `examples/notes/user-pref.md`
    /// § Stage 5). Each language tag maps to a bundle-relative path
    /// to a flat ICU-MessageFormat JSON file. The host serves them at
    /// `GET /extensions/<id>/i18n/<lang>.json`; the client merges them
    /// into the active `IntlProvider` bundle namespaced under the
    /// extension id (`com.nube.hello.greeting`, never bare `greeting`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub i18n: Option<ContributeI18n>,
    /// Flow node-kind contributions. Consumed by
    /// `starter-ext-flow::contributed_node_kinds`.
    ///
    /// Per `DOCS/extensions/scope/FLOW-NODES.md` R-flow-node-1 this is
    /// the *one* new contribution kind the flow-node track adds — there
    /// is no second adapter, no second wire method beyond
    /// `flow.node.invoke`, no parallel streaming shape. R-flow-node-3
    /// constrains every entry's `kind` to live under the extension's
    /// reverse-DNS namespace (host-reserved `starter.*` and
    /// non-descendant ids are rejected at load time by
    /// `starter-ext-host::validate`).
    #[serde(default)]
    pub nodes: Vec<ContributeNode>,
    /// `SKILL.md` bundle directories the extension ships
    /// (DOCS/agent/SCOPE.md R-agent-4). Each entry names a bundle
    /// root directory (relative to the extension's bundle root) that
    /// contains one or more `<id>/SKILL.md` bundles. The
    /// `starter-ext-flow` adapter walks each `dir:` and feeds the
    /// discovered bundles into `SkillRegistry::extend(...)` so they
    /// land **quarantined regardless of frontmatter trust**
    /// (DOCS/agent/SKILLS.md R-skills-3 row 3): an extension cannot
    /// self-approve a skill.
    #[serde(default)]
    pub skills: Vec<ContributeSkillsDir>,
    /// Warehouse-read template contributions. Consumed by
    /// `starter-ext-host::TemplateRegistry::extend_from_record`
    /// during the host's post-commit wiring step.
    ///
    /// Each entry adds a [`TemplateSpec`](crate::warehouse::TemplateSpec)
    /// to the in-process [`TemplateRegistry`] under the extension's
    /// reverse-DNS namespace; the host's `WarehouseReadHandle` backend
    /// then resolves `warehouse.query` calls against that registry.
    /// Per
    /// [`docs/scope/extensions-north-star`](../../../../rubix/docs/scope/extensions-north-star/README.md)
    /// row 3: the manifest is the only way an extension widens the
    /// warehouse-read catalog — there is no runtime `register_template`
    /// SDK call.
    #[serde(default)]
    pub warehouse_templates: Vec<ContributeWarehouseTemplate>,
    /// Warehouse-table declarations the extension owns. The host
    /// issues `CREATE TABLE IF NOT EXISTS <sanitized_id>__<name>`
    /// at boot for each entry, then routes
    /// `WarehouseWriteHandle::insert(name, rows)` calls against
    /// the resulting table after stamping `tenant_id` from the
    /// caller. See [`ContributeWarehouseTable`].
    ///
    /// Per
    /// [`docs/scope/extensions-north-star`](../../../../rubix/docs/scope/extensions-north-star/README.md)
    /// row 5 (Phase B): the lynchpin contribution for Use-Case-B
    /// — extensions that produce ad-hoc data the host's L1/L2/L3
    /// schema can't host directly.
    #[serde(default)]
    pub warehouse_tables: Vec<ContributeWarehouseTable>,
    /// Anomaly-rule contributions. Each entry names a tool the
    /// extension exports (under its reverse-DNS namespace); the host
    /// wraps the tool dispatch in a [`ToolAnomalyRule`](
    /// `https://docs.rs/`) adapter and appends it to the cleaner's
    /// `RuleRegistry` after the builtins. See [`ContributeAnomalyRule`].
    ///
    /// Per
    /// [`docs/scope/extensions-north-star`](../../../../rubix/docs/scope/extensions-north-star/README.md)
    /// row B5 (Phase B): the lynchpin contribution for the
    /// Use-Case-B "custom rules" goal — extensions ship their own
    /// detectors that plug into the same per-row pipeline the
    /// builtin NaN/Spike/Stuck rules use.
    #[serde(default)]
    pub anomaly_rules: Vec<ContributeAnomalyRule>,
}

/// One `contributes.skills[]` entry — a directory of `SKILL.md`
/// bundles the extension ships.
///
/// The on-disk layout under `dir` is one subdirectory per skill,
/// each containing a `SKILL.md` plus any resource files listed in
/// the frontmatter (same shape as the host's own
/// `<XDG_DATA>/<binary>/skills/` directory). The
/// `starter-ext-flow` adapter passes each bundle root to
/// `starter_skills::ContributedSkill::new(bundle_root)` and then to
/// `SkillRegistry::extend(...)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributeSkillsDir {
    /// Bundle-relative path to the skills root directory
    /// (e.g. `"skills/"`). The adapter resolves this against the
    /// extension's installed bundle root before walking.
    pub dir: String,
}

/// One `contributes.warehouse_templates[]` entry — a named
/// warehouse-read template the extension adds to the host's
/// [`TemplateRegistry`](crate::warehouse).
///
/// The extension ships:
/// - a JSON Schema describing the template's bound parameters,
///   resolved at load time from `params_schema` (relative to bundle
///   root, R7: never templated at runtime);
/// - an optional SQL body resolved from `sql_file` (also R7) —
///   stored verbatim in [`TemplateSpec::sql`] for audit and admin
///   surfaces. Per row 3 the host crate does not execute the SQL;
///   the host integration crate (`rubix-agent`) is the resolver;
///   `sql_file` is descriptive.
///
/// `tables` is the declared table allowlist for the template; the
/// supervisor will cross-check this against the extension's
/// `warehouse_read` grant when per-template enforcement lands
/// alongside row 5's host dispatch wiring (today the grant's table
/// allowlist gates the namespace).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributeWarehouseTemplate {
    /// Template name. Must be the extension id or a dotted descendant
    /// (R4 namespace ownership). Host-reserved prefixes (`starter.`,
    /// `sys.`) are rejected outright at load time.
    pub name: String,

    /// Path (relative to bundle root) to a JSON Schema describing
    /// the template's bound `params`. The loader reads this file at
    /// commit time and embeds the parsed schema into the resulting
    /// [`TemplateSpec::params`](crate::warehouse::TemplateSpec).
    pub params_schema: String,

    /// Tables the template touches. Used by the supervisor to gate
    /// the call against the extension's `warehouse_read` grant
    /// (the grant's `tables: [...]` must be a superset).
    pub tables: Vec<String>,

    /// Optional path (relative to bundle root) to a SQL file holding
    /// the template body. Captured verbatim into
    /// [`TemplateSpec::sql`](crate::warehouse::TemplateSpec) for
    /// audit; R7 — the SQL is never templated at runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sql_file: Option<String>,
}

/// One `contributes.warehouse_tables[]` entry — a warehouse table
/// the extension declares for `WarehouseWriteHandle::insert(...)`.
///
/// The host (rubix-agent) issues
/// `CREATE TABLE IF NOT EXISTS <sanitized_id>__<name>` at boot
/// using the declared columns + engine. A `tenant_id String` column
/// is always prepended by the host so the per-call tenant clamp
/// has somewhere to write; the extension must not declare it
/// itself.
///
/// The type strings are passed verbatim to the warehouse engine's
/// DDL — the SPI does not enumerate types, so consumers can target
/// ClickHouse (`Float64`, `DateTime`, `String`, …) or any other
/// engine the host wires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributeWarehouseTable {
    /// Unprefixed table name. The host namespaces it as
    /// `<sanitized_extension_id>__<name>` (the extension id with
    /// dots and dashes replaced by `_`) before issuing DDL. Must
    /// match `^[a-zA-Z_][a-zA-Z0-9_]*$`; reserved name `tenant_id`
    /// is rejected (it's a column, not a table).
    pub name: String,

    /// Ordered column definitions. The host prepends a
    /// `tenant_id String` column at position 0 — the extension
    /// must not declare it.
    pub columns: Vec<TableColumn>,

    /// Columns the engine sorts/orders rows by. Required because
    /// the default ClickHouse engine (`MergeTree`) demands an
    /// `ORDER BY` clause. Names must appear in `columns` or be
    /// the host-prepended `tenant_id`.
    pub order_by: Vec<String>,

    /// Storage engine. Defaults to `MergeTree`. Consumers may
    /// override (`ReplacingMergeTree`, `SummingMergeTree`, …) when
    /// they want idempotent re-materialisation or pre-aggregation.
    /// The string is passed through verbatim — the host does not
    /// inspect engine arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,

    /// Partition-key expression (e.g. `toYYYYMM(ts)`). Optional;
    /// omitted ⇒ the host emits no `PARTITION BY` clause and the
    /// engine uses one partition per table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition_by: Option<String>,

    /// TTL expression (e.g. `ts + INTERVAL 14 DAY`). Optional;
    /// omitted ⇒ rows live indefinitely (operator-configurable
    /// retention is the alternative path).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,

    /// What kind of relation this entry describes.
    ///
    /// Defaults to [`WarehouseTableKind::Table`] — a plain table the
    /// host creates at boot via `CREATE TABLE IF NOT EXISTS`. For
    /// derived relations such as Timescale continuous aggregates,
    /// flag the entry as [`WarehouseTableKind::ContinuousAggregate`]
    /// so the host skips DDL and leaves creation to the extension's
    /// own `scripts/post-load.sql` (or equivalent). The entry stays
    /// registered so the per-call `warehouse_tables[]` allowlist
    /// still covers templates that reference it.
    #[serde(default, skip_serializing_if = "WarehouseTableKind::is_default")]
    pub kind: WarehouseTableKind,
}

/// What kind of warehouse relation a [`ContributeWarehouseTable`]
/// describes.
///
/// Controls whether the host creates the relation at boot. The
/// allowlist gate that authorises templates referencing the table
/// is unaffected — every entry is registered regardless of kind.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarehouseTableKind {
    /// A plain table. The host issues `CREATE TABLE IF NOT EXISTS`
    /// at boot.
    #[default]
    Table,
    /// A derived relation (e.g. a Timescale continuous aggregate)
    /// created by the extension's own post-load SQL. The host does
    /// NOT issue DDL — emitting a plain table would race the real
    /// materialised-view installation and leave dashboards reading
    /// an empty stub.
    ContinuousAggregate,
}

impl WarehouseTableKind {
    fn is_default(&self) -> bool {
        matches!(self, WarehouseTableKind::Table)
    }

    /// Whether the host's boot DDL step should `CREATE TABLE` for
    /// this entry. `false` for kinds the extension creates itself
    /// (e.g. [`WarehouseTableKind::ContinuousAggregate`]).
    pub fn host_manages_ddl(&self) -> bool {
        matches!(self, WarehouseTableKind::Table)
    }
}

/// One column in a [`ContributeWarehouseTable`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableColumn {
    /// Column name. Must match `^[a-zA-Z_][a-zA-Z0-9_]*$` and
    /// must not be `tenant_id` (host-reserved).
    pub name: String,

    /// Warehouse-engine column type, passed verbatim into DDL.
    /// Examples for ClickHouse: `String`, `Float64`, `Int64`,
    /// `DateTime`, `DateTime64(3)`, `Nullable(String)`.
    #[serde(rename = "type")]
    pub ty: String,

    /// Optional default expression. Passed verbatim after
    /// `DEFAULT` in the column DDL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

/// One `contributes.anomaly_rules[]` entry — a per-row anomaly
/// detector the extension contributes to the host's cleaner
/// pipeline.
///
/// The host wraps each entry's `tool_id` (which must resolve in the
/// host's tool registry — typically a verb the *same* extension
/// also contributes via `contributes.tools[]`) inside a
/// `ToolAnomalyRule` adapter and appends it to the cleaner's
/// `RuleRegistry` after the three builtins (NaN → Spike → Stuck).
/// Per-tick the cleaner invokes the tool with a `{ row,
/// window_tail }` payload and interprets the JSON response as a
/// `RuleOutcome` (`{ outcome: "ok" }` / `{ outcome: "flag",
/// quality, note? }` / `{ outcome: "drop" }`).
///
/// Per
/// [`docs/scope/extensions-north-star`](../../../../rubix/docs/scope/extensions-north-star/README.md)
/// row B5 (Phase B): the contribution slot is the only way an
/// extension extends the cleaner's per-row pipeline — there is no
/// runtime `register_rule` SDK call. Builtin and extension rules
/// share the same `AnomalyRule` trait so the cleaner doesn't know
/// (or care) where a given rule's logic actually lives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributeAnomalyRule {
    /// Stable rule id. Must be the extension id or a dotted
    /// descendant (R4 namespace ownership). Host-reserved prefixes
    /// (`starter.`, `sys.`, `builtin.`) are rejected at load time
    /// — `builtin.*` is reserved for the in-process detectors so
    /// the cleaner can short-circuit them without consulting the
    /// tool registry. Logged on every fire so operators can trace
    /// which rule produced a tag.
    pub id: String,

    /// Tool id the host dispatches against for each row. Must be a
    /// reverse-DNS string the host's tool registry can resolve.
    /// Typical shape is the same extension contributing a tool
    /// under `contributes.tools[]` whose `id` matches this field;
    /// the host does not require this — a rule can dispatch to any
    /// tool the supervisor permits, including builtins.
    pub tool_id: String,

    /// Optional ordering hint. Lower values run earlier. When
    /// absent the host appends in declaration order *after* the
    /// builtins so a manifest cannot accidentally shadow
    /// `builtin.nan`. Two entries with the same priority preserve
    /// declaration order. Negative priorities are accepted but
    /// still run after the builtins (the builtins are not
    /// addressable from this field).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
}

/// The extension's i18n block.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributeI18n {
    /// Per-language catalog paths, relative to the bundle root. Keys are
    /// BCP-47 language tags; values are paths to flat JSON catalog files
    /// (e.g. `i18n/en.json`). Lazy-loaded by the host per active locale
    /// — every catalog except the current one is left on disk until the
    /// user flips to that language (D-NP.8).
    #[serde(default)]
    pub catalogs: std::collections::BTreeMap<String, String>,
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
    /// Optional fine-grained policy-engine gate (SCOPE-EXT R15 —
    /// Phase 7d). When set, the adapter wraps the entry's handler
    /// in `with_permission(resource, action)` so the host's
    /// `PolicyEngine` decides Allow/Deny per request. The
    /// `resource` must be a kind registered in the host's
    /// `ResourceRegistry`; an unknown kind is a load-time error
    /// (symmetric with `require_role`'s unknown-role rejection).
    ///
    /// **Shared across all adapters** (REST, MCP, gRPC). The
    /// field shape lives on `AuthGate` rather than per-contribute
    /// so the three adapters consume it uniformly (SCOPE-EXT
    /// R15: "the manifest field shipped here is the same one").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission: Option<PermissionGate>,
}

/// Fine-grained policy-engine gate declared on a contribute entry's
/// [`AuthGate`].
///
/// The host's `PolicyEngine` is asked to decide Allow/Deny on every
/// request against `(resource, action)`. `resource` must be a kind
/// the host registered in its `ResourceRegistry`; the adapter
/// validates this at build time so a typo is caught at deploy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermissionGate {
    /// Resource kind (e.g. `"weather"`, `"flows"`). Must be
    /// registered in the host's `ResourceRegistry`.
    pub resource: String,
    /// Action verb on the resource (e.g. `"read"`, `"refresh"`).
    pub action: String,
}

/// One flow node-kind the extension contributes.
///
/// Per `DOCS/extensions/scope/FLOW-NODES.md` R-flow-node-1 / R-flow-node-4:
///
/// - The manifest *declares* a kind (its reverse-DNS id, JSON Schema for
///   `settings:`, optional human description file, optional facet tags,
///   advisory `streaming` flag, optional per-entry auth gate).
/// - The extension's child process *implements* the kind's body. The
///   kernel invokes the body through the single new wire method
///   `flow.node.invoke` (slice B; stage 1 hands the descriptor through
///   the dynamic registry alongside a placeholder behaviour that
///   returns the typed "no behaviour bound" error).
///
/// `deny_unknown_fields` matches every other contribution kind so a
/// manifest typo is a load-time error, never a silent ignore (parent
/// SCOPE R3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributeNode {
    /// Reverse-DNS kind id. Must be the extension id or a dotted
    /// descendant (R-flow-node-3). The loader rejects ids starting with
    /// the host-reserved `starter.` / `sys.` prefixes outright.
    pub kind: String,

    /// Path (relative to bundle root) to the JSON Schema that validates
    /// a node-instance's `settings:` body. Required: every kind ships a
    /// schema so the engine can gate publication via
    /// `DefinitionManager::publish` (R-flow-node-7). The host serves the
    /// schema file at `GET /api/node-kinds/<kind>/settings-schema`.
    pub settings_schema: String,

    /// Path (relative to bundle root) to a static markdown file
    /// describing the kind. Optional — kinds with no extra prose simply
    /// omit it; the host returns 404 from
    /// `GET /api/node-kinds/<kind>/description` in that case. R7: never
    /// templated at runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_file: Option<String>,

    /// Optional palette facet tags (`"trigger"`, `"transport"`, …).
    /// Free-form; the editor surface filters by these without the host
    /// imposing a vocabulary.
    #[serde(default)]
    pub facets: Vec<String>,

    /// Advisory flag: `true` if this kind's `invoke` produces a
    /// `Stream<Item = Event>` rather than a single response. The proxy
    /// in slice B reads this to size internal channels; it is *not* a
    /// kernel-level mode switch (R-flow-node-1 forbids inventing a new
    /// streaming shape — the existing `stream.event/end/error/cancel`
    /// notifications carry the events).
    #[serde(default)]
    pub streaming: bool,

    /// Optional per-entry auth gate. Re-uses the same [`AuthGate`] type
    /// every other contribution kind uses (R-flow-node-1: one trait,
    /// one gate vocabulary across `tools`, `cli`, `rest`, `grpc`,
    /// `workers`, `ui`, `nodes`).
    #[serde(default)]
    pub auth: AuthGate,
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
    /// How the response is rendered to stdout. Defaults to
    /// [`CliStreaming::None`] (single JSON document printed once when
    /// the handler returns). When `Stdout`, the CLI adapter renders the
    /// extension's `Stream<Item = Event>` as line-delimited JSON
    /// (one event per line) on stdout, and a `SIGINT` from the
    /// terminal becomes a `stream.cancel` notification to the child
    /// within a few hundred milliseconds (Adapter Phase 6 — SCOPE
    /// post-R13).
    #[serde(default)]
    pub streaming: CliStreaming,
    /// Optional per-entry auth gate.
    #[serde(default)]
    pub auth: AuthGate,
}

/// Streaming-render mode for a CLI contribution entry (Adapter Phase 6).
///
/// Selects how `starter-ext-cli`'s CLI adapter renders the extension's
/// `Stream<Item = Event>` on stdout. The kernel shape is uniform
/// (`stream.event` / `stream.end` / `stream.error` / `stream.cancel`
/// notifications per SCOPE post-R13); this enum picks the stdout framing
/// that wraps it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliStreaming {
    /// Single JSON document printed when the handler returns. Default.
    #[default]
    None,
    /// Newline-delimited JSON; one event per line, flushed as it
    /// arrives. `SIGINT` from the terminal becomes a `stream.cancel`
    /// notification to the child.
    Stdout,
}

impl CliStreaming {
    /// `true` for any streaming variant (today: only `Stdout`).
    pub fn is_streaming(self) -> bool {
        !matches!(self, CliStreaming::None)
    }
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
    /// How the response is rendered to the HTTP client. Defaults to
    /// [`RestStreaming::None`] (single JSON body). When `Sse`, the REST
    /// adapter renders the extension's `Stream<Item = Event>` as a
    /// `text/event-stream` response with `retry:` + default heartbeat;
    /// when `Ndjson`, as `application/x-ndjson` newline-delimited frames.
    /// A client disconnect on either streaming flavour propagates back
    /// to the child as a `stream.cancel` notification within a few
    /// hundred milliseconds (Adapter Phase 5 — SCOPE post-R13).
    #[serde(default)]
    pub streaming: RestStreaming,
    /// Optional per-entry auth gate.
    #[serde(default)]
    pub auth: AuthGate,
}

/// Streaming-render mode for a REST contribution entry (Adapter Phase 5).
///
/// Selects how `starter-ext-server`'s REST adapter renders the extension's
/// `Stream<Item = Event>` on the wire. The kernel shape is uniform
/// (`stream.event` / `stream.end` / `stream.error` / `stream.cancel`
/// notifications per SCOPE post-R13); this enum picks the HTTP framing
/// that wraps it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestStreaming {
    /// Single response body; not a stream. Default.
    #[default]
    None,
    /// `Content-Type: text/event-stream` with `retry:` + heartbeat.
    Sse,
    /// `Content-Type: application/x-ndjson` newline-delimited JSON frames.
    Ndjson,
}

impl RestStreaming {
    /// `true` for any streaming variant (`Sse` or `Ndjson`).
    pub fn is_streaming(self) -> bool {
        !matches!(self, RestStreaming::None)
    }
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
///
/// Workers are **periodic, not jobs** (Adapter Phase 7, SCOPE non-goal
/// mirror): the adapter does not maintain a shared queue and does not
/// fan-out across hosts; each worker is a self-pacing task that runs at
/// `interval_seconds` (± up to `jitter_seconds`) and, when its handler
/// returns an error, backs off per [`OnErrorPolicy`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributeWorker {
    /// Worker id; must be the extension id or a dotted descendant (R4).
    pub id: String,
    /// Nominal interval between two successive invocations of the
    /// handler, in seconds. The adapter schedules
    /// `next_due = now + interval_seconds + rand(0..=jitter_seconds)`
    /// after every (successful or errored-and-retried-out) run.
    pub interval_seconds: u32,
    /// Maximum amount of uniform random jitter, in seconds, added on
    /// top of `interval_seconds` when computing `next_due`. Zero
    /// (the default) gives a strictly periodic cadence; non-zero
    /// spreads the load across replicas that come up at the same time.
    #[serde(default)]
    pub jitter_seconds: u32,
    /// Per-worker error backoff policy. See [`OnErrorPolicy`].
    #[serde(default)]
    pub on_error: OnErrorPolicy,
    /// Path (relative to bundle root) to the worker's static markdown
    /// description. R7.
    pub description_file: String,
    /// Optional cap on concurrent executions of this worker (default 1).
    /// `> 1` is reserved for future use; the v0.1 scheduler treats
    /// every worker as `1` because there is no shared queue and no
    /// fan-out (SCOPE non-goal).
    #[serde(default = "default_one")]
    pub concurrency: u32,
    /// Optional per-entry auth gate (workers run under a system principal
    /// if omitted; the gate is for operator-triggered runs).
    #[serde(default)]
    pub auth: AuthGate,
}

/// Error-recovery policy for a single periodic worker.
///
/// When the handler returns `Err`, the scheduler:
///
/// 1. Records the error and the failing attempt on the worker's state
///    (surfaced as `last_error` and `attempt` on
///    `GET /extensions/<id>`).
/// 2. If `retry == RetryStrategy::Exponential` **and**
///    `attempt < max_attempts`: schedules the next run at
///    `now + min(max_backoff_ms, initial_backoff_ms * 2^(attempt-1))`,
///    keeping the same cadence afterwards (so an error costs at most
///    one delayed cycle).
/// 3. If `attempt >= max_attempts`: the worker stops scheduling new
///    runs until the extension is re-enabled or `tick_now(id)` is
///    invoked (testing seam). The error is sticky on `last_error`.
/// 4. If `retry == RetryStrategy::Never`: any error transitions
///    straight to the "stopped, sticky error" state on attempt 1.
///
/// Defaults are conservative: exponential retry with 1s/60s bounds
/// and 5 attempts. A worker that loops on a permanently broken
/// downstream stops by itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OnErrorPolicy {
    /// Retry strategy. `Exponential` (the default) doubles the
    /// backoff between attempts within `[initial_backoff_ms, max_backoff_ms]`.
    #[serde(default)]
    pub retry: RetryStrategy,
    /// First backoff after the first failure, in milliseconds.
    #[serde(default = "default_initial_backoff_ms")]
    pub initial_backoff_ms: u32,
    /// Cap on the exponential backoff, in milliseconds.
    #[serde(default = "default_max_backoff_ms")]
    pub max_backoff_ms: u32,
    /// Cap on the number of consecutive failures before the worker
    /// goes into the sticky "stopped" state. Counts reset to zero
    /// after the first successful run.
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
}

impl Default for OnErrorPolicy {
    fn default() -> Self {
        Self {
            retry: RetryStrategy::default(),
            initial_backoff_ms: default_initial_backoff_ms(),
            max_backoff_ms: default_max_backoff_ms(),
            max_attempts: default_max_attempts(),
        }
    }
}

/// Retry shape for [`OnErrorPolicy`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryStrategy {
    /// Exponential backoff with the bounds declared in
    /// [`OnErrorPolicy::initial_backoff_ms`] / `max_backoff_ms`.
    #[default]
    Exponential,
    /// No retries at all. The first error stops the worker; operators
    /// re-enable through the admin route.
    Never,
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
fn default_initial_backoff_ms() -> u32 {
    1_000
}
fn default_max_backoff_ms() -> u32 {
    60_000
}
fn default_max_attempts() -> u32 {
    5
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
      interval_seconds: 300
      jitter_seconds: 30
      description_file: c.md
      on_error:
        retry: exponential
        initial_backoff_ms: 500
        max_backoff_ms: 30000
        max_attempts: 6
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
    fn anomaly_rules_round_trip_with_priority() {
        let yaml = r#"
v: 1
id: com.acme.weather
version: 0.0.1
display_name: "Weather"
runtime: { kind: builtin, crate_name: weather }
contributes:
  anomaly_rules:
    - id: com.acme.weather.spike
      tool_id: com.acme.weather.spike_check
      priority: 10
    - id: com.acme.weather.stale
      tool_id: com.acme.weather.stale_check
"#;
        let m: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(m.contributes.anomaly_rules.len(), 2);
        assert_eq!(m.contributes.anomaly_rules[0].id, "com.acme.weather.spike");
        assert_eq!(
            m.contributes.anomaly_rules[0].tool_id,
            "com.acme.weather.spike_check"
        );
        assert_eq!(m.contributes.anomaly_rules[0].priority, Some(10));
        assert_eq!(m.contributes.anomaly_rules[1].priority, None);
    }

    #[test]
    fn anomaly_rules_default_to_empty() {
        let yaml = r#"
v: 1
id: com.acme.no-rules
version: 0.0.1
display_name: "NoRules"
runtime: { kind: builtin, crate_name: no_rules }
contributes: {}
"#;
        let m: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert!(m.contributes.anomaly_rules.is_empty());
    }

    #[test]
    fn cli_streaming_modes_parse() {
        let yaml = r#"
v: 1
id: com.acme.clistream
version: 0.0.1
display_name: "CliStream"
runtime: { kind: builtin, crate_name: clistream }
contributes:
  cli:
    - id: com.acme.clistream.tail
      command: clistream-tail
      description_file: c.md
      args_schema: a.json
      streaming: stdout
    - id: com.acme.clistream.once
      command: clistream-once
      description_file: c.md
      args_schema: a.json
"#;
        let m: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(m.contributes.cli[0].streaming, CliStreaming::Stdout);
        assert_eq!(m.contributes.cli[1].streaming, CliStreaming::None);
        assert!(m.contributes.cli[0].streaming.is_streaming());
        assert!(!m.contributes.cli[1].streaming.is_streaming());
    }

    #[test]
    fn rest_streaming_modes_parse() {
        let yaml = r#"
v: 1
id: com.acme.stream
version: 0.0.1
display_name: "Stream"
runtime: { kind: builtin, crate_name: stream }
contributes:
  rest:
    - id: com.acme.stream.live
      method: GET
      path: /live/sse
      description_file: c.md
      streaming: sse
    - id: com.acme.stream.tail
      method: GET
      path: /live/ndjson
      description_file: c.md
      streaming: ndjson
    - id: com.acme.stream.now
      method: GET
      path: /now
      description_file: c.md
"#;
        let m: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(m.contributes.rest[0].streaming, RestStreaming::Sse);
        assert_eq!(m.contributes.rest[1].streaming, RestStreaming::Ndjson);
        assert_eq!(m.contributes.rest[2].streaming, RestStreaming::None);
        assert!(m.contributes.rest[0].streaming.is_streaming());
        assert!(!m.contributes.rest[2].streaming.is_streaming());
    }

    #[test]
    fn requires_accepts_bare_string_and_typed_struct() {
        let yaml = r#"
v: 1
id: com.acme.mixed
version: 0.0.1
display_name: "Mixed"
runtime: { kind: builtin, crate_name: mixed }
requires:
  - "@nube/starter-ui-core/preferences"
  - { id: starter.spi.tool, version: "^1" }
"#;
        let m: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(m.requires.len(), 2);
        assert_eq!(m.requires[0].id, "@nube/starter-ui-core/preferences");
        assert_eq!(m.requires[0].version, semver::VersionReq::STAR);
        assert_eq!(m.requires[1].id, "starter.spi.tool");
    }

    #[test]
    fn contributes_i18n_catalogs_parse() {
        let yaml = r#"
v: 1
id: com.acme.localised
version: 0.0.1
display_name: "Localised"
runtime: { kind: builtin, crate_name: localised }
contributes:
  i18n:
    catalogs:
      en: i18n/en.json
      es: i18n/es.json
"#;
        let m: Manifest = serde_yaml::from_str(yaml).unwrap();
        let i18n = m.contributes.i18n.as_ref().expect("i18n block parsed");
        assert_eq!(
            i18n.catalogs.get("en").map(String::as_str),
            Some("i18n/en.json")
        );
        assert_eq!(
            i18n.catalogs.get("es").map(String::as_str),
            Some("i18n/es.json")
        );
    }

    #[test]
    fn contributes_skills_dir_parses() {
        // R-agent-4 + Phase 6: a manifest may declare one or more
        // `skills: [{ dir: ... }]` entries that the `starter-ext-flow`
        // adapter walks and feeds into `SkillRegistry::extend(...)`.
        let yaml = r#"
v: 1
id: com.acme.skilled
version: 0.0.1
display_name: "Skilled"
runtime: { kind: builtin, crate_name: skilled }
contributes:
  skills:
    - dir: skills/
    - dir: more-skills/
"#;
        let m: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(m.contributes.skills.len(), 2);
        assert_eq!(m.contributes.skills[0].dir, "skills/");
        assert_eq!(m.contributes.skills[1].dir, "more-skills/");
    }

    #[test]
    fn contributes_nodes_parses() {
        // R-flow-node-1: `contributes.nodes` is the one new contribution
        // kind the flow-node track adds. `description_file`, `facets`,
        // `streaming`, and `auth` are `#[serde(default)]`; `kind` and
        // `settings_schema` are required.
        let yaml = r#"
v: 1
id: com.nube.mqtt
version: 0.1.0
display_name: "MQTT"
runtime: { kind: process, bin: ./bin/mqtt-driver }
contributes:
  nodes:
    - kind: com.nube.mqtt.publish
      settings_schema: schemas/publish.json
      description_file: docs/publish.md
      facets: ["transport", "io"]
      streaming: false
    - kind: com.nube.mqtt.subscribe
      settings_schema: schemas/subscribe.json
      streaming: true
      auth: { require_role: admin }
"#;
        let m: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(m.contributes.nodes.len(), 2);
        let pub_ = &m.contributes.nodes[0];
        assert_eq!(pub_.kind, "com.nube.mqtt.publish");
        assert_eq!(pub_.settings_schema, "schemas/publish.json");
        assert_eq!(pub_.description_file.as_deref(), Some("docs/publish.md"));
        assert_eq!(pub_.facets, vec!["transport", "io"]);
        assert!(!pub_.streaming);
        assert!(pub_.auth.require_role.is_none());

        let sub = &m.contributes.nodes[1];
        assert_eq!(sub.kind, "com.nube.mqtt.subscribe");
        assert!(sub.description_file.is_none());
        assert!(sub.facets.is_empty());
        assert!(sub.streaming);
        assert_eq!(sub.auth.require_role.as_deref(), Some("admin"));
    }

    #[test]
    fn contributes_nodes_rejects_unknown_field() {
        // `deny_unknown_fields` mirrors every other contribution kind.
        let yaml = r#"
v: 1
id: com.nube.mqtt
version: 0.1.0
display_name: "MQTT"
runtime: { kind: process, bin: ./bin/mqtt-driver }
contributes:
  nodes:
    - kind: com.nube.mqtt.publish
      settings_schema: schemas/publish.json
      unexpected_field: 1
"#;
        assert!(serde_yaml::from_str::<Manifest>(yaml).is_err());
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
