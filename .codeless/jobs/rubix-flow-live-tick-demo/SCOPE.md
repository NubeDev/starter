# Scope — rubix-flow-live-tick-demo

## Goal

Ship the smallest possible end-to-end proof of Niagara/Tridium-shape **always-on flow runtime**: a bundled rubix flow `trigger.schedule (every 5s)` → `counter (++)` → `log` that fires from boot, persists the counter across restarts, streams its slot values over SSE to `/flows/<id>`, and lets the operator hot-edit the cron expression and the counter's `step` without restarting rubix-agent. After this job, every layer needed for stateful, live, hot-editable flows is in place — every future stateful node (accumulator, debounce, rate-limit, last-value-buffer, holds) reuses the same seam.

Three real upstream lifts in starter:

1. **`NodeStateStore` trait** in `starter-flow-spi` — the per-node persistent state seam, keyed by `(flow_id, node_id, key)`, with in-mem + sqlite impls. Reconciles SCOPE R5 (stateless behaviours) by making state live in the store the trait exposes, not in the struct.
2. **`starter.flow.counter` node** in `starter-flow-nodes` — first consumer of the new seam. Stateful; `step` + `initial` + `reset_on_redeploy` settings.
3. **Stale-doc rewrite** on `DOCS/flow/scope/hot-reload.md` + `settings.md` — both have "Today: not implemented" sections; the code shipped (per `definition::resolver`, `active`, `classifier`, `manager`; `starter-flow-spi/src/settings.rs`; node kinds using `JsonSchema`). Rewrite to present tense.

Then the rubix-side wiring:

4. **`GET /api/v1/flows/<id>/events` SSE route** projecting `FlowEvent`s (`NodeStarted`, `NodeEmitted`, `RunCompleted`, `RunFailed`) per [`crates/starter-flow-spi/src/event_dto.rs`](../../../../crates/starter-flow-spi/src/event_dto.rs)'s `NodeSlotValue` shape.
5. **Always-on mounting** — on rubix-agent boot, every non-superseded `flows_definitions` row gets a live `FlowRunner` plus a `FlowAsService` registration for every node with a `trigger.schedule` kind. PR #32 wired this for `weekly-report`; this job generalises to "every deployed flow runs forever from boot."
6. **A bundled `com.rubix.tick-counter` flow YAML** — three nodes, fires every 5s, ships in the next `make start` cleanly.
7. **Frontend live view** at `/flows/<id>` — `<FlowCanvas overlay={runOverlay}>` subscribed via `useEventStream` (already shipped in `@nube/starter-client-react`) to the new SSE route; per-selected-node settings sidebar driven by the kind's `config_schema()` (already implemented); operator edits a setting → frontend calls `flow_ops.deploy` with the new YAML → engine atomically swaps the topology → next tick reflects the edit. No new rubix-agent flow REST verbs in this job — the existing `flow_ops.deploy` is the hot-reload entry point.

The visible success: an operator runs `make start`, opens `http://127.0.0.1:5185/flows/com.rubix.tick-counter`, sees `count: 0`, watches it climb to `count: 1` after 5s, `count: 2` after 10s, etc., live; clicks the counter node → settings panel shows `step: 1`; changes to `step: 10`, clicks Save; the next tick the count jumps `+10`; clicks the trigger → changes cron from `*/5 * * * * *` to `*/2 * * * * *`; ticks now arrive every 2s; refreshes the browser → count is preserved; kills + restarts rubix-agent → counter restores from sqlite, ticks resume from where they were.

## In scope

### Phase A+B — upstream: state seam + counter node (grouped)

R2 strictly. Trait first, impls next, consumer last; all three commits in one phase since they share `starter-flow-spi/src/state.rs` test infra. Five stages, two commits.

#### Phase A — `NodeStateStore` trait + impls + ctx wiring

