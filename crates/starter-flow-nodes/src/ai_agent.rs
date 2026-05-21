//! `ai-agent` — the LLM-loop node kind (Phase 4).
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "R7 — The AI
//! agent is a node kind, not a runtime" and § "Phase 4 — `ai-agent`
//! node kind (D1 resolution)". The body lifts Codeless's `Runner`
//! shape — turn-based LLM-loop with tool-dispatch — and routes every
//! model call through [`starter_spi::ai::AiRunner`]. adk-rust stays
//! out of the workspace dep tree (D-F4.2); the
//! `starter_flow_nodes_with_ai_agent_feature_does_not_pull_adk_rust`
//! integration test enforces this on the opt-in feature path.
//!
//! SCOPE rules honoured:
//!
//! - **R1 — Everything is a Node.** `ai-agent` is the canonical
//!   bridge between the flow graph and the LLM seam.
//! - **R2 — One write chokepoint.** The body returns its output
//!   [`SlotMap`]; the propagator funnels every value through the
//!   single `GraphStore::write_slot` call. The body never writes
//!   a slot directly.
//! - **R5 — Stateless behaviours.** `&self`, never `&mut self`. The
//!   registries are `Arc<dyn …>` — shared, immutable, host-built.
//! - **R8 — Nodes are not Tools; Tools are one node kind.** When
//!   the model emits a tool-call, the body dispatches it through
//!   the same `ToolRegistry::lookup` chokepoint that the
//!   `tool-call` body uses (D-F4.9).
//! - **R10 — Reverse-DNS ids.** [`KIND_ID`] under `starter.flow.*`.
//!   Provider ids are validated as [`KindId`] reverse-DNS strings.
//! - **R12 observability.** Every invocation opens an
//!   `ai_agent.invoke` tracing span recording `(node_id,
//!   provider_id, principal_id_hash, run_id, skill_id_or_none,
//!   turn_count, tool_call_count, cancel_observed)`.
//! - **R13 cancellation.** [`NodeCtx::cancel`] races the LLM loop
//!   via a `CancelAdapter` wrapping the engine's seam as
//!   `starter_spi::ai::Cancel`. Cancel-to-exit ≤ 200 ms (D-F4.7).
//! - **D-F4.5 tools intersection.** Effective allowlist =
//!   host `ToolRegistry` ∩ skill `allowed_tools` if `Selected` ∩
//!   node `config.allowed_tools` if declared. Empty intersection
//!   surfaces `NodeError::Domain { code: "no_tools_visible" }`.
//! - **D-F4.6 sessions.** [`SessionMode`] config slot drives
//!   `SessionId::for_ai_agent_node(...)`; an attached
//!   `SessionStore` persists the transcript across invocations.
//! - **D-F4.8 explicit provider.** `provider_id` config slot is
//!   mandatory; missing or unregistered surfaces typed
//!   `NodeError::Domain`. No implicit default.

use std::collections::{BTreeSet, HashMap};
use std::pin::Pin;
use std::sync::{Arc, LazyLock};

use async_trait::async_trait;
use schemars::{schema::RootSchema, JsonSchema};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::field;

use starter_flow_spi::ai_runner::AiRunnerRegistry;
use starter_flow_spi::flow::{FlowId, SessionId, SessionMode, SessionRecord, SessionStore};
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue};
use starter_flow_spi::skill::SkillSelection;
use starter_flow_spi::{Cancel as FlowCancel, Principal};
use starter_spi::ai::{
    AiRunner, Cancel as AiCancel, CliCfg, Event, HistoryMessage, RestCfg, RunnerInput, ToolChoice,
    ToolDef, ToolUse,
};

use crate::tool_registry::ToolRegistry;

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace.
pub const KIND_ID: &str = "starter.flow.ai-agent";

/// Static metadata for the catalog / discovery surface. Help text is
/// resolved through `starter-i18n`; see `crates/starter-i18n/catalogs/`.
pub const DESCRIPTOR: starter_flow_spi::node::NodeDescriptor =
    starter_flow_spi::node::NodeDescriptor::new(
        KIND_ID,
        "starter.flow.node.ai-agent.label",
        "starter.flow.node.ai-agent.summary",
        "starter.flow.node.ai-agent.help",
    );

/// Mandatory config slot carrying the reverse-DNS `provider_id`.
pub const PROVIDER_ID_SLOT: &str = "provider_id";

/// Optional config slot carrying the system prompt.
pub const SYSTEM_PROMPT_SLOT: &str = "system_prompt";

/// Optional config slot carrying a node-local tool allowlist
/// (JSON array of reverse-DNS tool ids).
pub const ALLOWED_TOOLS_SLOT: &str = "allowed_tools";

/// Optional config slot carrying the [`SessionMode`] string
/// (`"fresh_per_invocation"` / `"reuse_across_run"` /
/// `"reuse_across_flow"`). Absent defaults to
/// [`SessionMode::FreshPerInvocation`].
pub const SESSION_MODE_SLOT: &str = "session_mode";

/// Optional config slot carrying a per-node skill-selection
/// override (non-empty reverse-DNS string). Phase 4 logs a warn
/// and falls back to `ctx.skill`; the real per-node override
/// lands with the future `starter-skills` crate.
pub const SKILL_HINT_SLOT: &str = "skill_hint";

/// Optional config slot selecting the `RunnerInput` variant the
/// body hands to the resolved runner. Values: `"rest"` (default)
/// or `"cli"`. See D-F5.6 in `DOCS/flow/scope/SCOPE.md`.
pub const INPUT_KIND_SLOT: &str = "input_kind";

/// Input slot carrying the user message text.
pub const INPUT_SLOT: &str = "input";

/// Output slot the body writes the model's final assistant text into.
pub const OUTPUT_SLOT: &str = "output";

/// Output slot the body writes the observed turn count into.
pub const TURN_COUNT_SLOT: &str = "turn_count";

/// Hard upper bound on the LLM loop's turn count. Mirrors the
/// Phase 2 D1a hop-budget posture; future revisions may make this
/// a `RunOpts` config slot.
pub const MAX_TURNS: u32 = 64;

