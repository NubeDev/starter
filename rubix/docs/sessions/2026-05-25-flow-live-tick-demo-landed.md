# 2026-05-25 — Flow live-tick demo landed end-to-end

> **Update (post-PR #38):** the node-state wiring described
> below shipped against `SqliteNodeStateStore` +
> `~/.rubix/node_state.db`. Commit `63913c0` later migrated
> rubix node-state to Postgres (`PgNodeStateStore` over the
> shared PG pool, with a boot-time one-shot copy from the old
> SQLite file). See
> [`docs/scope/sqlite-to-postgres.md`](../scope/sqlite-to-postgres.md)
> for the plan and rationale. The SPI seam, the always-on
> mounter, the SSE route, the hot-edit classifier path, and
> the bundled `com.rubix.tick-counter` flow are all unchanged
> — only the concrete store impl moved.

Closing session note for branch
`codeless/rubix-flow-live-tick-demo`. The job framed itself as
"smallest end-to-end proof of a Niagara-shape always-on flow
runtime" and it lands as exactly that: a bundled
`trigger.schedule (cron '*/5 * * * * *')` → `starter.flow.counter
(++)` → `log` flow that fires from boot, persists its count
across restarts via the new `NodeStateStore` seam in
`starter-flow-spi`, streams slot values to `/flows/$flowId` over
SSE, and lets the operator hot-edit cron and step without
restarting `rubix-agent`.

## Niagara mental model — why this shape

Niagara users do not think in terms of "invoke a tool"; they
think in terms of *wires that are always on*. A point reads a
sensor every five seconds whether anyone is watching; a
schedule fires whether or not a user is logged in; an alarm
rule re-evaluates whenever an upstream point changes. The
canvas they see is a window into a process already running,
not a form they submit.

Before this branch rubix flows were request-shaped: a tool call
came in, the engine ran the graph once, the engine quiesced.
After this branch the engine is *running*. `trigger.schedule`
nodes are now mounted at boot via the upstream `FlowAsService`
durable cron scheduler, fire on their own cadence, write node
state to sqlite so a restart picks up where the prior process
left off, and emit `FlowEvent`s a connected browser consumes
live. Hot-edit goes through the same `flow_ops.deploy` path the
CLI uses; the engine's hot-reload classifier (Settings vs
Topology) decides whether to swap a slot value in place or
atomically swap the topology `Arc`. The operator never restarts
anything. That is the shape every later flow goal — alarms,
dashboards, ruler reactions — inherits.

## Phases — what landed where

### Phase A+B — upstream seams (`NodeStateStore` + counter node + present-tense docs)

Three commits — both R2 (upstream first) and load-bearing for
every later phase.

- **A+B.1 — `NodeStateStore` trait + two impls + ctx wiring**
  (`f68a90a` + `291d2fd`). New
  [`starter-flow-spi/src/state.rs`](../../../../crates/starter-flow-spi/src/state.rs)
  ships `NodeStateStore` with get / put / cas / delete over
  `NodeStateKey { flow_id, node_id, key }` and
  `NodeStateValue { bytes, version }`. `NodeCtx` gains
  `state: &'a dyn NodeStateStore` — this was the load-bearing
  API addition and every existing call site updated.
  [`starter-flow/src/state/in_memory.rs`](../../../../crates/starter-flow/src/state/in_memory.rs)
  ships `InMemoryNodeStateStore` over `RwLock<HashMap>` for
  tests; sqlite impl
  [`starter-store-sqlite/src/node_state.rs`](../../../../crates/starter-store-sqlite/src/node_state.rs)
  ships the production store with a `(flow_id, node_id, key)` PK
  + `version` column for CAS. Parameterised test matrix
  (get-missing / get-after-put / put-overwrites / cas-success /
  cas-mismatch / delete-then-get-missing) runs against both
  impls. New
  [`DOCS/flow/scope/node-state.md`](../../../../DOCS/flow/scope/node-state.md)
  covers R5 reconciliation + keying + CAS semantics + size caps
  (key ≤ 256 B, value ≤ 64 KiB) + `reset_on_redeploy`.