- **`starter-flow-spi/src/state.rs`** (new verb file, ~150 lines) exposing:
  ```rust
  #[async_trait]
  pub trait NodeStateStore: Send + Sync + 'static {
      async fn get(&self, key: &NodeStateKey) -> Result<Option<NodeStateValue>, NodeStateError>;
      async fn put(&self, key: &NodeStateKey, value: NodeStateValue) -> Result<(), NodeStateError>;
      async fn delete(&self, key: &NodeStateKey) -> Result<(), NodeStateError>;
      async fn cas(&self, key: &NodeStateKey, expected: Option<&NodeStateValue>, next: NodeStateValue) -> Result<bool, NodeStateError>;
  }

  pub struct NodeStateKey { pub flow_id: FlowId, pub node_id: NodeId, pub key: String }
  pub struct NodeStateValue(serde_json::Value);  // JSON value; serialisation owned by callers
  pub enum NodeStateError { Backend(String), Serde(String), KeyTooLong, ValueTooLarge }
  ```
  The trait is the seam — zero I/O of its own. R5 reconciliation: a `NodeBehavior` is still `&self` (no `&mut self`); state lives in the store this trait exposes, not in the struct.
- **`starter-flow-spi/src/node.rs`** — extend `NodeCtx` with `pub state: &'a dyn NodeStateStore`. **One existing `NodeCtx` field addition** — every existing call site that constructs a `NodeCtx` must be updated. This is the load-bearing API touch in this job.
- **`starter-flow/src/state/in_memory.rs`** (new) — `InMemoryNodeStateStore` over `RwLock<HashMap<NodeStateKey, NodeStateValue>>`. Pure-Rust, no deps beyond `tokio::sync::RwLock`. Used by every unit test in `starter-flow-nodes` and by the laptop fallback in `rubix-agent` when `RUBIX_DATABASE_URL` is unset.
- **`starter-store-sqlite/src/node_state.rs`** (new verb file, ~180 lines) — `SqliteNodeStateStore` over a single-table schema:
  ```sql
  CREATE TABLE node_state (
    flow_id   TEXT NOT NULL,
    node_id   TEXT NOT NULL,
    key       TEXT NOT NULL,
    value     BLOB NOT NULL,   -- JSON, stored as bytes
    version   INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (flow_id, node_id, key)
  ) WITHOUT ROWID;
  ```
  `cas` uses `UPDATE … WHERE version = $expected_version`. Migration ships under `starter-store-sqlite/migrations/node_state/`.
- **Tests:** `starter-flow/tests/node_state_in_memory_test.rs`, `starter-store-sqlite/tests/node_state_sqlite_test.rs`. Both run the same test matrix (get-missing, get-after-put, put-overwrites, cas-success, cas-mismatch, delete-then-get-missing) parameterised over the impl.
- **`DOCS/flow/scope/node-state.md`** (new, ~200 lines) — pair to `hot-reload.md` + `settings.md`. Covers: why state lives behind a trait (R5 reconciliation), the keying scheme (flow-scoped, node-scoped, key-scoped), CAS semantics for concurrent writes, the two-impl pattern (in-mem for tests, sqlite for production), versioning, the size caps (key ≤ 256 bytes, value ≤ 64 KiB — small-blob limits enforced by both impls), the operator-visible behaviour ("a node's state survives a redeploy unless the node opts out via `reset_on_redeploy`"), and the future shape (a per-tenant scope when multi-tenant lands).

Commit: `feat(starter-flow): NodeStateStore trait + in-mem + sqlite impls + NodeCtx.state`. One commit covering all four crates because the trait, the impls, the ctx wiring, and the docs are atomic — a partial landing breaks the build at every NodeCtx call site.

#### Phase B — `starter.flow.counter` node

- **`starter-flow-nodes/src/counter.rs`** (new verb file, ~180 lines). Settings:
  ```rust
  #[derive(Deserialize, JsonSchema)]
  pub struct CounterSettings {
      #[serde(default = "default_step")]
      pub step: i64,                  // default 1
      #[serde(default)]
      pub initial: i64,                // default 0
      #[serde(default)]
      pub reset_on_redeploy: bool,    // default false (state survives)
  }
  ```
- I/O slots: `in: any` (trigger — value is ignored, the fire is what matters), `out: i64` (the new count). Plus `state.count: i64` rendered via ctx.state (not a slot).
- On `invoke`:
  1. Read `state.count` from `ctx.state.get(NodeStateKey { flow_id, node_id, key: "count" })`.
  2. If `None` → use `settings.initial`. If `Some` → use the read value.
  3. `next = current + settings.step`.
  4. `ctx.state.put(..., next)` (or `cas` with the read version if we want strict-ordering across concurrent fires — start with `put`, add CAS later if needed).
  5. Return `out: next` as a slot map.