/// Publish-time configuration carried on an `ai-agent` node's
/// `settings:` field in a flow body. Per
/// [`DOCS/flow/scope/settings.md`](../../../DOCS/flow/scope/settings.md)
/// Phase S-4: the kind exposes a typed schema derived from this
/// struct via [`schemars`] so editor surfaces (REST, CLI, UI canvas)
/// can validate drafts and generate forms without re-implementing
/// per-kind knowledge.
///
/// Runtime [`AiAgent::invoke`] keeps reading from the
/// `PROVIDER_ID_SLOT` / `SYSTEM_PROMPT_SLOT` / … input slots via
/// [`AgentConfig::from_input`] — schema is the *publish-time gate*,
/// not a second runtime mechanism (settings.md "What does NOT
/// land"). Once `TopologyResolver::resolve` lands
/// (see `DOCS/flow/scope/hot-reload.md` HR5) it will project the
/// fields here into the matching config slots before invocation.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AiAgentSettings {
    /// Reverse-DNS provider id the body resolves against the
    /// [`AiRunnerRegistry`] (e.g. `anthropic/claude-sonnet-4-6`).
    /// Required at publish time — runtime still enforces the same
    /// constraint via `provider_id_required` / `provider_id_invalid`
    /// domain errors.
    pub provider_id: String,

    /// Optional system prompt prepended to the conversation.
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Node-local tool allowlist (reverse-DNS tool ids). Intersected
    /// with the host `ToolRegistry` and the active skill's
    /// `allowed_tools` per D-F4.5; an empty intersection surfaces
    /// `no_tools_visible` at runtime.
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Session-continuity policy. One of
    /// `"fresh_per_invocation"` | `"reuse_across_run"` |
    /// `"reuse_across_flow"`. Absent defaults to
    /// `"fresh_per_invocation"` (matches
    /// [`SessionMode::FreshPerInvocation`](starter_flow_spi::flow::SessionMode)).
    #[serde(default)]
    pub session_mode: Option<String>,

    /// `RunnerInput` transport. `"rest"` (default) drives the
    /// in-body turn loop with host tool dispatch;
    /// `"cli"` hands a `RunnerInput::Cli` to the runner once and
    /// lets the CLI binary own the tool loop (D-F5.6).
    #[serde(default)]
    pub input_kind: Option<String>,
}

/// Derived JSON Schema for [`AiAgentSettings`]. Returned by
/// reference from [`AiAgent::config_schema`]; built once per process
/// via [`LazyLock`].
pub static AI_AGENT_SETTINGS_SCHEMA: LazyLock<RootSchema> =
    LazyLock::new(|| schemars::schema_for!(AiAgentSettings));

/// Which `RunnerInput` variant the body hands to the resolved
/// runner. CLI-shape runners (e.g. `ClaudeRunner`) reject
/// `RunnerInput::Rest` with `RunnerError::WrongInputKind`; this
/// enum selects between the two transports up front.
///
/// Per D-F5.6, the CLI path drives the runner once and surfaces
/// `RunResult::text` as the body's output: the CLI binary runs
/// its own internal tool-call loop, so the body's `ToolRegistry`
/// dispatch path is skipped and `turn_count = 1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentInputKind {
    /// Hand a `RunnerInput::Rest` to the runner and drive the
    /// in-body turn loop with host tool dispatch (Phase 4 default).
    #[default]
    Rest,
    /// Hand a `RunnerInput::Cli` to the runner once; the CLI
    /// binary owns the tool loop. Skips the host `ToolRegistry`
    /// dispatch path.
    Cli,
}

impl AgentInputKind {
    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "rest" => Some(Self::Rest),
            "cli" => Some(Self::Cli),
            _ => None,
        }
    }
}

/// In-memory [`AiRunnerRegistry`] populated at engine-build time.
#[derive(Default)]
pub struct StaticAiRunnerRegistry {
    runners: HashMap<KindId, Arc<dyn AiRunner>>,
}

impl StaticAiRunnerRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a runner under the given reverse-DNS provider id.
    pub fn register(
        &mut self,
        provider_id: KindId,
        runner: Arc<dyn AiRunner>,
    ) -> Option<Arc<dyn AiRunner>> {
        self.runners.insert(provider_id, runner)
    }
}

impl AiRunnerRegistry for StaticAiRunnerRegistry {
    fn lookup(&self, provider_id: &KindId) -> Option<Arc<dyn AiRunner>> {
        self.runners.get(provider_id).cloned()
    }
}

/// The `ai-agent` node body.
pub struct AiAgent {
    tools: Arc<dyn ToolRegistry>,
    runners: Arc<dyn AiRunnerRegistry>,
    sessions: Option<Arc<dyn SessionStore>>,
    kind_id: KindId,
    /// Default `provider_id` used when the [`PROVIDER_ID_SLOT`] is
    /// not present in the invocation's input map. Set via
    /// [`Self::with_provider_id`].
    ///
    /// **Phase 4 workaround.** The Phase 2 propagator only routes
    /// declared trigger slots into a node body's `input` map. A
    /// `provider_id` declared as a non-trigger config slot is
    /// invisible to the body. Pending a NodeCtx extension that
    /// exposes the graph store (so the body can read config slots
    /// directly), instances pin the provider here at construction
    /// time. The [`PROVIDER_ID_SLOT`] still takes precedence when
    /// present so per-invocation overrides keep working.
    default_provider_id: Option<KindId>,
    /// Default [`AgentInputKind`] used when the [`INPUT_KIND_SLOT`]
    /// is not present in the invocation's input map. Set via
    /// [`Self::with_input_kind`]. Mirrors the [`default_provider_id`]
    /// Phase-4 workaround posture — the propagator only routes
    /// declared trigger slots, so CLI-shape runners need the kind
    /// pinned at construction time. `None` means [`AgentInputKind::Rest`].
    default_input_kind: Option<AgentInputKind>,
}

impl AiAgent {
    /// Construct an `ai-agent` body with the given tool + runner
    /// registries. No session store attached; no default
    /// `provider_id`.
    pub fn new(tools: Arc<dyn ToolRegistry>, runners: Arc<dyn AiRunnerRegistry>) -> Self {
        Self {
            tools,
            runners,
            sessions: None,
            kind_id: KindId::new(KIND_ID).expect("KIND_ID is a valid reverse-DNS"),
            default_provider_id: None,
            default_input_kind: None,
        }
    }

    /// Attach a [`SessionStore`] for session-mode persistence.
    pub fn with_session_store(mut self, sessions: Arc<dyn SessionStore>) -> Self {
        self.sessions = Some(sessions);
        self
    }

    /// Set the default `provider_id`, used when the per-invocation
    /// [`PROVIDER_ID_SLOT`] is absent.
    pub fn with_provider_id(mut self, provider_id: KindId) -> Self {
        self.default_provider_id = Some(provider_id);
        self
    }

    /// Set the default [`AgentInputKind`], used when the
    /// per-invocation [`INPUT_KIND_SLOT`] is absent. Required for
    /// CLI-shape runners (e.g. `ClaudeRunner`) until the propagator
    /// can route arbitrary config slots. The [`INPUT_KIND_SLOT`]
    /// still takes precedence when present.
    pub fn with_input_kind(mut self, kind: AgentInputKind) -> Self {
        self.default_input_kind = Some(kind);
        self
    }
}

#[async_trait]
impl NodeBehavior for AiAgent {
    fn kind_id(&self) -> &KindId {
        &self.kind_id
    }