- **A+B.2 — `starter.flow.counter` node + UI spec + tests**
  (`652334e`). The reference `NodeStateStore` consumer: loads
  `count` from state, adds `step`, writes back with CAS,
  emits the new value on its `value` slot. `reset_on_redeploy`
  honoured via the `DefinitionManager::publish` topology
  classifier.
- **A+B.3 — present-tense rewrite of `hot-reload.md` +
  `settings.md`** (`6d2df96` + `85c6a6a`). Both upstream scope
  docs lose their stale "Today (Phase 2-5)" sections; the new
  text describes the engine *as it is*, citing `resolver.rs` +
  `active.rs` + `classifier.rs` + `manager.rs` line numbers and
  the `SettingsField` / `SettingsKind` enums in
  `starter-flow-spi/src/settings.rs`. Lint-doc-refs clean.

Test counts at the end of Phase A+B: `cargo test -p
starter-flow-spi -p starter-flow -p starter-flow-nodes -p
starter-store-sqlite` green; `pnpm --filter @nube/starter-ui-flow
test` green.

### Phase C — rubix-side SSE + always-on mounter + integration test

Three commits.

- **C.1 — `flow_ops.deploy` publishes through
  `DefinitionManager`** (`d10ef9a`). Verification-only —
  confirmed the rubix-side store + NOTIFY listener already lands
  the body in `DefinitionManager::publish` on the listener side
  per the goals-2-4-3 PR #32 wiring. No code change; the call
  chain is now documented in the design README.
- **C.2 — SSE route + always-on mounter** (`612c374` + `6bbab07`).
  New
  [`rubix-agent/src/routes/flow_events.rs`](../../../crates/rubix-agent/src/routes/flow_events.rs)
  exposes `GET /api/v1/flows/{flow_id}/events` returning
  `text/event-stream`, subscribed via a shared
  `FlowSubscriptionRegistry` to `RunHandle::events_tx`. Each
  `FlowEvent` is projected through
  `starter_flow_spi::event_dto::NodeSlotValue` into JSON; 15 s
  heartbeat; CSRF-exempt per the existing SSE pattern. New
  [`rubix-agent/src/boot/flow_runtime.rs`](../../../crates/rubix-agent/src/boot/flow_runtime.rs)
  is the always-on mounter: after `flows_seed` populates the
  registry it iterates every live revision, constructs
  `FlowRunner` instances, and for every node with kind
  `starter.flow.trigger.schedule` reads the cron from the
  resolved topology and calls `FlowAsService::register_schedule`
  — the weekly-report wiring from PR #32 now consumes this
  module rather than duplicating it. `SqliteNodeStateStore` is
  wired against `~/.rubix/node_state.db` when
  `RUBIX_DATABASE_URL` is set, otherwise `InMemoryNodeStateStore`.
  `AgentConfig` gains a `[flow_runtime] state_db_path` knob.
  *(Superseded by `63913c0` — `PgNodeStateStore` over the shared
  PG pool; `state_db_path` removed. See
  [`docs/scope/sqlite-to-postgres.md`](../scope/sqlite-to-postgres.md).)*
- **C.3 — live tick + hot-edit + restart-persistence integration
  test** (`bc5993b` + `3a54ca5`). New
  [`rubix-agent/tests/flow_live_tick_test.rs`](../../../crates/rubix-agent/tests/flow_live_tick_test.rs)
  runs against testcontainers PG + tempdir sqlite. Boots the
  agent, deploys a tick-counter flow with cron
  `*/1 * * * * *`, sleeps 3 s, asserts `node_state.count ≥ 3`
  AND the SSE stream emitted at least three `NodeEmitted`
  events. Hot-edit deploys a new revision with `step = 10` and
  asserts the next tick jumps by ten. Restart drops the
  `FlowRunner`, reconstructs from the same sqlite db, and
  asserts the new runner picks up the prior count.

### Phase D — bundled `com.rubix.tick-counter` flow

One commit.