- `reset_on_redeploy`: when set and the publish path's `EditKind::Both` (or `Settings`) fires for this node, the publish-side hook calls `ctx.state.delete` for the `count` key. The hook is the engine's, not the node's — needs a small touch in `definition::manager::publish` to call a per-kind `on_redeploy(ctx_state, edit_kind)` extension if defined. Default trait impl is a no-op; counter overrides it.
- **`KIND_ID = "starter.flow.counter"`**. Reverse-DNS, in the reserved starter namespace per R10.
- **Tests:** `starter-flow-nodes/tests/counter_invoke_test.rs` covering: initial-fire-emits-initial-plus-step, second-fire-emits-prior-plus-step, settings-change-respects-new-step, `reset_on_redeploy=true`-clears-state, `reset_on_redeploy=false`-preserves-state. Uses `InMemoryNodeStateStore` from Phase A.
- **UI side:** add `COUNTER_SPEC` to `packages/starter-ui-flow/src/nodes/builtins.tsx` mirroring the existing built-in pattern (one of `inputs[]`, one of `outputs[]`, category `data`, icon `hash`). Append to `BUILTIN_NODE_KINDS`. No custom component — `genericRenderer` is enough since the counter has no special config to surface at the node level (the live `count` value comes through the slot-value overlay, not through node chrome).

Commit: `feat(starter-flow-nodes): counter — first consumer of NodeStateStore`. One commit; the UI spec ships in the same commit since it's the kind-id contract.

#### Phase A+B closing

- **Rewrite `DOCS/flow/scope/hot-reload.md`** — the "Today (Phase 2-5)" section is stale. Code shipped: `definition::resolver::resolve()`, `definition::active.rs` atomic swap, `definition::classifier.rs` edit-kind detection, `definition::manager.rs` publish path. Rewrite to present-tense covering: every `flow_ops.deploy` is a publish; the publish path classifies the edit (Initial / NoOp / Settings / Topology / Both); settings edits short-circuit to slot writes; topology edits atomically swap `Arc<FlowTopology>` so in-flight runs finish on the prior snapshot and new runs pick the new revision.
- **Rewrite `DOCS/flow/scope/settings.md`** — same treatment. Code shipped: `starter-flow-spi/src/settings.rs`, multiple node kinds declare `Settings: Deserialize + JsonSchema`, publish validates against `config_schema()` before writing a revision. Rewrite to present-tense.
- **Add `DOCS/flow/scope/node-state.md`** per Phase A above.

Commit: `docs(flow): present-tense hot-reload + settings; new node-state.md`. One commit; doc-only.

### Phase C — rubix-side SSE event stream + always-on mounting