    fn config_schema(&self) -> &'static RootSchema {
        &AI_AGENT_SETTINGS_SCHEMA
    }

    async fn invoke(&self, ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let principal = system_admin_principal();
        let principal_hash = principal_id_hash(&principal);

        let cfg = AgentConfig::from_input(
            &input,
            self.default_provider_id.as_ref(),
            self.default_input_kind,
        )?;
        let span = tracing::info_span!(
            "ai_agent.invoke",
            node_id = %ctx.node,
            provider_id = %cfg.provider_id,
            principal_id_hash = %principal_hash,
            run_id = %ctx.run,
            skill_id_or_none = field::Empty,
            turn_count = field::Empty,
            tool_call_count = field::Empty,
            cancel_observed = field::Empty,
        );
        match ctx.skill {
            SkillSelection::Selected { skill_id, .. } => {
                span.record("skill_id_or_none", tracing::field::display(skill_id));
            }
            _ => {
                span.record("skill_id_or_none", "none");
            }
        }

        let _enter = span.enter();

        if let Some(hint) = cfg.skill_hint.as_ref() {
            tracing::warn!(
                skill_hint = %hint,
                "ai_agent: skill_hint override is no-op until starter-skills lands; using ctx.skill"
            );
        }

        // Phase 4b on-mount verification (R-skills-7). Before the body
        // does anything else with the frozen `SkillSelection`, read
        // every mounted resource off disk and rehash it against the
        // `ResourceRef.content_hash` captured at selection time. A
        // racing `SkillRegistry::reload()` can swap the on-disk bytes
        // underneath an in-flight run; without this check the model
        // would silently see the drifted bytes. The mismatch arm
        // surfaces a typed `Domain { code: "skill_resource_hash_mismatch" }`
        // so run telemetry shows a structured node failure rather than
        // an opaque backend error.
        if let SkillSelection::Selected { resources, skill_id, .. } = ctx.skill {
            for resource in resources {
                if let Err(err) = starter_skills::read_and_verify(resource) {
                    return Err(map_resource_mount_error(skill_id.as_str(), err));
                }
            }
        }

        let effective_tools = compute_visible_tools(&*self.tools, ctx.skill, &cfg.allowed_tools)?;
        let session_id = SessionId::for_ai_agent_node(
            cfg.session_mode,
            ctx.node,
            ctx.run,
            cfg.flow_id_fallback(),
            &principal,
        );

        let prior_history = match (&self.sessions, cfg.session_mode, cfg.input_kind) {
            (
                Some(store),
                SessionMode::ReuseAcrossRun | SessionMode::ReuseAcrossFlow,
                AgentInputKind::Rest,
            ) => store
                .get(session_id)
                .await
                .map_err(|e| NodeError::Backend(format!("session_store.get: {e}")))?
                .map(|rec| decode_history(&rec.body))
                .unwrap_or_default(),
            _ => Vec::new(),
        };

        let cancel = CancelAdapter::new(ctx.cancel);
        let provider_runner =
            self.runners
                .lookup(&cfg.provider_id)
                .ok_or_else(|| NodeError::Domain {
                    code: "provider_not_registered",
                    message: format!("no AiRunner registered for {}", cfg.provider_id),
                })?;

        let loop_outcome = match cfg.input_kind {
            AgentInputKind::Rest => {
                run_agent_loop(LoopInputs {
                    runner: &*provider_runner,
                    tools: &*self.tools,
                    visible_tools: &effective_tools,
                    session_id,
                    cancel: &cancel,
                    system_prompt: cfg.system_prompt.as_deref(),
                    initial_user_msg: cfg.user_input,
                    prior_history,
                })
                .await
            }
            AgentInputKind::Cli => {
                run_cli_once(CliInputs {
                    runner: &*provider_runner,
                    session_id,
                    cancel: &cancel,
                    system_prompt: cfg.system_prompt.as_deref(),
                    user_input: cfg.user_input,
                })
                .await
            }
        };

        if let (Some(store), SessionMode::ReuseAcrossRun | SessionMode::ReuseAcrossFlow) =
            (&self.sessions, cfg.session_mode)
        {
            if let Some(history) = loop_outcome.history_snapshot.as_ref() {
                let record =
                    SessionRecord::new(session_id, principal.clone(), encode_history(history));
                store
                    .put(session_id, record)
                    .await
                    .map_err(|e| NodeError::Backend(format!("session_store.put: {e}")))?;
            }
        }

        span.record("turn_count", loop_outcome.turn_count);
        span.record("tool_call_count", loop_outcome.tool_call_count);
        span.record("cancel_observed", cancel.observed());

        let final_text = loop_outcome.result?;
        let mut out = SlotMap::new();
        out.insert(OUTPUT_SLOT.to_string(), SlotValue::String(final_text));
        out.insert(
            TURN_COUNT_SLOT.to_string(),
            SlotValue::Int(loop_outcome.turn_count as i64),
        );
        Ok(out)
    }
}

// ---------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------

/// Translate a [`starter_skills::ResourceMountError`] surfaced by the
/// Phase 4b on-mount check into a typed [`NodeError`] the run telemetry
/// surface can render.
///
/// Hash-mismatch is the load-bearing case: it surfaces under the
/// reverse-DNS code `skill_resource_hash_mismatch` so operators can
/// alert on it specifically. The other two variants
/// (`UnsupportedScheme`, `Io`) bucket into distinct codes so the
/// failure mode is still legible at the telemetry layer.
fn map_resource_mount_error(
    skill_id: &str,
    err: starter_skills::ResourceMountError,
) -> NodeError {
    use starter_skills::ResourceMountError as E;
    match err {
        E::HashMismatch { uri, expected, actual } => NodeError::Domain {
            code: "skill_resource_hash_mismatch",
            message: format!(
                "skill `{skill_id}` resource `{uri}` hash drifted between selection and mount: \
                 expected {expected}, got {actual}"
            ),
        },
        E::UnsupportedScheme { uri } => NodeError::Domain {
            code: "skill_resource_unsupported_scheme",
            message: format!(
                "skill `{skill_id}` resource `{uri}` uses an unsupported URI scheme \
                 (v1 supports file:// only)"
            ),
        },
        E::Io { path, source } => NodeError::Domain {
            code: "skill_resource_io",
            message: format!(
                "skill `{skill_id}` resource `{}` failed to read at mount: {source}",
                path.display()
            ),
        },
        // `ResourceMountError` is `#[non_exhaustive]` upstream. Future
        // variants surface here as a generic backend error until a
        // typed mapping is added; the run still fails closed.
        other => NodeError::Backend(format!(
            "skill `{skill_id}` resource mount: {other}"
        )),
    }
}

struct AgentConfig {
    provider_id: KindId,
    system_prompt: Option<String>,
    allowed_tools: Option<BTreeSet<KindId>>,
    session_mode: SessionMode,
    skill_hint: Option<String>,
    user_input: String,
    input_kind: AgentInputKind,
}

