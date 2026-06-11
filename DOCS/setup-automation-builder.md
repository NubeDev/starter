# Setup / Automation Builder

**Status:** design proposal. Not yet implemented. **Revised 2026-06-11 after
peer review** — corrected resume semantics (§8), trusted-identity propagation
into nodes (§9) and authz (§10), tenant-safe template identity (§5), YAML
envelope parsing (§6); §12A split into [setup-identity-scoped-pages.md](setup-identity-scoped-pages.md).
**Owner:** ap@nube-io.com
**Date:** 2026-06-11
**Companions:** [setup-identity-scoped-pages.md](setup-identity-scoped-pages.md) (downstream product scenarios), [flow SCOPE](flow/), [auth SCOPE](auth/), [agent MEMORY](agent/MEMORY.md), [extensions](extensions/), [storage ADRs](storage/)

---

## 1. One-line summary

A **template-driven automation builder**: authors compose a parameterized
multi-step automation once (visually or in YAML), and any authorized user —
including a mobile app scanning a barcode — runs it with a small input form.
Long runs stream per-step progress and resume from the exact step they failed
on. The generic machinery lives in the **core server**; **custom steps and
templates plug in through extensions**.

It is almost entirely composition of primitives that already exist:
[`starter-flow`](../crates/starter-flow) (engine, checkpoints, events),
[`starter-auth-users`](../crates/starter-auth-users) +
[`starter-authz`](../crates/starter-authz) (users/teams/access), the extension
SDK (custom node kinds + UI), and the namespaced sqlx migration pattern. The
net-new code is a thin **Template/Run domain** plus its REST/MCP/UI surface.

---

## 2. Why this exists

Today, building an onboarding/setup automation means hand-writing a flow body,
wiring it to a surface, and there is no user-facing concept of "a reusable,
parameterized setup a non-author can launch and watch." We want:

- A user signs up → lands on a nav of setups/automations they're allowed to run.
- A mobile app scans a barcode on a box → launches the "add device" automation
  with `{ barcode, location }` → a new IoT sensor is provisioned through a
  100-step pipeline.
- The app gets a `run_id` **immediately** (no 100-second wait) and streams
  progress: "step 12/100 — registering sensor…".
- A step fails (flaky network) → the run is recoverable; a tap **continues from
  the failed step**, not from scratch. *(The checkpoint substrate for this
  exists; the "halt-on-node-failure + resume-from-cursor" policy is new work —
  see §8b.)*
- Authors version automations as **YAML in git**, or edit them in a **visual
  builder**; both round-trip to the same stored definition.
- Ops add **custom steps** (e.g. `device.create`) and **custom templates**
  without forking the core server — they ship an **extension**.

### Goals

- G1. One friendly **Template** concept on top of the flow graph.
- G2. **Instant launch + streamed progress** for long runs.
- G3. **Resume-from-failure** at step granularity (new policy — §8b; crash
  recovery alone exists today — §8a).
- G4. **YAML ⇄ DB** import/export.
- G5. **Users / teams / access** gating who can author vs run each template.
- G6. **Extension seam** for custom node kinds and bundled templates.
- G7. Exposed **REST (humans)** + **MCP (AI)** APIs — see [Rubix AI surface
  direction: MCP-only for AI].

### Non-goals

- Not a new workflow engine — we reuse [`starter-flow`](../crates/starter-flow)
  verbatim. No new checkpoint format, no new event bus.
- Not a new auth system — we reuse `Principal`/`Tenant`/`Team` and the RBAC
  engine. We only **register resource kinds and rules**.
- Not replacing skills/flow-as-tool surfaces; this sits alongside them.

---

## 3. Architecture: core owns the machine, extensions own the custom parts

```
┌──────────────────────────── CORE SERVER ────────────────────────────┐
│                                                                      │
│  starter-setup-spi   Template, TemplateInput, SetupRun, traits       │
│  starter-setup       TemplateStore, YAML import/export, run service  │
│                      resume service, SSE projector                   │
│        │                                                             │
│        │ uses                                                        │
│        ▼                                                             │
│  starter-flow        Engine, FlowRunner, RunStore (checkpoint),      │
│                      FlowEvent broadcast, DynamicNodeKindRegistry    │
│  starter-flow-nodes  built-in kinds: transform, http-out,           │
│                      tool-call, branch, gate, subflow, ai-agent …    │
│  starter-authz       resource registry + RBAC rules                  │
│  starter-auth-users  User / Tenant / Team / Membership / Principal   │
│  starter-store-*     setup migration source (sqlite TEXT / pg JSONB) │
│                                                                      │
│  REST + MCP routes mounted by starter-server                         │
└──────────────────────────────────────────────────────────────────────┘
                       ▲                         ▲
                       │ registers               │ contributes
          DynamicNodeKindRegistry::insert        block.yaml
                       │                         │
┌──────────────────────┴─────────────────────────┴────────────────────┐
│              EXTENSION  e.g. com.acme.devices                         │
│  • custom node kinds:  device.create, sensor.register  (side-effects)│
│  • bundled templates:  add-device.yaml  (imported into TemplateStore)│
│  • optional UI slots:  custom step config panels                     │
└──────────────────────────────────────────────────────────────────────┘
```

