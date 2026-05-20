# `starter-flow` — Node settings & JSON Schema

Companion to [SCOPE.md](SCOPE.md) and [hot-reload.md](hot-reload.md).
Resolves the gap surfaced while writing [hot-reload.md](hot-reload.md)
HR1 step 1: today a node kind has **no declared settings surface**, so
`DefinitionManager::publish` cannot validate a draft's per-node
configuration at publish time. All errors defer to `invoke()`.

## One-line summary

**Each `NodeBehavior` exposes a typed `Settings` struct
(`#[derive(Deserialize, JsonSchema)]`), the trait gains
`config_schema() -> &'static RootSchema` and
`validate_settings(&serde_json::Value) -> Result<(), SettingsError>`,
and `DefinitionManager::publish` validates every node's settings
against its kind's schema before a `FlowRevision` is written.** Runtime
`invoke()` keeps reading from slots — schema is the *publish-time gate*,
not a second runtime mechanism.

This mirrors the SCOPE posture: one chokepoint (`publish`), one
validation surface (schema-from-struct), one source of truth (the kind's
Rust type).

## Why this exists

Today (Phase 2–5):

- Node kinds declare config as untyped `pub const SLOT_NAME: &str`
  identifiers
  ([`ai_agent.rs:80-105`](../../../crates/starter-flow-nodes/src/ai_agent.rs#L80-L105),
  [`log.rs:56-62`](../../../crates/starter-flow-nodes/src/log.rs#L56-L62),
  [`http_out.rs:58-78`](../../../crates/starter-flow-nodes/src/http_out.rs#L58-L78)).
- There is no machine-readable description of *what slots a kind
  accepts*, *what types they carry*, *which are required*, *what
  defaults apply*, or *what validation rules govern them*.
- Validation lives inside each kind's `invoke()`
  (e.g. [`http_out.rs:106-130`](../../../crates/starter-flow-nodes/src/http_out.rs#L106-L130),
  [`log.rs:131-141`](../../../crates/starter-flow-nodes/src/log.rs#L131-L141))
  and surfaces as `NodeError::Domain` with ad-hoc string codes.
- A UI canvas, REST handler, or file-watcher publishing a draft cannot
  catch `cost_cap: "banana"` or a missing required slot until the run
  fires — defeating the "edit and observe" loop
  [hot-reload.md](hot-reload.md) targets.

The cost: every settings typo is a *runtime* failure, every editor
surface has to re-implement validation from scratch (or do without),
and the schema for a kind is "read the source of `invoke`".

## Hard rules (load-bearing)

These rules govern the settings subsystem. Each follows the SCOPE
posture: one declared surface, one validation chokepoint, one source
of truth.

### S1 — Settings are a `#[derive(Deserialize, JsonSchema)]` struct on the kind

Every `NodeBehavior` declares an associated `Settings` type:

```rust
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AiAgentSettings {
    /// Provider id, e.g. "anthropic/claude-sonnet-4-6".
    pub provider_id: String,

    /// System prompt prepended to the conversation.
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Tools the agent is allowed to call.
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Max cost in USD per invocation. Default 0.10.
    #[serde(default = "default_cost_cap")]
    pub cost_cap: f64,
}
```

**Reason: the Rust struct is the single source of truth.** The schema
is *derived* from it via `schemars::schema_for!(AiAgentSettings)`; the
deserialiser is *derived* from it via `serde`. There is no second
artifact to keep in sync. If `cost_cap` gains a field, the schema
changes the next compile.

`#[serde(deny_unknown_fields)]` is mandatory — an unknown field in a
draft is a publish-time error, not silently dropped.

### S2 — `NodeBehavior` gains two methods, both with sensible defaults

```rust
pub trait NodeBehavior: Send + Sync + 'static {
    // ...existing methods...

    /// JSON Schema describing this kind's settings.
    /// Default: empty object (kind has no settings).
    fn config_schema(&self) -> &'static schemars::schema::RootSchema {
        &EMPTY_SCHEMA
    }

    /// Validate a settings JSON blob against this kind's schema.
    /// Default: derived from `config_schema()` via `jsonschema` crate.
    fn validate_settings(
        &self,
        body: &serde_json::Value,
    ) -> Result<(), SettingsError> {
        default_validate(self.config_schema(), body)
    }
}
```

**Reason: existing kinds compile unchanged.** A kind with no settings
returns the empty schema; a kind that wants settings derives the struct,
returns `&*KIND_SCHEMA` (a `LazyLock<RootSchema>`), and gets validation
for free.

`validate_settings` is overridable for kinds whose validation rules
exceed JSON Schema's expressiveness (e.g. cross-field constraints like
*"if `auth_kind = bearer` then `auth_token` is required"*). The default
implementation covers ~95% of cases.

### S3 — Settings live on the flow definition, not as runtime slot writes

A flow's JSON/YAML body carries each node's settings as a typed object:

```yaml
flow_id: examples.notes.codeless-demo
apply_policy: drain
nodes:
  - id: agent
    kind: starter.flow.ai-agent
    settings:
      provider_id: anthropic/claude-sonnet-4-6
      cost_cap: 0.25
      allowed_tools: [http-out, log]
  - id: logger
    kind: starter.flow.log
    settings:
      level: info
links:
  - { from: agent.output, to: logger.value }
```

The `settings` object is what `validate_settings` checks at publish
time. `DefinitionManager::publish` walks every node, looks up its
`NodeBehavior` via `NodeKindRegistry`, and calls
`validate_settings(&node.settings)`. Any failure aborts the publish
([hot-reload.md HR6](hot-reload.md#hr6-bad-drafts-never-go-live-last_good-is-the-fallback)).

### S4 — Settings flow into slots at topology-resolve time

`TopologyResolver::resolve` ([hot-reload.md HR5](hot-reload.md#hr5-boot-is-resume-to-last-known-good))
deserialises each node's `settings` object into the kind's `Settings`
struct (via `serde_json::from_value`) and writes the resulting fields
into the node's config slots before the topology is returned:

```rust
let settings: AiAgentSettings =
    serde_json::from_value(node.settings.clone())?;
graph_store.write_slot(
    SlotRef::new(node.id, PROVIDER_ID_SLOT),
    SlotValue::String(settings.provider_id),
    WriteSlotOpts::config(),
)?;
// ...one write per field...
```

**Reason: runtime is unchanged.** `invoke()` still reads slots; the
existing reactive Rubix shape (downstream nodes subscribe to config
slot changes via `SlotChanged`) keeps working verbatim. Settings
become a *publish-time projection onto slots*, not a parallel runtime
mechanism.

The field-to-slot mapping is declared once per kind via a small
`#[settings_slot = "..."]` attribute or a manual `into_slots()` impl —
TBD; see [Open questions Q-S1](#open-questions).

### S5 — Linked (reactive) settings: the `$link` escape hatch

A config field can be *driven by another node's output* instead of
holding a literal value. JSON Schema validation by default rejects
this — a string field doesn't accept `{ "$link": "agent.cost_estimate" }`.

The schema for every leaf is auto-wrapped at registration time:

```jsonc
// Generated wrapper applied to every leaf field
{
  "oneOf": [
    { /* original schema, e.g. {"type": "number"} */ },
    { "type": "object",
      "properties": { "$link": { "type": "string" } },
      "required": ["$link"],
      "additionalProperties": false }
  ]
}
```

`TopologyResolver::resolve` (S4) detects `$link` values and, instead of
writing a literal slot value, records a link in the topology's `links`
map. The node sees the resolved value at invoke time exactly as if it
had been written directly — same `SlotChanged` semantics, same
reactive propagation.

**Reason: settings and wiring are two views of the same graph.** A
form-driven UI surfaces literal-valued settings; a canvas-driven UI
surfaces links. Both produce the same `FlowRevision` body and the same
resolved topology.

### S6 — Schema versioning is per-revision, not per-kind

A kind's `Settings` struct can grow fields, deprecate fields, or add
defaults across releases. The rules:

- **Adding an optional field** (`#[serde(default)]`) is backward
  compatible. Old revisions deserialise; the new field gets its
  default.
- **Adding a required field** is a breaking change. Old revisions fail
  to deserialise. The kind must bump its `KindId` (e.g.
  `starter.flow.ai-agent.v2`) or provide a `#[serde(default)]` with a
  migration note.
- **Removing a field** is backward compatible only if `deny_unknown_fields`
  is relaxed for that field via `#[serde(alias = "...")]` or a custom
  deserialiser. Otherwise: bump `KindId`.

`DefinitionManager::publish` validates against the *currently
registered* kind's schema. A stored revision that no longer validates
under the current kind surfaces as
`FlowDefinitionEvent::ResolveFailed` at boot
([hot-reload.md HR6](hot-reload.md#hr6-bad-drafts-never-go-live-last_good-is-the-fallback))
— same fallback path as a deregistered kind. Operators re-publish with
a corrected body.

**Reason: schema drift is a real operational concern, but the existing
`ResolveFailed` mechanism already handles "a stored revision the live
engine cannot mount". No new failure mode.**

### S7 — Settings errors are structured, not stringly-typed

```rust
pub enum SettingsError {
    /// JSON shape doesn't match schema. Carries the JSON Pointer
    /// path of the offending field and the schema rule that failed.
    SchemaViolation {
        pointer: String,
        rule: &'static str,
        detail: String,
    },
    /// Deserialisation into the Settings struct failed.
    Deserialise(serde_json::Error),
    /// Cross-field rule (kind's override of validate_settings) failed.
    Domain { code: &'static str, detail: String },
}
```

`SchemaViolation` is rendered by REST / CLI / UI with the JSON Pointer
highlighting the bad field — the editor surface gets a "this field is
wrong" error pointing at the right form input without parsing free
text.

## Worked example: `ai-agent`

Current state ([`ai_agent.rs:80-105`](../../../crates/starter-flow-nodes/src/ai_agent.rs#L80-L105)):

```rust
pub const PROVIDER_ID_SLOT: &str = "provider_id";
pub const SYSTEM_PROMPT_SLOT: &str = "system_prompt";
pub const ALLOWED_TOOLS_SLOT: &str = "allowed_tools";
pub const SESSION_MODE_SLOT: &str = "session_mode";
pub const INPUT_KIND_SLOT: &str = "input_kind";
// Validation in AgentConfig::from_input — runtime only.
```

After:

```rust
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AiAgentSettings {
    /// Provider id, e.g. "anthropic/claude-sonnet-4-6".
    pub provider_id: String,

    #[serde(default)]
    pub system_prompt: Option<String>,

    #[serde(default)]
    pub allowed_tools: Vec<String>,

    #[serde(default)]
    pub session_mode: Option<SessionMode>,

    #[serde(default)]
    pub input_kind: Option<AgentInputKind>,

    #[serde(default = "default_cost_cap")]
    pub cost_cap: f64,
}

fn default_cost_cap() -> f64 { 0.10 }

static AI_AGENT_SCHEMA: LazyLock<RootSchema> =
    LazyLock::new(|| schemars::schema_for!(AiAgentSettings));

impl NodeBehavior for AiAgent {
    fn config_schema(&self) -> &'static RootSchema { &AI_AGENT_SCHEMA }
    // validate_settings: default impl is enough.
    // invoke(): unchanged — still reads PROVIDER_ID_SLOT etc.
}
```

`AgentConfig::from_input` ([`ai_agent.rs:391-410`](../../../crates/starter-flow-nodes/src/ai_agent.rs#L391-L410))
stays — it's the runtime guard for linked values whose upstream node
might produce garbage. Schema is the publish-time gate; `from_input`
is the invoke-time gate. They are not redundant: schema can't validate
something that doesn't exist yet (a future `SlotChanged`).

## Relationship to existing crates

```
starter-flow-spi  (existing)
   NodeBehavior, NodeKindRegistry, SlotValue
        ▲
        │  add: config_schema() default method
        │  add: validate_settings() default method
        │  add: SettingsError enum
        │  add: EMPTY_SCHEMA static
        │  new dep: schemars (workspace)
        │
starter-flow      (existing engine)
   add: TopologyResolver projects settings → slots (S4)
   add: $link wrapping at schema-fetch time (S5)
   modify: DefinitionManager::publish calls validate_settings
           per node before writing the FlowRevision
        ▲
        │
starter-flow-nodes  (existing)
   modify: each kind declares its Settings struct + schema static
   no behaviour change in invoke()
        ▲
        │
starter-store-sqlite  (existing)
   no schema change — settings are part of the existing
   FlowRevision body blob
```

New workspace deps:

- **`schemars`** (≥ 0.8) — derives `JsonSchema` from Rust structs.
  Zero runtime overhead; the schema is a `LazyLock<RootSchema>`.
- **`jsonschema`** (≥ 0.18) — validates JSON values against a schema.
  Used by `default_validate`. Smallish dep, no async.

Both land in `starter-flow-spi` so kinds outside the workspace
(extension crates) can use them.

## Observability

Settings validation participates in the publish span
([hot-reload.md Observability](hot-reload.md#observability)):

- `flow.definition.publish` gains fields:
  `settings_validated` (count of nodes whose settings were checked),
  `settings_violations` (count of nodes that failed; 0 on success).
- On failure: `outcome = "rejected"`,
  `error.kind = "settings_violation"`,
  `error.pointer = "/nodes/3/settings/cost_cap"`,
  `error.rule = "type"`.

Metrics:
- `flow_settings_validations_total{outcome}` —
  `outcome ∈ {ok, schema_violation, deserialise, domain}`.
- `flow_settings_kinds_registered_with_schema` — gauge of registered
  kinds whose `config_schema()` is non-empty (sanity check that new
  kinds aren't forgetting to declare).

## What does NOT land

- **No runtime schema enforcement.** Schema is the publish-time gate.
  Runtime values arrive via `SlotChanged` from upstream nodes whose
  output cannot be predicted at publish time. `invoke()` keeps its
  runtime checks.
- **No schema-driven UI generation in this crate.** The schema is
  emitted; a UI canvas (`starter-ui-flow`, SCOPE D5) consumes it. Form
  rendering is not in `starter-flow`'s scope.
- **No cross-kind schema references (`$ref` to another kind's schema).**
  Each kind's schema is self-contained. If two kinds share a config
  shape, they share a Rust type — schema dedup is the responsibility
  of `schemars`'s `definitions` table.
- **No migration framework for old revisions.** S6 punts to "re-publish
  with a corrected body". A future skill (`starter-flow-migrate`?) may
  add scripted migrations; not in this scope.
- **No partial settings updates.** A publish carries the whole flow
  body, including all node settings. Editing one field is "load body,
  edit one field, publish whole body" — the editor's concern, not the
  engine's. Idempotent short-circuit
  ([hot-reload.md HR1](hot-reload.md#hr1-one-definition-write-chokepoint))
  means no wasted work for unchanged nodes.

## Decisions made

- **D-S1 — `schemars` over hand-written JSON Schema.** Single source
  of truth (the Rust struct). Zero drift. Compile-time guarantee that
  the schema matches the deserialiser. The cost — a workspace dep —
  is paid once and amortised across every kind.
- **D-S2 — Default-impl `config_schema()` returning empty.** Existing
  kinds compile unchanged. Migration to typed settings is opt-in per
  kind. No big-bang refactor.
- **D-S3 — Settings live in the flow body, not as a separate store.**
  The `FlowRevision` body already contains the node graph; settings
  are a field on each node. No new persistence surface, no new store
  trait, no new sync question.
- **D-S4 — Publish-time validation, runtime checks retained.** Schema
  cannot validate linked values that don't exist yet; `invoke()`
  guards remain. Avoids the trap of "schema passed, so this can't
  fail at runtime" — which is false for any reactive system.
- **D-S5 — `$link` escape hatch wraps every leaf at schema-fetch
  time.** A kind's author writes the struct; the engine handles the
  wiring story. The schema a UI consumes always permits linking;
  no per-kind boilerplate.
- **D-S6 — `SettingsError::SchemaViolation` carries a JSON Pointer.**
  Editors can highlight the wrong field without parsing free text.
  Matches the surface `jsonschema` already produces.

## Open questions

- **Q-S1 — Settings → slot field mapping.** S4 needs a declaration of
  *which struct field writes which slot*. Three options:
  (a) `#[settings_slot = "provider_id"]` attribute (needs proc-macro,
  more deps);
  (b) Manual `impl IntoSlots for AiAgentSettings` (boilerplate but no
  macro);
  (c) Convention: field name = slot name (zero ceremony, requires the
  struct to be 1:1 with slot names — viable today for every existing
  kind). **Probable answer: (c) for v1, (a) if a kind ever needs to
  diverge.**
- **Q-S2 — Cross-field validation.** Some kinds need rules JSON
  Schema can't express (`if auth_kind = bearer, auth_token required`).
  S2's `validate_settings` override handles this, but the *editor* sees
  only the schema and can't catch the violation client-side. Probable
  answer: emit the cross-field rule as a `description` annotation;
  rely on server-side rejection. A future schema dialect
  (JSON Schema 2020-12 `dependentRequired`) may close this gap.
- **Q-S3 — i18n of validation messages.** `SchemaViolation::detail` is
  currently English. Should it route through `starter-i18n` like
  `NodeDescriptor::label_key`? Probable answer: yes, with the
  `jsonschema` crate's English message as the fallback when no key
  resolves.
- **Q-S4 — Schema endpoint.** Should the engine expose
  `GET /kinds/:kind/schema` (REST) and `kinds schema <kind>` (CLI) so
  external editors can fetch a kind's schema without linking to
  `starter-flow-nodes`? Probable answer: yes, lands in
  [extensions adapter](../../extensions/scope/SCOPE.md) since the kind
  registry is the natural owner.

## Smoke tests (before merging)

### "A kind with no settings still works"

`starter.flow.trigger.explicit` ships with no `Settings` struct
declared. `config_schema()` returns the empty schema.
`DefinitionManager::publish` of a flow using this kind succeeds
regardless of what (if anything) is in the node's `settings` object.

If publishing fails for a no-settings kind, the default impl has
slipped.

### "Bad type rejected at publish"

A draft has `settings: { cost_cap: "banana" }` for an `ai-agent`
node. `publish` returns
`SettingsError::SchemaViolation { pointer: "/cost_cap", rule: "type", ... }`.
`FlowStore::head` is unchanged.

If `"banana"` becomes a stored revision and only blows up at
`invoke()`, S3/S7 has slipped.

### "Required field missing rejected at publish"

A draft has `settings: {}` for an `ai-agent` node (no
`provider_id`). `publish` returns
`SettingsError::SchemaViolation { pointer: "/provider_id", rule: "required", ... }`.

If the missing field surfaces only at runtime, S1's `deny_unknown_fields`
or S2's default validator has slipped.

### "Unknown field rejected"

A draft has `settings: { cost_cap: 0.1, typo_field: 42 }`. `publish`
returns `SchemaViolation { rule: "additionalProperties", ... }`.

If the typo silently passes, the `#[serde(deny_unknown_fields)]`
contract has slipped.

### "Linked setting accepted"

A draft has `settings: { cost_cap: { "$link": "estimator.cost" } }`.
`publish` succeeds. `TopologyResolver::resolve` records a link from
`estimator.cost` into `agent.cost_cap` instead of writing a literal.

If `$link` is rejected, S5's leaf-wrapping has slipped. If `$link` is
*accepted* but written as a literal (the agent gets the string
`"{ $link: ... }"` as its cost cap), S4/S5 has slipped.

### "Schema endpoint round-trips"

`GET /kinds/starter.flow.ai-agent/schema` returns a JSON Schema
document. A UI deserialises it and generates a form. The form's
output, posted back as `settings`, validates against the same schema
locally and at the server.

If client- and server-side validation disagree, the schema is being
serialised inconsistently.

## Phasing

Each phase independently mergeable; each ships its own dep-tree gate
per [hot-reload.md Phasing](hot-reload.md#phasing) precedent.

### Phase S-1 — Trait surface + empty defaults

- `schemars` + `jsonschema` added to workspace deps.
- `NodeBehavior::config_schema()` default returns `EMPTY_SCHEMA`.
- `NodeBehavior::validate_settings()` default uses `jsonschema`.
- `SettingsError` enum in `starter-flow-spi`.
- No kind migrated yet. Existing tests pass.
- Smoke: "a kind with no settings still works".

### Phase S-2 — `DefinitionManager::publish` validates

- `publish` walks `body.nodes`, calls `validate_settings` per node.
- `FlowDefinitionEvent::Rejected { settings_violations }` on failure.
- Tracing fields added.
- Still no kind migrated; validation is a no-op (empty schemas).
- Smoke: "bad type rejected at publish" (with a test-only kind that
  declares `Settings`).

### Phase S-3 — `TopologyResolver` projects settings → slots

- Resolver deserialises `node.settings` via the kind's `Settings` and
  writes each field into its config slot (convention: field name =
  slot name, per Q-S1 (c)).
- Test-only kind exercises the projection.
- Smoke: "required field missing rejected at publish",
  "unknown field rejected".

### Phase S-4 — Migrate `starter-flow-nodes` kinds

- `ai-agent`, `log`, `http-out`, `trigger.explicit` each gain a
  `Settings` struct.
- `pub const *_SLOT` strings stay (back-compat for any external code
  reading them).
- Existing `invoke()` runtime validation stays.
- Existing examples (e.g. `examples/notes`) migrate their flow body
  to the new `settings:` shape.
- Smoke: re-run the existing `flow_demo` end-to-end.

### Phase S-5 — `$link` escape hatch

- Schema-fetch wraps every leaf with the `oneOf` link wrapper.
- `TopologyResolver` detects `$link` values and writes a link record
  instead of a literal.
- Smoke: "linked setting accepted".

### Phase S-6 — Schema endpoint

- REST: `GET /kinds/:kind/schema`.
- CLI: `starter flow kind schema <kind>`.
- Returns the kind's `config_schema()` serialised as JSON.
- Smoke: "schema endpoint round-trips".

## Bottom line

**Settings stop being undeclared string constants and become a typed
`#[derive(Deserialize, JsonSchema)]` struct on each `NodeBehavior`.**
The schema is derived, not hand-written, so it can't drift from the
deserialiser. `DefinitionManager::publish` validates every node's
settings against its kind's schema before writing a `FlowRevision`,
catching `"banana"` and typos at the editor instead of at the runtime.
`TopologyResolver::resolve` projects settings onto config slots, so
`invoke()` is unchanged and the reactive Rubix shape keeps working
verbatim. Linked (`$link`) values fall through to the existing
topology-link machinery. Kinds without settings compile unchanged
thanks to default trait impls.

This resolves the gap [hot-reload.md HR1 step 1](hot-reload.md#hr1-one-definition-write-chokepoint)
left open — *what does "validate the draft" mean for per-node
configuration?* — and gives the Codeless / Rubix editor surfaces a
single source of truth they can drive a form generator off without
re-implementing per-kind knowledge.