impl AgentConfig {
    fn from_input(
        input: &SlotMap,
        default_provider_id: Option<&KindId>,
        default_input_kind: Option<AgentInputKind>,
    ) -> Result<Self, NodeError> {
        let provider_id = match read_string(input, PROVIDER_ID_SLOT) {
            Some(s) => KindId::new(s).map_err(|e| NodeError::Domain {
                code: "provider_id_invalid",
                message: format!("invalid reverse-DNS provider id: {e}"),
            })?,
            None => default_provider_id
                .cloned()
                .ok_or_else(|| NodeError::Domain {
                    code: "provider_id_required",
                    message: format!(
                    "{PROVIDER_ID_SLOT} config slot is required (and no default set on AiAgent)"
                ),
                })?,
        };

        let system_prompt = read_string(input, SYSTEM_PROMPT_SLOT);

        let allowed_tools = match input.get(ALLOWED_TOOLS_SLOT) {
            Some(SlotValue::Json(serde_json::Value::Array(arr))) if !arr.is_empty() => {
                let mut set = BTreeSet::new();
                for entry in arr {
                    let s = entry.as_str().ok_or_else(|| NodeError::Domain {
                        code: "allowed_tools_invalid",
                        message: format!("{ALLOWED_TOOLS_SLOT} entries must be strings"),
                    })?;
                    let id = KindId::new(s.to_string()).map_err(|e| NodeError::Domain {
                        code: "allowed_tools_invalid",
                        message: format!("invalid tool id `{s}`: {e}"),
                    })?;
                    set.insert(id);
                }
                Some(set)
            }
            None
            | Some(SlotValue::Json(serde_json::Value::Null))
            | Some(SlotValue::Json(serde_json::Value::Array(_))) => None,
            Some(_) => {
                return Err(NodeError::Domain {
                    code: "allowed_tools_invalid",
                    message: format!("{ALLOWED_TOOLS_SLOT} must be a JSON array of strings"),
                });
            }
        };

        let session_mode = match read_string(input, SESSION_MODE_SLOT).as_deref() {
            None | Some("") | Some("fresh_per_invocation") => SessionMode::FreshPerInvocation,
            Some("reuse_across_run") => SessionMode::ReuseAcrossRun,
            Some("reuse_across_flow") => SessionMode::ReuseAcrossFlow,
            Some(other) => {
                return Err(NodeError::Domain {
                    code: "session_mode_invalid",
                    message: format!("unknown session_mode `{other}`"),
                });
            }
        };

        let skill_hint = read_string(input, SKILL_HINT_SLOT).filter(|s| !s.is_empty());
        let user_input = read_string(input, INPUT_SLOT).unwrap_or_default();

        let input_kind = match read_string(input, INPUT_KIND_SLOT).as_deref() {
            None | Some("") => default_input_kind.unwrap_or_default(),
            Some(raw) => AgentInputKind::parse(raw).ok_or_else(|| NodeError::Domain {
                code: "input_kind_invalid",
                message: format!("unknown {INPUT_KIND_SLOT} `{raw}`; expected `rest` or `cli`"),
            })?,
        };

        Ok(Self {
            provider_id,
            system_prompt,
            allowed_tools,
            session_mode,
            skill_hint,
            user_input,
            input_kind,
        })
    }

    fn flow_id_fallback(&self) -> FlowId {
        // NodeCtx does not yet carry the owning FlowId; use a stable
        // placeholder so the ReuseAcrossFlow derivation is at least
        // deterministic per (node_id, principal). Threading the real
        // FlowId lands with the Phase 5 NodeCtx extension that also
        // carries the Principal.
        FlowId::new("starter.flow.ai-agent.unknown-flow").expect("static valid")
    }
}

fn read_string(input: &SlotMap, slot: &str) -> Option<String> {
    match input.get(slot)? {
        SlotValue::String(s) => Some(s.clone()),
        SlotValue::Json(serde_json::Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn compute_visible_tools(
    registry: &dyn ToolRegistry,
    skill: &SkillSelection,
    node_allowed: &Option<BTreeSet<KindId>>,
) -> Result<Vec<ToolBinding>, NodeError> {
    let skill_layer: Option<BTreeSet<KindId>> = match skill {
        SkillSelection::Selected { allowed_tools, .. } => {
            Some(allowed_tools.iter().cloned().collect())
        }
        _ => None,
    };

    let intersection: Option<BTreeSet<KindId>> = match (skill_layer.as_ref(), node_allowed.as_ref())
    {
        (Some(a), Some(b)) => Some(a.intersection(b).cloned().collect()),
        (Some(a), None) => Some(a.clone()),
        (None, Some(b)) => Some(b.clone()),
        (None, None) => None,
    };

    let Some(set) = intersection else {
        // No allowlist constraints anywhere. Body advertises an empty
        // tool list to the model; if the model still emits a tool call
        // by name the dispatcher resolves it against the host registry
        // at call time.
        return Ok(Vec::new());
    };

    if set.is_empty() {
        return Err(NodeError::Domain {
            code: "no_tools_visible",
            message: "tools intersection is empty (host ∩ skill ∩ node)".to_string(),
        });
    }

    let mut candidates: Vec<ToolBinding> = Vec::new();
    for id in &set {
        if let Some(tool) = registry.lookup(id) {
            let def = tool.definition();
            candidates.push(ToolBinding {
                id: id.clone(),
                name: def.name,
                description: def.description,
                input_schema: def.input_schema,
            });
        }
    }
    if candidates.is_empty() {
        return Err(NodeError::Domain {
            code: "no_tools_visible",
            message: "tools intersection produced no entries the host registry could resolve"
                .to_string(),
        });
    }
    Ok(candidates)
}

struct ToolBinding {
    id: KindId,
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

struct LoopInputs<'a> {
    runner: &'a dyn AiRunner,
    tools: &'a dyn ToolRegistry,
    visible_tools: &'a [ToolBinding],
    session_id: SessionId,
    cancel: &'a CancelAdapter<'a>,
    system_prompt: Option<&'a str>,
    initial_user_msg: String,
    prior_history: Vec<HistoryMessage>,
}

struct LoopOutcome {
    result: Result<String, NodeError>,
    turn_count: u32,
    tool_call_count: u32,
    history_snapshot: Option<Vec<HistoryMessage>>,
}

async fn run_agent_loop<'a>(inputs: LoopInputs<'a>) -> LoopOutcome {
    let LoopInputs {
        runner,
        tools,
        visible_tools,
        session_id,
        cancel,
        system_prompt,
        initial_user_msg,
        prior_history,
    } = inputs;

    let mut history: Vec<HistoryMessage> = prior_history;
    history.push(HistoryMessage {
        role: "user".to_string(),
        content: initial_user_msg.clone(),
    });

    let tool_defs: Vec<ToolDef> = visible_tools
        .iter()
        .map(|b| ToolDef {
            name: b.name.clone(),
            description: Some(b.description.clone()),
            input_schema: b.input_schema.clone(),
        })
        .collect();

    let mut turn = 0u32;
    let mut tool_call_count = 0u32;

    loop {
        if cancel.is_cancelled() {
            return LoopOutcome {
                result: Err(NodeError::Cancelled),
                turn_count: turn,
                tool_call_count,
                history_snapshot: Some(history),
            };
        }

        turn = turn.saturating_add(1);
        if turn > MAX_TURNS {
            return LoopOutcome {
                result: Err(NodeError::Domain {
                    code: "turn_budget_exhausted",
                    message: format!("ai-agent loop exceeded MAX_TURNS={MAX_TURNS}"),
                }),
                turn_count: turn - 1,
                tool_call_count,
                history_snapshot: Some(history),
            };
        }

        let (tx, mut rx) = mpsc::channel::<Event>(64);
        let drain = tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let cfg = RestCfg {
            prompt: initial_user_msg.clone(),
            system_prompt: system_prompt.map(|s| s.to_string()),
            history: history.clone(),
            tools: tool_defs.clone(),
            tool_choice: if tool_defs.is_empty() {
                Some(ToolChoice::None)
            } else {
                Some(ToolChoice::Auto)
            },
            ..RestCfg::default()
        };

        let ai_session_id: starter_spi::ai::SessionId = session_id.to_string().into();
        let no_op_cancel = NoOpAiCancel;
        let run_fut = runner.run(RunnerInput::Rest(cfg), ai_session_id, tx, &no_op_cancel);
        let run_res = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                drain.abort();
                return LoopOutcome {
                    result: Err(NodeError::Cancelled),
                    turn_count: turn,
                    tool_call_count,
                    history_snapshot: Some(history),
                };
            }
            r = run_fut => {
                drain.abort();
                r
            }
        };

        let result = match run_res {
            Ok(r) => r,
            Err(e) => {
                return LoopOutcome {
                    result: Err(NodeError::Backend(format!("ai_runner.run: {e}"))),
                    turn_count: turn,
                    tool_call_count,
                    history_snapshot: Some(history),
                };
            }
        };

        if let Some(upstream_err) = result.error.clone() {
            return LoopOutcome {
                result: Err(NodeError::Backend(format!(
                    "ai_runner upstream error: {upstream_err}"
                ))),
                turn_count: turn,
                tool_call_count,
                history_snapshot: Some(history),
            };
        }

        if !result.text.is_empty() {
            history.push(HistoryMessage {
                role: "assistant".to_string(),
                content: result.text.clone(),
            });
        }

        if result.tool_uses.is_empty() {
            return LoopOutcome {
                result: Ok(result.text),
                turn_count: turn,
                tool_call_count,
                history_snapshot: Some(history),
            };
        }

        for tu in &result.tool_uses {
            tool_call_count = tool_call_count.saturating_add(1);
            let reply = dispatch_tool_use(tools, visible_tools, tu, cancel).await;
            history.push(HistoryMessage {
                role: "user".to_string(),
                content: reply,
            });
        }
    }
}