**Why this split (the decision in §the user's words: "custom logic in an
extension but using the core server"):**

- The **engine, checkpointing, event streaming, resume, the builder UI, the
  generic REST/MCP API, and authz** are stable, security-sensitive, and tested
  once — they belong in core.
- The **domain-specific steps** ("create a device", "parse this barcode
  format") and **packaged templates** change per customer/deployment — they
  belong behind the extension seam that already exists
  (`DynamicNodeKindRegistry` for steps, `block.yaml` `contributes` for
  templates/UI). An extension cannot fork the engine; it can only *add kinds and
  templates*, which is exactly the blast radius we want.

---

## 4. Domain model (net-new)

Lives in `crates/starter-setup-spi`. The engine speaks `FlowRevision` (a node
graph) and `RunId` (an execution); the setup layer adds the friendly wrapper.

```rust
/// A published, parameterized automation a user can launch.
pub struct Template {
    pub id: TemplateId,            // reverse-DNS, e.g. "com.acme.add-device"
    pub version: SemVer,           // immutable once published
    pub display_name: String,
    pub description: String,
    pub icon: Option<String>,
    pub category: String,          // groups templates in the nav

    /// The form the launcher fills in. Standard JSON Schema, rendered by the UI
    /// and validated server-side before a run starts.
    pub input_schema: serde_json::Value,

    /// The node graph — the "100 steps". This IS a starter-flow FlowBody.
    pub flow_body: FlowBody,

    /// How seeded inputs map onto the flow's entry slots.
    pub input_bindings: Vec<InputBinding>,   // form field -> SlotRef

    /// Which terminal slots become the run's result.
    pub output_bindings: Vec<OutputBinding>, // SlotRef -> result field

    /// Who may author vs run this. Empty teams = all teams in tenant.
    pub access: TemplateAccess,    // { tenant_id, allowed_teams, run_role }

    pub source: TemplateSource,    // Yaml { path } | Api | Extension { ext_id }
}

/// A launch of a Template. A thin index row over a flow RunId so we can list,
/// authorize, and resume runs by template/owner/tenant without touching the
/// engine's internal run tables.
pub struct SetupRun {
    pub run_id: RunId,             // the flow engine's RunId (FK)
    pub template_id: TemplateId,
    pub template_version: SemVer,
    pub owner: String,             // Principal.subject who launched it
    pub tenant_id: Option<String>,
    pub team: Option<String>,
    pub status: SetupRunStatus,    // Pending|Running|Failed|Completed|Cancelled
    pub progress: Progress,        // { done, total, current_step }
    pub created_at, pub finished_at,
}
```

### Traits (core)

```rust
pub trait TemplateStore: Send + Sync + 'static {
    async fn put(&self, t: Template) -> Result<TemplateId>;
    async fn get(&self, id: &TemplateId, v: Option<SemVer>) -> Result<Option<Template>>;
    async fn list(&self, filter: TemplateFilter) -> Result<Vec<TemplateSummary>>;
    async fn delete(&self, id: &TemplateId, v: SemVer) -> Result<()>;
}

pub trait SetupRunStore: Send + Sync + 'static {
    async fn record(&self, run: SetupRun) -> Result<()>;
    async fn get(&self, run_id: RunId) -> Result<Option<SetupRun>>;
    async fn list(&self, filter: SetupRunFilter) -> Result<Vec<SetupRun>>;
    async fn update_progress(&self, run_id: RunId, p: Progress, s: SetupRunStatus) -> Result<()>;
    async fn list_open(&self) -> Result<Vec<RunId>>;   // for resume-on-boot
}
```

`SqliteTemplateStore`/`PgTemplateStore` etc. implement these, feature-gated
`setup`, exactly like the flow stores do.

---

## 5. Storage & migrations

New namespaced migration source `setup` (sqlite TEXT / pg JSONB), composed at
boot alongside `flow`:

```rust
migrate(pool)
    .with_source(FLOW_MIGRATION_SOURCE)
    .with_source(SETUP_MIGRATION_SOURCE)   // new
    .run().await?;
```

We **reuse** the flow engine's `runs` / `run_checkpoints` tables for execution
state (checkpoints, resume). We **only add** the template catalog and a run
index.

`crates/starter-store-sqlite/migrations/setup/0001_init.sql`:

```sql
CREATE TABLE setup_templates (
    -- Tenant is part of IDENTITY (peer-review fix): two tenants installing the
    -- same extension template id@version must not collide. The reserved
    -- sentinel '__global__' namespaces extension-provided templates that all
    -- tenants inherit (and may override with a same-(id,version) row under
    -- their own tenant_id — the read path prefers tenant rows over global,
    -- mirroring the query-kind "extension source → tenant overlay" model).
    tenant_id     TEXT NOT NULL DEFAULT '__global__',
    id            TEXT NOT NULL,
    version       TEXT NOT NULL,
    display_name  TEXT NOT NULL,
    category      TEXT NOT NULL DEFAULT '',
    input_schema  TEXT NOT NULL,        -- JSON
    flow_body     TEXT NOT NULL,        -- JSON (FlowBody)
    bindings      TEXT NOT NULL,        -- JSON (input/output bindings)
    access        TEXT NOT NULL,        -- JSON (teams, run_role)
    source        TEXT NOT NULL,        -- JSON
    created_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (tenant_id, id, version)
);

CREATE TABLE setup_runs (
    run_id        TEXT PRIMARY KEY,     -- FK -> runs.run_id (flow source)
    template_id   TEXT NOT NULL,
    template_ver  TEXT NOT NULL,
    owner         TEXT NOT NULL,
    tenant_id     TEXT,
    team          TEXT,
    status        TEXT NOT NULL,
    progress_json TEXT NOT NULL,        -- { done, total, current_step }
    created_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at   TEXT
);
CREATE INDEX setup_runs_by_owner  ON setup_runs(owner, created_at);
CREATE INDEX setup_runs_by_tenant ON setup_runs(tenant_id, created_at);
CREATE INDEX setup_runs_open      ON setup_runs(status) WHERE status IN ('Pending','Running','Failed');
```

Postgres mirror uses `JSONB` for the `*_json`/schema/body columns and
`TIMESTAMPTZ` — same split the flow migrations already use.

---

## 6. YAML format (import/export)

One file is one template — an **envelope** whose `flow:` key holds a `FlowBody`.

**Correction (peer review):** we do **not** call
[`starter-flow-watch::parse_flow_file`](../crates/starter-flow-watch/src/lib.rs)
on this file. That parser expects a **top-level `flow_id`** and returns the
whole body; our file is an envelope with the flow body *nested* under `flow:`.
The import path is: `serde_yaml` → a `TemplateEnvelope` struct → take the nested
`flow` value and deserialize **only that** into `FlowBody` → validate it through
`DefinitionManager` (the same validation flow-watch uses, reused at the body
level, not the file level).

```yaml
id: com.acme.add-device
version: 1.2.0
display_name: Add a device
category: Provisioning
icon: scan

input_schema:
  type: object
  required: [barcode, location]
  properties:
    barcode:  { type: string, title: "Scan barcode" }
    location: { type: string, title: "Install location" }

# The barcode is INPUT DATA, not a step. The scan happens on the phone; by the
# time the run starts, `barcode` is just a string on the seed slots. Bind it
# straight into the first action(s) that need it.
# (Bindings are objects — `{ field, slot }` — not the `a -> b` pseudo-syntax the
# first draft used, which is not valid YAML.)
input_bindings:
  - { field: barcode,  slot: lookup-model.barcode }   # data flows in as a slot value
  - { field: barcode,  slot: create-device.barcode }  # also the idempotency key (§8c)
  - { field: location, slot: create-device.location }
output_bindings:
  - { slot: create-device.device_id, field: device_id }

access:
  allowed_teams: [hvac-ops]      # empty = any team in tenant
  run_role: writer

# Nodes are ACTIONS (verbs), not data. Custom node kinds are justified ONLY for
# domain side-effects (create-device, register-sensor). Everything else is a
# built-in kind. If the barcode needs decoding (e.g. GS1 -> {gtin, serial}),
# that's a built-in `transform` node — NOT a bespoke kind — or done at binding.
flow:                            # FlowBody — nodes + links (nested; deserialized into FlowBody)
  nodes:
    - { id: lookup-model,    kind: com.starter.http-out }    # resolve barcode -> device model (catalog)
    - { id: create-device,   kind: com.acme.device.create }  # CUSTOM side-effect, idempotent on barcode
    - { id: register-sensor, kind: com.acme.sensor.register } # CUSTOM side-effect
    - { id: notify,          kind: com.starter.tool-call }   # built-in
  links:
    - { from: lookup-model.out,    to: create-device.in }
    - { from: create-device.out,   to: register-sensor.in }
    - { from: register-sensor.out, to: notify.in }
```

> **Rule of thumb — what is a node?** A node is a *step that does something*:
> calls a service, transforms data, branches, waits. Inputs (the barcode, the
> location) are **values on slots**, supplied by the launcher and bound to the
> nodes that consume them — never nodes themselves. When in doubt: if it's a
> noun the user typed/scanned, it's input; if it's a verb the automation
> performs, it's a node. And prefer a **built-in kind** (`transform`,
> `http-out`, `tool-call`, `branch`, `gate`) — only reach for a custom kind
> when a step performs a domain-specific external side-effect.

- **Import:** `POST /setup/templates/import` (multipart or raw YAML) →
  `serde_yaml` into `TemplateEnvelope` → deserialize the nested `flow` value
  into `FlowBody` → validate it against registered node-kind schemas (reusing
  `DefinitionManager`'s body-level validation, **not** `parse_flow_file`) →
  `TemplateStore::put`.
- **Export:** `GET /setup/templates/{id}?format=yaml` → serialize back. The
  builder UI's "Save" and a `git`-committed YAML produce byte-identical stored
  definitions (canonicalize before hashing, as the flow layer already does).

---

## 7. Execution & progress (G2 — instant launch, streamed feedback)

Running a template is: validate input → seed slots → `FlowRunner::start` →
return `run_id` immediately. **Nothing blocks on completion.**

```rust
async fn run_template(store, engine, principal, id, input) -> Result<RunId> {
    let t = store.get(&id, None).await?.ok_or(NotFound)?;
    authz.check(&principal, "run", &ResourceRef::row("setup.templates", &id)
        .with_tenant(t.access.tenant_id))?;       // §10
    validate_json_schema(&input, &t.input_schema)?;     // reject bad form early

    let seed = bind_inputs(&t.input_bindings, &input); // form -> Vec<(SlotRef, SlotValue)>
    let handle = engine.runner().start(RunSpec::from(&t), seed)?;  // returns instantly
    setup_runs.record(SetupRun::new(handle.run_id(), &t, &principal,
                                    Progress { done: 0, total: t.flow_body.nodes.len(), .. }))?;
    spawn_progress_projector(handle, setup_runs.clone());  // §below
    Ok(handle.run_id())                                    // 202 Accepted
}
```

### Progress feed

The engine already emits, per step, over a `broadcast::Sender<FlowEvent>`:

| FlowEvent          | Meaning for the UI                        |
|--------------------|-------------------------------------------|
| `RunStarted`       | run begins                                |
| `NodeStarted`      | step N begins → bump `current_step`       |
| `NodeEmitted`      | step produced output → partial result     |
| `NodeFailed`       | step failed → status Failed, keep index   |
| `RunCompleted`     | done → map terminal slots to result       |
| `RunFailed`        | run-level failure                         |

Two consumers subscribe to that one broadcast:

1. **Progress projector** (server task) updates `setup_runs.progress_json` /
   `status` so list views and reconnecting clients see current state without
   replaying the stream.
2. **SSE endpoint** `GET /setup/runs/{id}/events` translates `FlowEvent` →
   Server-Sent Events for the live client. On connect it first replays the
   stored `progress` snapshot, then tails the live broadcast (the runner
   pre-subscribes its receiver before starting, so **no early events are
   lost**).

```
event: step
data: { "done": 3, "total": 12, "current_step": "create-device", "status": "running" }

event: step
data: { "done": 4, "total": 12, "current_step": "register-sensor", "status": "running" }

event: failed
data: { "done": 4, "total": 12, "current_step": "register-sensor",
        "error": "upstream timeout", "resumable": true }
```

A 100-step run thus shows continuous progress; the client never waits for the
whole thing, and a dropped connection reconnects to the snapshot + tail.

---

## 8. Failure recovery & resume (G3)

**Correction (peer review):** this is *partly* free from the engine and partly
new work. The checkpoint substrate exists; **node-failure → halt → resume-from-
that-step does not, and must be built.** Be precise about the two distinct
recovery modes:

### 8a. Crash recovery — EXISTS today

- The engine **checkpoints after every propagation tick** via
  `RunStore::checkpoint(run_id, seq, state, writes)` — every slot write up to
  the prior step is durably persisted.
- If the **process dies** mid-run, the run's row still has `finished_at IS
  NULL`, so [`RunStore::list_open()`](../crates/starter-store-postgres/src/flow/run_store.rs)
  finds it on boot; load the latest checkpoint, replay its writes with
  `replay = true` (suppresses duplicate `SlotChanged`), and propagation
  continues. This works now.

### 8b. Logical step-failure resume — NEW WORK (do not assume it exists)

The headline "a step fails → continue from that step" is **not** current engine
behaviour, and the original draft overclaimed it. As verified:

- On a node error the propagator emits `FlowEvent::NodeFailed` and **keeps
  propagating** ([propagator.rs](../crates/starter-flow/src/propagator.rs)); it
  does **not** halt the run.
- The run coordinator treats only `RunCancelled` / `RunFailed` / `RunCompleted`
  as terminal ([run.rs](../crates/starter-flow/src/run.rs)) — a `NodeFailed`
  alone is non-terminal, and after quiescence the run may still emit
  `RunCompleted`.
- `list_open()` is `WHERE finished_at IS NULL`, so a run that *finished* in a
  failed state is **not** "open" and won't be picked up for resume.

So §G3 requires an **explicit new policy**, owned jointly by the engine and the
setup layer (slot it as **P1a**, before claiming resume in any demo):

1. **Define failure as terminal-but-resumable.** A node error in a setup run
   transitions the run to a terminal `Failed` outcome (via `RunFailed` or a new
   coordinator rule that escalates a configured-fatal `NodeFailed`), with
   `finished_at` set **and** a persisted **failed-node cursor** (which node, which
   seq) and `resumable = true` in `setup_runs`.
2. **Make resume re-enter at the cursor.** `POST /setup/runs/{id}/resume`:
   `RunStore::load(id)` → replay writes with `replay = true` → re-invoke
   starting at the failed node (not from scratch). This needs an engine entry
   point to "resume a finished-failed run from cursor" — today's replay path
   only reconstructs state for an *open* run, so this is the genuinely new
   engine surface.
3. **Decide the open question** (Q1): is resume allowed after a finished-failed
   outcome auto, manual, or bounded-retry — and which `NodeError`s are fatal vs.
   retryable-in-place. Until (1)+(2) land, the honest capability is **8a crash
   recovery only**.

### 8c. Idempotency (node-author contract — required by both modes)

Replay re-applies prior writes and resume may re-enter a partially-completed
step, so any node that touches an external system (e.g. `device.create`) **must
be idempotent** — use a natural dedup key (the scanned barcode). The engine's
idempotent-write short-circuit covers *slot* writes, not *external* side
effects; that part is the author's responsibility (§9, Q5).

---

## 9. Custom logic via extensions (G6)

An extension adds domain steps and templates **without forking core**.

### Custom node kinds (the steps)

Implement `NodeBehavior` (from
[`starter-flow-spi`](../crates/starter-flow-spi/src/node.rs)) and register into
the `DynamicNodeKindRegistry` the core engine exposes:

```rust
pub struct DeviceCreateNode { /* warehouse handle via Ctx capability */ }

#[async_trait]
impl NodeBehavior for DeviceCreateNode {
    fn kind_id(&self) -> &KindId { &KIND }                  // "com.acme.device.create"
    fn trigger_slots(&self) -> &[&str] { &["in"] }
    fn config_schema(&self) -> &RootSchema { &SCHEMA }      // validated at import
    async fn invoke(&self, ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        // Identity comes from a TRUSTED, server-seeded slot — never client form
        // input. See "Trusted identity" below: the run service writes
        // `caller_user_id` / `caller_team_ids` from the verified Principal at
        // FlowRunner::start; the node reads them like any other slot.
        let owner = input.get_str("caller_user_id")?;       // seeded, not form-supplied
        let site  = input.get_first("caller_team_ids")?;
        // MUST be idempotent: dedup on input.barcode so resume is safe (§8c)
        let device = self.devices.upsert_by_barcode(input.get_str("barcode")?, owner, site).await?;
        Ok(slotmap! { "device_id" => device.id, "out" => device.summary })
    }
}
```

The extension registers it through the SDK (the capability `Ctx` gives it the
warehouse/DB handle it declared in `block.yaml`); the core
`CompositeNodeKindRegistry` looks up built-in kinds first, dynamic
(extension-contributed) second.

### Trusted identity in nodes (correction — peer review)

`NodeCtx` today carries `run / node / cancel / skill / state / flow` — **not a
`Principal`** ([node.rs](../crates/starter-flow-spi/src/node.rs)). `RunSpec::
with_principal` records identity for the *run store and skill selection*, **not**
for node invocation. So a custom node **cannot** read `principal.teams`
directly, and the run's caller/site **must not** be passed as ordinary form
input (a client could spoof it). Pick one mechanism (proposed: the first, it
mirrors the query host-token pattern and needs no engine change):

1. **Server-seed trusted identity slots (recommended).** At `FlowRunner::start`,
   the setup run service writes `caller_user_id` / `caller_team_ids` /
   `caller_tenant_id` onto reserved entry slots **from the verified
   `Principal`**, distinct from the template's `input_bindings`. Validate that a
   template's bindings can never target these reserved slots, so form input can
   never overwrite them. Nodes read them like any slot. No engine change.
2. **Add `Principal` to `NodeCtx` (alternative).** `NodeCtx` is
   `#[non_exhaustive]` and its doc-comment already lists `Principal` as a planned
   field; threading the host-bound principal through the propagator is a clean
   core change if we want nodes to read identity without a slot convention. More
   invasive; defer unless multiple node kinds need it.

Either way the rule is identical: **identity is host-bound, never
client-supplied.**

### Bundled templates

`block.yaml` `contributes` a `setup_templates` list pointing at YAML files in
the bundle; on enable, the host imports them through `TemplateStore::put` with
`source = Extension { ext_id }`. Disabling the extension removes its templates
and unregisters its node kinds.

### Author contract for resumable custom steps

Documented requirement for extension authors: any node that mutates external
state must be idempotent under replay (natural key or explicit dedup), because
resume replays prior writes and may re-enter a partially-completed step.

---

## 10. Users, teams & access (G5)

Reuse [`starter-auth-users`](../crates/starter-auth-users) and
[`starter-authz`](../crates/starter-authz) wholesale — we register resources and
rules, we do not add auth code.

### Sign-up & identity

- `POST /auth/signup` already creates a `UserRecord` and issues a session.
- The mobile app uses an **API token** (`sak_…` Bearer) scoped to a tenant/team,
  issued from the user/team it belongs to. `Principal` carries
  `{ subject, role, scopes, tenant_id, teams }` — everything access checks need.

### Resource kinds (register in the authz registry at boot)

```rust
registry.register_spec(ResourceSpec {
    kind: "setup.templates".into(),
    actions: vec!["read","create","update","delete","run"].into(),
    ownership: Ownership::Subject,   // author owns their template
    tenant_scoped: true,             // cross-tenant predicate applies
    label: "Setup templates".into(),
    description: "Parameterized automations.".into(),
});
registry.register_spec(ResourceSpec {
    kind: "setup.runs".into(),
    actions: vec!["read","create","cancel","resume"].into(),
    ownership: Ownership::Subject,   // launcher owns their run
    tenant_scoped: true,
    label: "Setup runs".into(), description: "Automation executions.".into(),
});
```

### Default rules (TOML / API — deny-overrides)

```toml
# Authors (writers) manage their own templates; admins manage all.
[[rules]]
role = "writer"  resource = "setup.templates"  actions = ["read","create","update","delete"]
condition = "owner"  effect = "Allow"

# Coarse gate: who may attempt to RUN a template at all (role/tenant only).
[[rules]]
role = "writer"  resource = "setup.templates"  actions = ["run"]  effect = "Allow"

# Launchers see and resume their own runs; admins see all.
[[rules]]
role = "*"  resource = "setup.runs"  actions = ["read","resume","cancel"]
condition = "owner"  effect = "Allow"
```

**Correction (peer review) — the per-template team check is NOT a generic authz
condition.** The condition language exposes only `object.{kind,id,owner,tenant}`
([engine.rs](../crates/starter-authz/src/engine.rs)) and `ResourceRef` carries
no arbitrary attributes ([decision.rs](../crates/starter-spi/src/authz/decision.rs)),
so `object.allowed_teams contains principal.teams` **cannot be expressed** — the
engine never sees `allowed_teams`. The original draft's rule was invalid. Use a
**two-layer check**:

1. **Generic authz** handles the coarse `setup.templates/run` gate, ownership
   (`condition = "owner"`), and the `tenant_scoped` cross-tenant predicate
   (enforced before role evaluation — a team in tenant A can never run a
   template in tenant B). Conditions *can* match a **fixed** team
   (`principal.teams contains "hvac-ops"`), since `principal.teams` is exposed —
   but not the *object's* team list.
2. **Setup-layer team check (in the run handler, Rust):** after the generic
   `check` passes, load the template and assert
   `template.access.allowed_teams.is_empty() || !allowed_teams.is_disjoint(&principal.teams)`
   before starting the run; else `403`. This is the data-dependent part the
   engine can't see.

*Alternative if we want it declarative:* extend `ResourceRef` with an
`attributes: Map<String,Value>` bag and expose it as `object.*` in the condition
context — a real but larger core change to `starter-authz` and every call site.
Defer unless several resources need attribute-based rules (tracked as Q8).

Route-level checks use `with_permission(router, "setup.templates", "run")`;
row-level checks (own template / own run) pass
`ResourceRef::row(...).with_owner(...).with_tenant(...)` into `engine.check(...)`;
the team-membership predicate is the §2 setup-layer step above, not a condition.

### Nav

A `sidebar-nav` UI federation slot lists the categories/templates the
`Principal` is allowed to `run` (server filters the list by authz before
returning), plus an "Authoring" section for writers/admins.

---

## 11. API surface

All under `/api/v1/setup`. **REST for humans/mobile, MCP for AI** (per the
MCP-only-for-AI direction).

### REST

| Method | Path                                   | Action               | Authz (kind/action)        |
|--------|----------------------------------------|----------------------|----------------------------|
| GET    | `/setup/templates`                     | list (nav)           | setup.templates/read       |
| GET    | `/setup/templates/{id}`                | fetch (`?format=yaml`)| setup.templates/read      |
| POST   | `/setup/templates`                     | create/publish       | setup.templates/create     |
| PUT    | `/setup/templates/{id}`                | update (new version) | setup.templates/update     |
| POST   | `/setup/templates/import`              | YAML import          | setup.templates/create     |
| DELETE | `/setup/templates/{id}`                | delete               | setup.templates/delete     |
| POST   | `/setup/templates/{id}/run`            | **launch → 202 {run_id}** | setup.templates/run   |
| GET    | `/setup/runs`                          | list my runs         | setup.runs/read            |
| GET    | `/setup/runs/{id}`                      | run + progress snapshot | setup.runs/read         |
| GET    | `/setup/runs/{id}/events`              | **SSE progress**     | setup.runs/read            |
| POST   | `/setup/runs/{id}/resume`              | **continue from failure** | setup.runs/resume     |
| POST   | `/setup/runs/{id}/cancel`              | cancel               | setup.runs/cancel          |
| GET    | `/setup/node-kinds`                    | palette for builder  | setup.templates/read       |

### MCP tools (for AI agents)

- `setup.list_templates` → templates the principal may run.
- `setup.run_template { template_id, input }` → `{ run_id }`.
- `setup.run_status { run_id }` → progress snapshot (poll; MCP has no SSE).
- `setup.resume_run { run_id }`.

Both surfaces share the same domain service (`run_template`, `resume`, etc.);
the surface adapters (REST handlers, MCP tool wrappers) are thin.

---

## 12. Worked example: scan a barcode → add an IoT sensor

1. Field tech signs in on mobile; app holds an `sak_…` token (tenant `acme`,
   team `hvac-ops`, role `writer`).
2. App `GET /setup/templates` → nav shows "Add a device" (allowed: team match).
3. Tech scans box barcode; app `POST /setup/templates/com.acme.add-device/run`
   with `{ barcode: "0X1A…", location: "Roof AHU-3" }`.
4. Server validates input vs `input_schema`, checks `setup.templates/run`
   (team `hvac-ops` ∈ `allowed_teams`), seeds the `barcode`/`location` values
   onto the entry slots, `FlowRunner::start` → returns **`202 { run_id }`** in
   ~milliseconds.
5. App opens `GET /setup/runs/{run_id}/events` (SSE). Watches the action steps
   `lookup-model → create-device → register-sensor → notify` tick by tick:
   "3/4 — registering sensor…".
6. `register-sensor` hits a gateway timeout → `NodeFailed`. **Under the §8b
   policy (new work)** the setup layer escalates this to a terminal `Failed`
   outcome, persists the **failed-node cursor**, and marks the run
   `resumable`; the checkpoint holds the created `device_id`.
7. Tech taps Retry → `POST /setup/runs/{run_id}/resume` → engine replays writes
   and re-enters at the cursor (`register-sensor`); the device is **not**
   recreated (`device.create` idempotent on barcode), succeeds, `RunCompleted
   { device_id }`. *(This step depends on §8b landing — it is not free from the
   current engine.)*
8. App shows the new sensor. `setup_runs` row → `Completed`.

---

## 12A. Reusable, identity-scoped pages → moved

The two product scenarios (consumer power-meter self-service, and the BMS/EMS
onsite-electrician flow) and the identity-scoped-page gap analysis now live in a
companion doc: **[setup-identity-scoped-pages.md](setup-identity-scoped-pages.md)**.

They were split out because they are **downstream product context** layered on
the Nexus product surface (WS-13 nav/context, query-kinds, `$caller_*` host
tokens), not core automation-builder scope. The single core change they need —
the `$caller_team_ids` host token — stays in this doc's build plan as **P3a**.

---

## 13. Build plan (phased)

- **P0 — domain + storage.** `starter-setup-spi` (`Template`, `SetupRun`,
  traits); `starter-setup` (`TemplateStore`, YAML import/export); `setup`
  migration source for sqlite + pg. No surface yet; unit-tested round-trip.
- **P1 — run service + crash recovery.** Wire `run_template` /
  progress-projector onto `starter-flow`; seed **trusted identity slots**
  (`caller_user_id`/`caller_team_ids`/`caller_tenant_id`) from the verified
  `Principal` at start, and forbid template bindings from targeting them (§9).
  Prove instant-launch + **§8a crash recovery** (`list_open` → replay) with an
  integration test. Reuse existing built-in node kinds only.
- **P1a — node-failure → terminal+resumable policy (§8b, NEW engine work).**
  Escalate a configured-fatal `NodeFailed` to a terminal `Failed` outcome with a
  persisted **failed-node cursor** and `resumable` flag; add the engine entry
  point that resumes a *finished-failed* run from its cursor (today's replay
  only reconstructs *open* runs). Wire `POST /setup/runs/{id}/resume` to it.
  This is the item the "continue from the failed step" promise depends on — do
  not demo resume before it lands. Decide fatal-vs-retryable `NodeError`
  classification here (Q1).
- **P2 — REST + SSE.** Mount the routes in `starter-server`; SSE projector;
  list/snapshot endpoints.
- **P3 — authz.** Register `setup.*` resource specs + default rules; wire
  `with_permission`; team-scoped run rule; tenant isolation test.
- **P3a — `$caller_team_ids` host token ([identity-scoped-pages](setup-identity-scoped-pages.md) gap #1).** Mirror
  `$caller_user_id`/`$caller_tenant_id` in
  `query/bind/{context,vars,scan}.rs`, bound from `Principal.teams`,
  un-spoofable and rejected in caller-supplied position; RLS remains the
  backstop. Unblocks every team/site-scoped reusable page.
- **P4 — MCP tools.** `setup.*` tools sharing the P1 service.
- **P5 — extension seam.** `block.yaml` `contributes.setup_templates`; dynamic
  node-kind registration; ship a `com.acme.devices` example extension
  (`device.create`, `sensor.register`) + `add-device.yaml`.
- **P6 — builder UI.** `sidebar-nav` + `main` federation slots: nav, run
  progress panel (consumes SSE), then the visual canvas (node palette from
  `/setup/node-kinds`, link wiring, input-form designer) round-tripping YAML.

P0–P1 + P3 deliver the barcode story headlessly **with crash recovery**; the
"resume from the failed step" promise specifically requires **P1a**. P6 is the
largest genuinely new surface and can land last.

---

## 14. Open questions

- **Q1 (now also gates P1a — §8b).** Resume policy: (a) which `NodeError`s are
  *fatal* (halt → resumable) vs *retryable-in-place*; (b) is resume after a
  finished-`Failed` outcome auto, manual, or bounded-retry; (c) on boot, do we
  auto-resume idempotent crashed runs or wait for a tap? Suggest: configurable;
  default manual for runs touching external systems; treat all `NodeError` as
  fatal-halt until per-node retry policy exists.
- **Q2.** Template versioning — pin a run to the template version it launched
  with (proposed: yes, `setup_runs.template_ver`), and how to migrate in-flight
  runs across template edits (proposed: never; edits create a new version).
- **Q3.** Concurrency limits per tenant/team for fan-out (100 mobile scans at
  once) — reuse any existing rate-limit layer or add a per-tenant run quota?
- **Q4.** Builder canvas: build in-house vs adopt an existing React flow canvas
  in the UI kit — needs a frontend spike.
- **Q5.** Does `device.create`-style external idempotency need a core helper
  (a dedup-key store) or is it purely the node author's responsibility?
- **Q6 ([identity-scoped-pages](setup-identity-scoped-pages.md) gap #4 — decide before P5).** Is a **site == a team**
  (`TeamRecord`, recommended — reuses teams + the `$caller_team_ids` token +
  WS-13 nav, no new tenancy tier) or **a sub-tenant** (full RLS isolation via
  the existing tenant hierarchy, heavier)? The setup automation's meter tagging
  and the verify page both depend on this.
- **Q7 ([identity-scoped-pages](setup-identity-scoped-pages.md) gap #2).** Add a `principal` source to the `context` VariableKind
  (no-SQL access to user/team in a panel), or rely solely on the `$caller_*`
  host tokens in query-kinds? Suggest: tokens first (the security boundary),
  add the context source only if authors ask for it.
- **Q8 (§10).** Keep template team-access as a **setup-layer Rust check**
  (recommended — no engine change), or extend `ResourceRef` with an
  `attributes` bag exposed as `object.*` so it can be a declarative authz
  condition? The latter is a broader `starter-authz` change touching every call
  site; do it only if several resources need attribute-based rules.
- **Q9 (identity into nodes — §9).** Server-seeded trusted slots (recommended,
  no engine change) vs. adding `Principal` to `NodeCtx` (cleaner for many node
  kinds, more invasive)? Affects every custom node that needs caller/site.