- **D — bundled YAML** (`d7764b6` + `cdf9c0f`). New
  [`rubix/crates/rubix-flows/flows/tick-counter.yaml`](../../../crates/rubix-flows/flows/tick-counter.yaml)
  is the three-node graph: `trigger.schedule (cron '*/5 * * * *
  *')` → `counter (step=1, initial=0, reset_on_redeploy=false)`
  → `log (level=info, message_template='tick {value}')` with
  links `tick.fire → count.in` and `count.out → emit.value`.
  The existing `flows_seed` picks it up on next boot as a
  non-superseded `flows_definitions` row; sanity test in
  [`rubix-flows/tests/load_test.rs`](../../../crates/rubix-flows/tests/load_test.rs)
  asserts the YAML parses to a `FlowBody` with three nodes and
  two edges.

### Phase E — frontend live view + settings sidebar

Four commits.

- **E.1 — `flow_ops.list` carries `body_yaml` +
  `flow_ops.kinds`** (`7e18919` + `fd40300`). The `FlowListItem`
  Rust DTO in `rubix-spi` and the matching TS DTO in
  `rubix-client-ts` gain `body_yaml: String`; the handler in
  `rubix-tools/src/flow_ops/list.rs` includes the body in the
  same SELECT. New `flow_ops.kinds` verb returns
  `[{kind_id, config_schema, default_label}]` for every
  registered kind, sourced from `NodeKindRegistry`; one cheap
  SELECT-free endpoint. New `useFlowKinds()` hook in
  `rubix-client-react/src/hooks/flow-ops.ts` caches under
  `['rubix','flow_ops','kinds']`.
- **E.2 — `useFlowEvents` hook** (`e09d320` + `7b92caa`). New
  [`rubix/packages/rubix-client-react/src/hooks/flow-events.ts`](../../../packages/rubix-client-react/src/hooks/flow-events.ts)
  calls `useEventStream` from `@nube/starter-client-react`
  against `/api/v1/flows/${flowId}/events`; returns
  `{ events, status, reconnect, runOverlay }` where
  `runOverlay` aggregates `NodeStarted = running`,
  `NodeEmitted = ok + stash latest slot value`,
  `NodeFailed = error`, suitable for passing to
  `<FlowCanvas overlay=…>`.
- **E.3 — live values overlay + settings sidebar on
  `/flows/$flowId`** (`d100b4a` + `51c29e3`). The synthetic
  placeholder graph in `useFlowDefinition` is replaced by the
  real `body_yaml` read from `useFlowsList()` via
  `queryClient.getQueryData(FLOW_LIST_KEY)`; the page
  subscribes to `useFlowEvents(flowId)` and passes `runOverlay`
  to `<FlowCanvas overlay=…>` plus `slotValues = latest-per-node`
  to each node. New
  [`rubix/frontend/src/routes/flows/settings-sidebar.tsx`](../../../frontend/src/routes/flows/settings-sidebar.tsx)
  reads the selected node from xyflow selection, looks up its
  kind via `useFlowKinds()`, renders a minimal hand-rolled
  JSON-Schema form for string / number / boolean / enum
  primitives (complex schemas fall back to a `<textarea>` of
  raw JSON with validation feedback per SCOPE out-of-scope).
  Save calls `flowDeploy` with the updated YAML; the engine's
  hot-reload classifies the diff as Settings and short-circuits
  to slot writes; conflict-toast if deploy fails because the
  revision moved.
- **E.4 — playwright live-tick spec** (`cb934fa` + `5c2065c`).
  New
  [`rubix/frontend/e2e/flow-live-tick.spec.ts`](../../../frontend/e2e/flow-live-tick.spec.ts)
  logs in, navigates to `/flows/com.rubix.tick-counter`, waits
  6 s, asserts the displayed count > 0; clicks the counter
  node, changes step to 10, clicks Save, waits 5 s, asserts the
  next count reflects +10; refresh asserts the count is
  preserved. `pnpm --filter @nube/rubix-frontend e2e` green
  against a running backend.

### Phase F — closing docs + session note + PR