async fn dispatch_tool_use<'a>(
    tools: &'a dyn ToolRegistry,
    visible_tools: &'a [ToolBinding],
    tu: &ToolUse,
    cancel: &'a CancelAdapter<'a>,
) -> String {
    let kind_id = visible_tools
        .iter()
        .find(|b| b.name == tu.name)
        .map(|b| b.id.clone())
        .or_else(|| KindId::new(tu.name.clone()).ok());

    let Some(kind_id) = kind_id else {
        return format!(
            "tool `{}` (id={}) refused: not a valid reverse-DNS id and no visible tool advertises it",
            tu.name, tu.id
        );
    };

    let Some(tool) = tools.lookup(&kind_id) else {
        return format!(
            "tool `{}` (id={}) refused: not registered in the host ToolRegistry",
            tu.name, tu.id
        );
    };

    if cancel.is_cancelled() {
        return format!("tool `{}` (id={}) aborted: cancelled", tu.name, tu.id);
    }

    match tool.invoke(tu.input.clone()).await {
        Ok(v) => format!("tool `{}` (id={}) returned: {}", tu.name, tu.id, v),
        Err(e) => format!("tool `{}` (id={}) errored: {}", tu.name, tu.id, e),
    }
}

// ---------------------------------------------------------------------
// CLI-shape path (D-F5.6)
// ---------------------------------------------------------------------

struct CliInputs<'a> {
    runner: &'a dyn AiRunner,
    session_id: SessionId,
    cancel: &'a CancelAdapter<'a>,
    system_prompt: Option<&'a str>,
    user_input: String,
}

/// Drive a CLI-shape runner exactly once.
///
/// Per D-F5.6, the CLI binary owns its own tool-call loop and
/// session/transcript management, so this path:
///
/// - Does not advertise the host `ToolRegistry` to the model
///   (CLI tools are dispatched by the wrapper, e.g. via MCP).
/// - Reports `turn_count = 1` and `tool_call_count = 0` from the
///   body's perspective — the wrapper's per-call log is on
///   `RunResult::tool_call_log` for callers that want it.
/// - Returns `history_snapshot = None` so the post-loop session
///   write in [`AiAgent::invoke`] is a no-op; CLI resume support
///   lives on `CliCfg::resume_id` and is wired by Phase 5 / later
///   work, not by `SessionStore`.
async fn run_cli_once<'a>(inputs: CliInputs<'a>) -> LoopOutcome {
    let CliInputs {
        runner,
        session_id,
        cancel,
        system_prompt,
        user_input,
    } = inputs;

    if cancel.is_cancelled() {
        return LoopOutcome {
            result: Err(NodeError::Cancelled),
            turn_count: 0,
            tool_call_count: 0,
            history_snapshot: None,
        };
    }

    let cfg = CliCfg {
        prompt: user_input,
        system_prompt: system_prompt.map(|s| s.to_string()),
        ..CliCfg::default()
    };

    let (tx, mut rx) = mpsc::channel::<Event>(64);
    let drain = tokio::spawn(async move { while rx.recv().await.is_some() {} });

    let ai_session_id: starter_spi::ai::SessionId = session_id.to_string().into();
    let no_op_cancel = NoOpAiCancel;
    let run_fut = runner.run(RunnerInput::Cli(cfg), ai_session_id, tx, &no_op_cancel);
    let run_res = tokio::select! {
        biased;
        _ = cancel.cancelled() => {
            drain.abort();
            return LoopOutcome {
                result: Err(NodeError::Cancelled),
                turn_count: 1,
                tool_call_count: 0,
                history_snapshot: None,
            };
        }
        r = run_fut => {
            drain.abort();
            r
        }
    };

    let result = match run_res {
        Ok(r) => r,
        Err(e) => {
            return LoopOutcome {
                result: Err(NodeError::Backend(format!("ai_runner.run: {e}"))),
                turn_count: 1,
                tool_call_count: 0,
                history_snapshot: None,
            };
        }
    };

    if let Some(upstream_err) = result.error.clone() {
        return LoopOutcome {
            result: Err(NodeError::Backend(format!(
                "ai_runner upstream error: {upstream_err}"
            ))),
            turn_count: 1,
            tool_call_count: 0,
            history_snapshot: None,
        };
    }

    LoopOutcome {
        result: Ok(result.text),
        turn_count: 1,
        tool_call_count: 0,
        history_snapshot: None,
    }
}

/// Adapter wrapping the engine's borrowed `FlowCancel` seam so
/// the loop can race it via `tokio::select!`. The `AiRunner` itself
/// receives a static [`NoOpAiCancel`] — the outer select in
/// [`run_agent_loop`] is the single source of cancellation; routing
/// the borrowed seam through `AiCancel` would require a `'static`
/// impl, which a borrow can't satisfy.
struct CancelAdapter<'a> {
    inner: &'a dyn FlowCancel,
    observed: std::sync::atomic::AtomicBool,
}