- **`rubix/crates/rubix-agent/src/routes/flow_events.rs`** (new verb file, ~150 lines). `GET /api/v1/flows/{flow_id}/events` returning an `axum::response::sse::Sse<Stream<Event>>`. Subscribes to the `RunHandle::events_tx` for the named flow (via a shared `FlowSubscriptionRegistry` mounted at boot), projects each `FlowEvent` through `starter_flow_spi::event_dto::NodeSlotValue` into a JSON `Event::data(...)`. Reconnect-friendly: the SSE stream emits a heartbeat every 15s. CSRF-exempt per the existing SSE pattern in extensions.
- **`rubix/crates/rubix-agent/src/boot/flow_runtime.rs`** (new verb file, ~200 lines) — the "always-on" mounter. On boot, after `flows_seed` populates the registry from `flows_definitions`, this module:
  1. Iterates every live revision in `FlowRegistry`.
  2. Constructs a `FlowRunner` instance per flow (or one shared runner driving all flows; check the starter-flow API surface and pick the idiomatic shape — likely shared).
  3. For every node with `kind = starter.flow.trigger.schedule`, reads the cron expression from the resolved topology's settings and calls `FlowAsService::register_schedule(tenant_id, flow_id, cron_expr)`. Generalises the `weekly-report` wiring from PR #32; refactor that path to consume this module rather than duplicate the logic.
  4. Wires the `NodeStateStore` impl: when `RUBIX_DATABASE_URL` is set, `SqliteNodeStateStore` opened against a sibling sqlite db at `~/.rubix/node_state.db` (separate from Postgres, kept in sqlite per Phase A's sizing); when unset, `InMemoryNodeStateStore`. The path is configurable via `agent.toml`'s `[flow_runtime] state_db_path = "..."`.
- **`rubix/crates/rubix-agent/src/main.rs`** — wire both: mount the new SSE route alongside the existing extensions SSE; call `boot::flow_runtime::mount(...)` after `flows_seed`. The same `FlowRunner` is used by the existing tool dispatch when a flow is invoked by name (no change to `flow_ops.deploy`'s handler — deploy still goes through `DefinitionManager::publish` and the engine's atomic swap kicks in).
- **`flow_ops.deploy`'s hot-reload contract** — verify that the existing handler at `rubix/crates/rubix-tools/src/flow_ops/deploy.rs` already calls `DefinitionManager::publish` (or its rubix-side equivalent). If it only writes the PG row without publishing through the engine, that's a bug — fix in this stage; commit as `fix(rubix-tools): flow_ops.deploy publishes through DefinitionManager for hot-reload`. If the publish path is already in place, mark the stage's `git` as `skipped — verification only` and document the evidence in the handover.
- **Integration test:** `rubix/crates/rubix-agent/tests/flow_live_tick_test.rs` against testcontainers PG + tempdir sqlite. Boots the agent, deploys a tick-counter flow with `*/1 * * * * *` (every second), sleeps 3s, asserts `node_state.count >= 3` AND the SSE stream emitted at least three `NodeEmitted` events. Hot-edit test: deploys a new revision with `step: 10`, asserts the next tick's count jumps by 10. Restart test: drops the `FlowRunner` and reconstructs it from the same sqlite db; asserts the new runner picks up the prior count from state.

Three commits: `feat(rubix-agent): SSE flow-events route`, `feat(rubix-agent): always-on flow_runtime + NodeStateStore wiring`, `test(rubix-agent): live tick + hot-edit + restart-persistence`. Plus the potential fourth `fix(rubix-tools): deploy publishes through DefinitionManager` if verification surfaces the gap.

### Phase D — bundled `com.rubix.tick-counter` flow

- **`rubix/crates/rubix-flows/flows/tick-counter.yaml`** (new):
  ```yaml
  id: com.rubix.tick-counter
  description: |
    Always-on demo flow. A schedule trigger fires every 5 seconds; the
    counter increments and emits the new value; the log node records it.
    Demonstrates persistent node state, hot-reload of settings, and the
    SSE live-value feed at /flows/com.rubix.tick-counter.
  trigger: schedule
  cron_expr: "*/5 * * * * *"
  nodes:
    - id: tick
      kind: starter.flow.trigger.schedule
      config:
        cron_expr: "*/5 * * * * *"
    - id: count
      kind: starter.flow.counter
      config:
        step: 1
        initial: 0
        reset_on_redeploy: false
    - id: emit
      kind: starter.flow.log
      config:
        level: info
        message_template: "tick {value}"
  links:
    - { from: "tick.fire", to: "count.in" }
    - { from: "count.out", to: "emit.value" }
  ```
- The bundled YAML is picked up by the existing `flows_seed` path (per PR #32) and lands as a non-superseded row in `flows_definitions` on first boot.
- **No MessageKeys** needed in catalogues — this flow's outputs are slot values, not user-facing strings. The `log` node's `message_template` is a tracing event, not an i18n surface.
- One sanity test in `rubix/crates/rubix-flows/tests/load_test.rs` asserting the YAML parses and converts to a `FlowBody` with three nodes and two edges.

Commit: `feat(rubix-flows): bundled com.rubix.tick-counter live demo flow`.

### Phase E — frontend live view

The visible piece. Three concrete changes to `rubix/frontend`.

- **`rubix/packages/rubix-client-react/src/hooks/flow-events.ts`** (new) — `useFlowEvents(flowId, opts?)`. Internally calls `useEventStream` from `@nube/starter-client-react` against `/api/v1/flows/${flowId}/events`. Returns `{ events: NodeSlotValue[], status, reconnect, runOverlay }` where `runOverlay` is an in-memory aggregation suitable for passing to `<FlowCanvas overlay={...}>`. Each `NodeStarted` → marks node `running`; each `NodeEmitted` → marks node `ok` + stashes the latest slot value; each `NodeFailed` → marks `error`. Sibling `.test.tsx`.
- **`rubix/frontend/src/routes/flows/$flowId.tsx`** — extend the existing route:
  1. Replace the synthetic placeholder graph from `useFlowDefinition` with a real fetch — but **this depends on `flow_ops.get` which doesn't exist yet**. For this job, two options: (a) extend `flow_ops.list` to return the YAML body in the same response (smallest backend touch — one column already in the response shape, just include it); (b) raise BLOCKED and ship the live view against the placeholder graph in the immediate term. **Default (a)**: extend `flow_ops.list`'s response to carry `body_yaml` per row. The yaml parser already runs on the client; just wire the body through. Update `useFlowDefinition` to read from the list response cache via `useQueryClient().getQueryData(FLOW_LIST_KEY)` to avoid a second round-trip. Add the `body_yaml` field to `FlowListItem` in `rubix-client-ts` and to the underlying Rust DTO in `rubix-spi`; both en + es catalogues unchanged.
  2. Subscribe to `useFlowEvents(flowId)`.
  3. Pass the overlay to `<FlowCanvas overlay={runOverlay}>`. The canvas's `RunOverlay` type already exists (per `starter-ui-flow/types.ts`); pass it through.
  4. Render the latest slot value per node on the node itself via `slotValues` (already supported by `BaseNode`).
- **Settings sidebar:** `rubix/frontend/src/routes/flows/$flowId/settings-sidebar.tsx` (~250 lines verb file split if needed). Reads the selected node from the canvas's xyflow selection state (existing); looks up the node's `kind` in the `flowRegistry`; fetches the kind's `config_schema()` via a new `rubix.flow_ops.kinds()` endpoint that returns every registered kind's `KIND_ID` + `config_schema` + `i18n_key_prefix` (one cheap REST call cached by react-query under `['rubix','flow_ops','kinds']`). The schema is JSON Schema; render via a minimal hand-rolled JSON-Schema form (or pull in `@rjsf/core` if the dep budget allows — confirm at Phase E.1; default hand-rolled for `string`/`number`/`boolean`/`enum` since those are all the bundled kinds use today). Save button calls `flowDeploy` with the updated YAML — the engine's hot-reload classifies the edit as `Settings` and short-circuits to slot writes; the running counter picks up `step: 10` on the next tick.
- **A small "Save" affordance + conflict-toast** — if the deploy fails because the revision moved (someone else edited), show a toast and refetch. Optimistic concurrency comes for free from `DefinitionManager::publish`'s prior-revision check; surface the error.
- **Playwright spec:** `rubix/frontend/e2e/flow-live-tick.spec.ts`. Log in, navigate to `/flows/com.rubix.tick-counter`, wait 6s, assert the displayed count value > 0; click the counter node, change `step` to `10`, click Save, wait 5s, assert the next count value reflects `+10`; refresh page, assert the count is preserved.

Four commits: `feat(rubix-client-react): useFlowEvents hook`, `feat(rubix-client-ts+rubix-spi): flow_ops.list includes body_yaml + flow_ops.kinds`, `feat(rubix-frontend): live values overlay + settings sidebar on /flows/$flowId`, `test(rubix-frontend): playwright live tick + hot-edit + restart`.

### Phase F — closing docs + session note + PR

- **`rubix/docs/design/flows/README.md`** — extend with the live-view + always-on + node-state pattern, link to the three upstream docs.
- **`rubix/docs/sessions/<today>-flow-live-tick-demo-landed.md`** — closing session note: per-phase commits, the operator-runnable manual flow (boot → open `/flows/com.rubix.tick-counter` → watch ticks → edit settings hot → restart → verify persistence), test counts, the three present-tense doc rewrites.
- **`rubix/docs/scope/THIN-SLICE.md`** — add a "Live flow runtime" row to the "Goals lit up beyond the thin slice" table.
- **PR** — one PR off `codeless/rubix-flow-live-tick-demo` with phase-by-phase commits.

## Out of scope

- **`flow_ops.get` / `update` / `delete` / `history` verbs.** Deferred to a separate CRUD job. This job's settings sidebar uses `flow_ops.deploy` for the hot-edit (the engine handles hot-reload through deploy already); no new write verbs.
- **Graph editing (drag nodes, connect edges) in the UI.** The canvas stays effectively read-only-with-settings-editing. Operator can edit YAML directly through a future editor; for v1, settings are the only editable surface.
- **Counter overflow handling.** `i64` is the value type; if a flow runs long enough to overflow, that's a follow-up. Document the limit in `node-state.md`.
- **Multi-tenant `NodeStateStore` scope.** Today the key is `(flow_id, node_id, key)`. Per-tenant scoping is a future extension when multi-tenant lands.
- **Cross-instance state sync.** Two rubix-agents pointing at the same sqlite file is undefined; sqlite isn't multi-writer. If multi-instance is needed, the state store grows a Postgres impl. Out of scope.
- **WebSocket transport.** SSE is enough. WS is a future job.
- **A separate "fire now" button.** The trigger is the schedule node; if a future test needs manual fire, a `trigger.explicit` node is the right answer, not a button.
- **Customising the log node's output sink.** It emits tracing events; whatever subscriber the host has wired routes them. Out of scope.
- **Settings forms beyond JSON-Schema primitives.** `array`/`object` nested schemas, refs, oneOf/anyOf — handle the primitive types only; complex schemas fall back to a `<textarea>` of raw JSON with validation feedback. Operator-visible TODO in the design doc.
- **`flow_ops.kinds` becoming i18n-aware.** The endpoint returns the kind id + schema + a single `default_label`; localisation is a follow-up.
- **Live LLM in CI.** No relevance to this job; the demo is LLM-free.
- **No `--no-verify`, no `--force`.** No phasing markers in code.

## Constraints

- **R1 — one verb per file.** Rust ≤ 400 lines hard; TS ≤ 200 lines hard. Most new files in this job target 100-200 lines.
- **R2 — upstream-first.** The state seam, counter, and doc rewrites land in starter before rubix consumes them. Phase A+B → Phase C → Phase D+E. Five REVIEW gates total (A+B, C, D, E, F).
- **R3 — doc-tier rule.** Code comments link `docs/design/<area>/README.md` only. Three new upstream docs (`hot-reload.md` rewrite, `settings.md` rewrite, `node-state.md` new) live under `DOCS/flow/scope/` — those are authoritative SCOPE-tier docs and ARE referenced from upstream code today and may continue to be (the upstream rule may differ from rubix R3; verify; if upstream code currently references them, do not break the references).
- **R4 — tool outputs are `Diagnostic` + structured data.** The new SSE route returns `NodeSlotValue` JSON, not `Diagnostic`; matches the existing event-dto pattern.
- **R5 — stateless behaviours.** Reconciled via the new `NodeStateStore` seam — state lives in the store, behaviour is `&self`-only. Documented in the new `node-state.md`.
- **R6 — tests live with the code in the same commit.**
- **R10 — reverse-DNS kind ids.** `starter.flow.counter` is the kind id; ships in the reserved starter namespace.
- **R12 — observability.** The counter node emits a `counter.invoke` tracing span recording `(node_id, run_id, prior, next)`. The SSE route emits a `flow_events.subscribe` span.
- **R13 — cancellation.** The counter's `invoke` checks `ctx.cancel.is_cancelled()` once before reading state; bounded sub-ms cancel-to-exit.
- **Commit messages.** `feat(starter-flow-spi):`, `feat(starter-flow):`, `feat(starter-flow-nodes):`, `feat(starter-store-sqlite):`, `docs(flow):` for upstream; `feat(rubix-agent):`, `feat(rubix-tools):`, `feat(rubix-spi):`, `feat(rubix-flows):`, `feat(rubix-client-ts+rubix-client-react):`, `feat(rubix-frontend):`, `test(...):` for downstream.

## Open questions

1. **`NodeStateStore` value size cap — 64 KiB or 256 KiB?** Default 64 KiB; counter's i64 is 8 bytes; bigger states (e.g. a debounce node's history) might want more. Document the cap; revisit if a real consumer needs more. Phase A confirms at design-doc-review.
2. **CAS or just put?** Counter's increment is read-modify-write — under concurrent fires from a misbehaving multi-trigger, `put` would lose increments. The trait exposes `cas`; counter uses `put` for simplicity in v1 since the engine guarantees sequential invokes per node. Document the v1 choice in `node-state.md`; mark CAS as the upgrade path. Phase B closing-handover confirms.
3. **`on_redeploy` hook signature.** A trait method on `NodeBehavior` (`async fn on_redeploy(&self, ctx: &NodeCtx, edit_kind: EditKind) -> Result<(), NodeError>` with a default no-op impl) lets counter clear state when `reset_on_redeploy=true` AND the edit is `Settings|Topology|Both`. Confirm the engine's publish path has a hook point; if not, this becomes a small upstream addition. Phase A confirms at design-doc-review.
4. **`flow_ops.kinds` endpoint shape.** Returns `[{ kind_id, config_schema, default_label }]`. Confirm the JSON Schema serialisation library — `schemars` is the existing dep; use `schemars::schema::RootSchema` directly. Phase E.1 confirms.
5. **SSE subscription registry shape.** Does `RunHandle::events_tx.subscribe()` return a tokio broadcast receiver scoped per-flow or per-run? Phase C.1 reads the starter-flow source to confirm; default assumption is per-run, so the rubix-agent SSE route fans every run's events through one channel per flow keyed by `flow_id`.
6. **Settings hot-edit conflict UX.** If the operator's deploy fails because the revision moved (someone else just edited), the toast reads "the flow was updated by someone else, latest is now revision X" and offers a "reload" button that refetches. Phase E.3 confirms the wire shape `DefinitionManager::publish` returns on stale-revision (likely a typed `ConflictError`).
7. **`/flows/com.rubix.tick-counter` route URL escaping.** TanStack Router file routes use `$flowId.tsx`; the dot in `com.rubix.tick-counter` works in URL segments. Confirm by manually navigating to the URL after deploy; if escaping is needed, the existing system-check flow id has the same shape and works, so this should be a no-op.

## References

- `DOCS/flow/scope/SCOPE.md` — authoritative starter-flow scope (R1–R13).
- `DOCS/flow/scope/hot-reload.md` — gets a present-tense rewrite in this job.
- `DOCS/flow/scope/settings.md` — gets a present-tense rewrite in this job.
- `crates/starter-flow/src/definition/{resolver,active,classifier,manager}.rs` — the hot-reload implementation (shipped); the rewrite cites these.
- `crates/starter-flow-spi/src/settings.rs` — the settings schema seam (shipped).
- `crates/starter-flow-spi/src/event_dto.rs` — the `NodeSlotValue` SSE projection.
- `crates/starter-flow-spi/src/node.rs` — `NodeCtx` gets the new `state` field.
- `crates/starter-flow-nodes/src/trigger_schedule.rs` — the cron trigger this demo uses.
- `crates/starter-flow-nodes/src/log.rs` — the terminal log node.
- `crates/starter-store-sqlite/` — home of the new `node_state.rs` impl.
- `rubix/crates/rubix-tools/src/flow_ops/deploy.rs` — the deploy verb; verify the publish-path wiring in Phase C.
- `rubix/crates/rubix-agent/src/boot/scheduler.rs` — the existing schedule wiring from PR #32; Phase C generalises.
- `rubix/crates/rubix-agent/src/boot/flows_seed.rs` — the bundled-YAML seeder; Phase D's flow lands through it.
- `rubix/packages/rubix-client-react/src/hooks/extensions.ts` and `use-extension-events.ts` — the existing SSE-hook patterns Phase E mirrors.
- `packages/starter-ui-flow/src/canvas/FlowCanvas.tsx` — the canvas already supports `overlay`, `readOnly`, and per-node `slotValues`.
- `rubix/docs/sessions/2026-05-25-handover-flow-crud-and-orientation.md` — the current handover with the codeless runbook.
- `rubix/SCOPE.md`, `rubix/HOW-TO-CODE.md`, `rubix/FILE-LAYOUT.md`, `rubix/NEW-SESSION.md`.