This commit. Extends
[`rubix/docs/design/flows/README.md`](../design/flows/README.md)
with the §"Always-on flow runtime + live view" section linking
to upstream `node-state.md`; adds the "Live flow runtime" row
to the [Goals lit up](../scope/THIN-SLICE.md#goals-lit-up-beyond-the-thin-slice)
table; lands this session note; opens the PR.

## Operator-runnable manual flow

```bash
# 1. Boot
make start

# 2. Open the live tick-counter canvas
open http://127.0.0.1:5185/flows/com.rubix.tick-counter

# 3. Watch the count climb every 5 s under the counter node.

# 4. Click the counter node, change `step` to 10, click Save.
# → Next tick the displayed value jumps by 10 — settings hot-edit
#   short-circuits through DefinitionManager to a slot write.

# 5. Click the trigger node, change `cron` to '*/2 * * * * *',
#    click Save.
# → Ticks accelerate to every 2 s — topology hot-edit swaps the
#   topology Arc atomically; the next tick uses the new cron.

# 6. Refresh the browser tab.
# → Count is preserved (NodeStateStore replays the row — SQLite
#   in PR #38, Postgres after 63913c0).

# 7. make restart
# → Count restored from the durable store; the always-on mounter
#   picks the flow up at boot and FlowAsService resumes firing on
#   the already-persisted schedule. The new ticks continue climbing
#   from the prior total.
```

## Test counts

- `cargo test -p starter-flow-spi -p starter-flow -p
  starter-flow-nodes -p starter-store-sqlite` — green (six new
  cases from the `NodeStateStore` matrix + counter-node
  behaviour test).
- `cargo test -p rubix-flows --test load_test` — green
  (tick-counter YAML round-trip).
- `cargo test -p rubix-agent --test flow_live_tick_test` —
  green under testcontainers PG + tempdir sqlite (live tick +
  hot-edit + restart-persistence). *(Post-`63913c0` the
  tempdir-sqlite half is gone; the test runs against
  testcontainers PG only.)*
- `pnpm --filter @nube/starter-ui-flow test` — green.
- `pnpm --filter @nube/rubix-client-react test` — green
  (`flow-events.test.tsx`).
- `pnpm --filter @nube/rubix-frontend typecheck` — green.
- `pnpm --filter @nube/rubix-frontend e2e` — green
  (`flow-live-tick.spec.ts` + all prior specs).

## Present-tense doc rewrites

Three docs lost their "Today (Phase 2-5)" past-tense scaffolding
and now describe the engine as it is:

1. [`DOCS/flow/scope/hot-reload.md`](../../../../DOCS/flow/scope/hot-reload.md)
   — every `flow_ops.deploy` is a publish through
   `DefinitionManager::publish`; the publish path classifies
   the edit via `classifier.rs` (Initial / NoOp / Settings /
   Topology / Both); settings edits short-circuit to slot
   writes via `definition::manager`; topology edits atomically
   swap `Arc<FlowTopology>` in `definition::active` so
   in-flight runs finish on the prior snapshot and new runs
   pick the new revision. Cites `resolver.rs` + `active.rs` +
   `classifier.rs` + `manager.rs` line numbers.
2. [`DOCS/flow/scope/settings.md`](../../../../DOCS/flow/scope/settings.md)
   — `starter-flow-spi/src/settings.rs` ships `SettingsField` +
   `SettingsKind`; the five trigger / log / ai-agent / http-out
   kinds derive `Settings: Deserialize + JsonSchema`; the trait
   gains `config_schema() -> &'static RootSchema`;
   `DefinitionManager::publish` validates each node's settings
   against the kind's schema before writing a revision.
3. [`DOCS/flow/scope/node-state.md`](../../../../DOCS/flow/scope/node-state.md)
   — net-new — covers R5 reconciliation + keying scheme + CAS
   semantics + the two-impl pattern + size caps +
   `reset_on_redeploy` semantics.

## PR

One PR off `codeless/rubix-flow-live-tick-demo` reviewed
phase-by-phase. Each phase is a contiguous group of commits the
reviewer can land independently; A+B and C are upstream-first
(R2) so a starter-only reviewer can land them without rubix
context.