impl<'a> CancelAdapter<'a> {
    fn new(inner: &'a dyn FlowCancel) -> Self {
        Self {
            inner,
            observed: std::sync::atomic::AtomicBool::new(false),
        }
    }

    fn is_cancelled(&self) -> bool {
        let c = self.inner.is_cancelled();
        if c {
            self.observed
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        c
    }

    fn cancelled<'b>(&'b self) -> Pin<Box<dyn std::future::Future<Output = ()> + Send + 'b>> {
        Box::pin(async move {
            self.inner.cancelled().await;
            self.observed
                .store(true, std::sync::atomic::Ordering::Relaxed);
        })
    }

    fn observed(&self) -> bool {
        self.observed.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Static no-op `AiCancel` handed to `AiRunner::run`. Cancellation
/// is enforced by the outer `tokio::select!` in [`run_agent_loop`].
struct NoOpAiCancel;

impl AiCancel for NoOpAiCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
    fn cancelled<'a>(&'a self) -> Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(std::future::pending())
    }
}

// ---------------------------------------------------------------------
// Session encoding
// ---------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Default)]
struct SessionBody {
    history: Vec<HistoryMessage>,
}

fn encode_history(history: &[HistoryMessage]) -> serde_json::Value {
    serde_json::to_value(SessionBody {
        history: history.to_vec(),
    })
    .unwrap_or(serde_json::Value::Null)
}

fn decode_history(value: &serde_json::Value) -> Vec<HistoryMessage> {
    serde_json::from_value::<SessionBody>(value.clone())
        .map(|b| b.history)
        .unwrap_or_default()
}

// ---------------------------------------------------------------------
// Principal helpers (mirrors stage 5 hardcode; retired when NodeCtx
// gains a Principal field per the Phase 5 follow-up).
// ---------------------------------------------------------------------

fn system_admin_principal() -> Principal {
    Principal {
        subject: "system/Admin".to_string(),
        role: starter_spi::auth::Role::Admin,
        scopes: Vec::new(),
        extra: serde_json::Value::Null,
    }
}

fn principal_id_hash(principal: &Principal) -> String {
    let mut tag = String::with_capacity(16);
    for (i, b) in principal.subject.bytes().enumerate() {
        if i >= 8 {
            break;
        }
        use std::fmt::Write;
        let _ = write!(tag, "{b:02x}");
    }
    tag
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::time::Duration;

    use async_trait::async_trait;
    use starter_flow_spi::flow::RunId;
    use starter_flow_spi::node::NodeId;
    use starter_spi::ai::{Provider, RunResult};
    use starter_spi::tool::{Tool, ToolDefinition};
    use tokio::sync::Notify;

    #[derive(Clone, Default)]
    struct ScriptTurn {
        text: String,
        tool_uses: Vec<ToolUse>,
    }

    #[derive(Default)]
    struct RecordingAiRunnerInner {
        script: Vec<ScriptTurn>,
        cursor: usize,
        calls: Vec<RecordedCall>,
    }

    #[derive(Clone, Debug)]
    #[allow(dead_code)]
    struct RecordedCall {
        history_len: usize,
        tools_count: usize,
        session_id: starter_spi::ai::SessionId,
    }

    struct RecordingAiRunner {
        provider: Provider,
        inner: Mutex<RecordingAiRunnerInner>,
        slow_first_turn: bool,
    }

    impl RecordingAiRunner {
        fn new(script: Vec<ScriptTurn>) -> Arc<Self> {
            Arc::new(Self {
                provider: Provider::Anthropic,
                inner: Mutex::new(RecordingAiRunnerInner {
                    script,
                    cursor: 0,
                    calls: Vec::new(),
                }),
                slow_first_turn: false,
            })
        }

        fn new_slow_first(script: Vec<ScriptTurn>) -> Arc<Self> {
            Arc::new(Self {
                provider: Provider::Anthropic,
                inner: Mutex::new(RecordingAiRunnerInner {
                    script,
                    cursor: 0,
                    calls: Vec::new(),
                }),
                slow_first_turn: true,
            })
        }

        fn calls(&self) -> Vec<RecordedCall> {
            self.inner.lock().unwrap().calls.clone()
        }
    }

    #[async_trait]
    impl AiRunner for RecordingAiRunner {
        fn provider(&self) -> &Provider {
            &self.provider
        }
        async fn ready(&self) -> bool {
            true
        }
        async fn run(
            &self,
            input: RunnerInput,
            session_id: starter_spi::ai::SessionId,
            _on_event: mpsc::Sender<Event>,
            cancel: &dyn AiCancel,
        ) -> Result<RunResult, starter_spi::ai::RunnerError> {
            let history_len = match &input {
                RunnerInput::Rest(c) => c.history.len(),
                _ => 0,
            };
            let tools_count = match &input {
                RunnerInput::Rest(c) => c.tools.len(),
                _ => 0,
            };
            let turn_script = {
                let mut g = self.inner.lock().unwrap();
                g.calls.push(RecordedCall {
                    history_len,
                    tools_count,
                    session_id,
                });
                let idx = g.cursor;
                g.cursor += 1;
                g.script.get(idx).cloned().unwrap_or_else(|| ScriptTurn {
                    text: "(no more scripted turns)".to_string(),
                    ..Default::default()
                })
            };

            if self.slow_first_turn {
                tokio::select! {
                    _ = cancel.cancelled() => {}
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {}
                }
            }

            Ok(RunResult {
                text: turn_script.text,
                tool_uses: turn_script.tool_uses,
                provider: self.provider.to_string(),
                ..RunResult::default()
            })
        }
    }

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: "starter.test.echo".to_string(),
                description: "Echoes the input under the `echoed` key".to_string(),
                input_schema: serde_json::json!({"type": "object"}),
            }
        }
        async fn invoke(&self, input: serde_json::Value) -> starter_spi::Result<serde_json::Value> {
            Ok(serde_json::json!({"echoed": input}))
        }
    }

    struct TestToolRegistry {
        tools: HashMap<KindId, Arc<dyn Tool>>,
    }

    impl ToolRegistry for TestToolRegistry {
        fn lookup(&self, tool_id: &KindId) -> Option<Arc<dyn Tool>> {
            self.tools.get(tool_id).cloned()
        }
    }

    fn registry_with_echo() -> Arc<dyn ToolRegistry> {
        let mut tools: HashMap<KindId, Arc<dyn Tool>> = HashMap::new();
        tools.insert(
            KindId::new("starter.test.echo").unwrap(),
            Arc::new(EchoTool) as Arc<dyn Tool>,
        );
        Arc::new(TestToolRegistry { tools })
    }

    fn empty_tool_registry() -> Arc<dyn ToolRegistry> {
        Arc::new(TestToolRegistry {
            tools: HashMap::new(),
        })
    }

    fn runner_registry_with(
        provider_id: &str,
        runner: Arc<dyn AiRunner>,
    ) -> Arc<dyn AiRunnerRegistry> {
        let mut r = StaticAiRunnerRegistry::new();
        r.register(KindId::new(provider_id).unwrap(), runner);
        Arc::new(r)
    }

    fn input_with(slots: &[(&str, SlotValue)]) -> SlotMap {
        let mut m = SlotMap::new();
        for (k, v) in slots {
            m.insert(k.to_string(), v.clone());
        }
        m
    }

    struct NoCancel;
    impl FlowCancel for NoCancel {
        fn is_cancelled(&self) -> bool {
            false
        }
        fn cancelled<'a>(&'a self) -> Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
            Box::pin(std::future::pending())
        }
    }

    struct ManualCancel {
        notify: Arc<Notify>,
        flag: std::sync::atomic::AtomicBool,
    }
    impl ManualCancel {
        fn new() -> Self {
            Self {
                notify: Arc::new(Notify::new()),
                flag: std::sync::atomic::AtomicBool::new(false),
            }
        }
        fn fire(&self) {
            self.flag.store(true, std::sync::atomic::Ordering::Relaxed);
            self.notify.notify_waiters();
        }
    }
    impl FlowCancel for ManualCancel {
        fn is_cancelled(&self) -> bool {
            self.flag.load(std::sync::atomic::Ordering::Relaxed)
        }
        fn cancelled<'a>(&'a self) -> Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
            let n = self.notify.clone();
            Box::pin(async move { n.notified().await })
        }
    }

    fn ctx<'a>(node: &'a NodeId, cancel: &'a dyn FlowCancel) -> NodeCtx<'a> {
        NodeCtx::new(RunId::new(), node, cancel, SkillSelection::NONE)
    }

    fn node(id: &str) -> NodeId {
        NodeId::new(id).unwrap()
    }

    #[tokio::test]
    async fn missing_provider_id_surfaces_typed_domain_error() {
        let agent = AiAgent::new(
            empty_tool_registry(),
            runner_registry_with("p.test", RecordingAiRunner::new(vec![])),
        );
        let n = node("flow.test.ai");
        let cancel = NoCancel;
        let err = agent
            .invoke(ctx(&n, &cancel), input_with(&[]))
            .await
            .expect_err("expected NodeError::Domain");
        match err {
            NodeError::Domain { code, .. } => assert_eq!(code, "provider_id_required"),
            other => panic!("expected provider_id_required; got {other:?}"),
        }
    }

    #[tokio::test]
    async fn unregistered_provider_id_surfaces_typed_domain_error() {
        let agent = AiAgent::new(
            empty_tool_registry(),
            Arc::new(StaticAiRunnerRegistry::new()),
        );
        let n = node("flow.test.ai");
        let cancel = NoCancel;
        let err = agent
            .invoke(
                ctx(&n, &cancel),
                input_with(&[(PROVIDER_ID_SLOT, SlotValue::String("p.test".to_string()))]),
            )
            .await
            .expect_err("expected NodeError::Domain");
        match err {
            NodeError::Domain { code, .. } => assert_eq!(code, "provider_not_registered"),
            other => panic!("expected provider_not_registered; got {other:?}"),
        }
    }

    #[tokio::test]
    async fn empty_tools_intersection_surfaces_no_tools_visible() {
        let agent = AiAgent::new(
            empty_tool_registry(),
            runner_registry_with(
                "p.test",
                RecordingAiRunner::new(vec![ScriptTurn {
                    text: "ok".to_string(),
                    ..Default::default()
                }]),
            ),
        );
        let n = node("flow.test.ai");
        let cancel = NoCancel;
        let err = agent
            .invoke(
                ctx(&n, &cancel),
                input_with(&[
                    (PROVIDER_ID_SLOT, SlotValue::String("p.test".to_string())),
                    (
                        ALLOWED_TOOLS_SLOT,
                        SlotValue::Json(serde_json::json!(["starter.test.missing"])),
                    ),
                ]),
            )
            .await
            .expect_err("expected NodeError::Domain");
        match err {
            NodeError::Domain { code, .. } => assert_eq!(code, "no_tools_visible"),
            other => panic!("expected no_tools_visible; got {other:?}"),
        }
    }

    #[tokio::test]
    async fn one_tool_use_then_text_drives_exactly_two_turns_and_dispatches() {
        let script = vec![
            ScriptTurn {
                text: String::new(),
                tool_uses: vec![ToolUse {
                    id: "tool_use_1".to_string(),
                    name: "starter.test.echo".to_string(),
                    input: serde_json::json!({"hello": "world"}),
                }],
            },
            ScriptTurn {
                text: "final answer".to_string(),
                ..Default::default()
            },
        ];
        let runner = RecordingAiRunner::new(script);
        let agent = AiAgent::new(
            registry_with_echo(),
            runner_registry_with("p.test", runner.clone()),
        );
        let n = node("flow.test.ai");
        let cancel = NoCancel;
        let out = agent
            .invoke(
                ctx(&n, &cancel),
                input_with(&[
                    (PROVIDER_ID_SLOT, SlotValue::String("p.test".to_string())),
                    (
                        ALLOWED_TOOLS_SLOT,
                        SlotValue::Json(serde_json::json!(["starter.test.echo"])),
                    ),
                    (INPUT_SLOT, SlotValue::String("hi".to_string())),
                ]),
            )
            .await
            .expect("loop succeeds");
        assert_eq!(
            out.get(OUTPUT_SLOT),
            Some(&SlotValue::String("final answer".to_string()))
        );
        assert_eq!(out.get(TURN_COUNT_SLOT), Some(&SlotValue::Int(2)));
        assert_eq!(runner.calls().len(), 2);
        assert!(runner.calls()[0].tools_count >= 1);
        // Second-turn history grew because the tool reply was appended.
        assert!(runner.calls()[1].history_len > runner.calls()[0].history_len);
    }

    #[tokio::test]
    async fn cancel_fired_mid_turn_surfaces_cancelled_within_budget() {
        let runner = RecordingAiRunner::new_slow_first(vec![ScriptTurn {
            text: "won't get here".to_string(),
            ..Default::default()
        }]);
        let agent = AiAgent::new(
            empty_tool_registry(),
            runner_registry_with("p.test", runner),
        );
        let cancel = ManualCancel::new();
        let n = node("flow.test.ai");

        // Race: fire cancel after 50ms while the invoke awaits the
        // 5-second slow runner. Both futures live in this async scope
        // so the borrowed ctx is well-formed.
        let inputs = input_with(&[
            (PROVIDER_ID_SLOT, SlotValue::String("p.test".to_string())),
            (INPUT_SLOT, SlotValue::String("go".to_string())),
        ]);
        let start = std::time::Instant::now();
        let invoke_fut = agent.invoke(ctx(&n, &cancel), inputs);
        let cancel_fire = async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            cancel.fire();
        };
        tokio::pin!(invoke_fut);
        tokio::pin!(cancel_fire);
        let res = tokio::select! {
            r = &mut invoke_fut => r,
            _ = &mut cancel_fire => invoke_fut.await,
        };
        let elapsed = start.elapsed();
        let err = res.expect_err("expected cancelled");
        assert!(
            matches!(err, NodeError::Cancelled),
            "expected NodeError::Cancelled, got {err:?}"
        );
        assert!(
            elapsed < Duration::from_millis(500),
            "cancel-to-exit took {elapsed:?}, budget is 200ms (+ slack)"
        );
    }

    #[tokio::test]
    async fn invalid_session_mode_surfaces_typed_domain_error() {
        let agent = AiAgent::new(
            empty_tool_registry(),
            runner_registry_with(
                "p.test",
                RecordingAiRunner::new(vec![ScriptTurn {
                    text: "ok".to_string(),
                    ..Default::default()
                }]),
            ),
        );
        let n = node("flow.test.ai");
        let cancel = NoCancel;
        let err = agent
            .invoke(
                ctx(&n, &cancel),
                input_with(&[
                    (PROVIDER_ID_SLOT, SlotValue::String("p.test".to_string())),
                    (
                        SESSION_MODE_SLOT,
                        SlotValue::String("not_a_real_mode".to_string()),
                    ),
                ]),
            )
            .await
            .expect_err("expected NodeError::Domain");
        match err {
            NodeError::Domain { code, .. } => assert_eq!(code, "session_mode_invalid"),
            other => panic!("expected session_mode_invalid; got {other:?}"),
        }
    }

    #[tokio::test]
    async fn terminal_text_only_turn_yields_one_turn_and_writes_output_slot() {
        let runner = RecordingAiRunner::new(vec![ScriptTurn {
            text: "the answer".to_string(),
            ..Default::default()
        }]);
        let agent = AiAgent::new(
            empty_tool_registry(),
            runner_registry_with("p.test", runner.clone()),
        );
        let n = node("flow.test.ai");
        let cancel = NoCancel;
        let out = agent
            .invoke(
                ctx(&n, &cancel),
                input_with(&[
                    (PROVIDER_ID_SLOT, SlotValue::String("p.test".to_string())),
                    (INPUT_SLOT, SlotValue::String("q".to_string())),
                ]),
            )
            .await
            .expect("loop succeeds");
        assert_eq!(
            out.get(OUTPUT_SLOT),
            Some(&SlotValue::String("the answer".to_string()))
        );
        assert_eq!(out.get(TURN_COUNT_SLOT), Some(&SlotValue::Int(1)));
        assert_eq!(runner.calls().len(), 1);
    }

    /// CLI-only runner: rejects `RunnerInput::Rest` and records the
    /// `CliCfg.prompt` / `CliCfg.system_prompt` it observed. Models
    /// the `ClaudeRunner` shape from `starter-ai/src/runners/claude.rs`
    /// without needing the `claude` binary on PATH.
    struct CliOnlyRecordingRunner {
        provider: Provider,
        observed: Mutex<Vec<(String, Option<String>)>>,
        reply: String,
    }

    impl CliOnlyRecordingRunner {
        fn new(reply: &str) -> Arc<Self> {
            Arc::new(Self {
                provider: Provider::Claude,
                observed: Mutex::new(Vec::new()),
                reply: reply.to_string(),
            })
        }
    }

    #[async_trait]
    impl AiRunner for CliOnlyRecordingRunner {
        fn provider(&self) -> &Provider {
            &self.provider
        }
        async fn ready(&self) -> bool {
            true
        }
        async fn run(
            &self,
            input: RunnerInput,
            _session_id: starter_spi::ai::SessionId,
            _on_event: mpsc::Sender<Event>,
            _cancel: &dyn AiCancel,
        ) -> Result<RunResult, starter_spi::ai::RunnerError> {
            let cfg = match input {
                RunnerInput::Cli(c) => c,
                other => {
                    return Err(starter_spi::ai::RunnerError::WrongInputKind {
                        provider: self.provider.to_string(),
                        expected: "cli",
                        got: other.kind_tag(),
                    });
                }
            };
            self.observed
                .lock()
                .unwrap()
                .push((cfg.prompt.clone(), cfg.system_prompt.clone()));
            Ok(RunResult {
                text: self.reply.clone(),
                provider: self.provider.to_string(),
                ..RunResult::default()
            })
        }
    }

    #[tokio::test]
    async fn input_kind_cli_drives_cli_runner_once_and_returns_text() {
        let runner = CliOnlyRecordingRunner::new("claude says hi");
        let agent = AiAgent::new(
            empty_tool_registry(),
            runner_registry_with("p.claude.cli", runner.clone()),
        );
        let n = node("flow.test.ai");
        let cancel = NoCancel;
        let out = agent
            .invoke(
                ctx(&n, &cancel),
                input_with(&[
                    (
                        PROVIDER_ID_SLOT,
                        SlotValue::String("p.claude.cli".to_string()),
                    ),
                    (INPUT_KIND_SLOT, SlotValue::String("cli".to_string())),
                    (
                        SYSTEM_PROMPT_SLOT,
                        SlotValue::String("you are terse".to_string()),
                    ),
                    (INPUT_SLOT, SlotValue::String("hello".to_string())),
                ]),
            )
            .await
            .expect("cli path succeeds");
        assert_eq!(
            out.get(OUTPUT_SLOT),
            Some(&SlotValue::String("claude says hi".to_string()))
        );
        assert_eq!(out.get(TURN_COUNT_SLOT), Some(&SlotValue::Int(1)));
        let observed = runner.observed.lock().unwrap().clone();
        assert_eq!(observed.len(), 1, "CLI runner driven exactly once");
        assert_eq!(observed[0].0, "hello");
        assert_eq!(observed[0].1.as_deref(), Some("you are terse"));
    }

    #[tokio::test]
    async fn with_input_kind_cli_default_drives_cli_path_without_slot() {
        let runner = CliOnlyRecordingRunner::new("ok");
        let agent = AiAgent::new(
            empty_tool_registry(),
            runner_registry_with("p.claude.cli", runner.clone()),
        )
        .with_input_kind(AgentInputKind::Cli);
        let n = node("flow.test.ai");
        let cancel = NoCancel;
        let out = agent
            .invoke(
                ctx(&n, &cancel),
                input_with(&[
                    (
                        PROVIDER_ID_SLOT,
                        SlotValue::String("p.claude.cli".to_string()),
                    ),
                    (INPUT_SLOT, SlotValue::String("hi".to_string())),
                ]),
            )
            .await
            .expect("cli path succeeds without explicit slot");
        assert_eq!(
            out.get(OUTPUT_SLOT),
            Some(&SlotValue::String("ok".to_string()))
        );
        assert_eq!(runner.observed.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn invalid_input_kind_surfaces_typed_domain_error() {
        let agent = AiAgent::new(
            empty_tool_registry(),
            runner_registry_with("p.test", RecordingAiRunner::new(vec![])),
        );
        let n = node("flow.test.ai");
        let cancel = NoCancel;
        let err = agent
            .invoke(
                ctx(&n, &cancel),
                input_with(&[
                    (PROVIDER_ID_SLOT, SlotValue::String("p.test".to_string())),
                    (INPUT_KIND_SLOT, SlotValue::String("graphql".to_string())),
                ]),
            )
            .await
            .expect_err("expected NodeError::Domain");
        match err {
            NodeError::Domain { code, .. } => assert_eq!(code, "input_kind_invalid"),
            other => panic!("expected input_kind_invalid; got {other:?}"),
        }
    }
}
