# 2026-05-25 — Live-tick R3 fix + flow_ops Postgres persistence

Continuation of [2026-05-25-tick-counter-no-live-values.md](2026-05-25-tick-counter-no-live-values.md)
and [2026-05-25-flow-live-tick-demo-landed.md](2026-05-25-flow-live-tick-demo-landed.md).

## What was broken (measured before patching)

End-to-end debugging confirmed two distinct root causes, not the four
in the handover brief. Evidence captured live against the running
agent on `:5173` against PG on `:5433`:

### Root cause 1 — R3 idempotent-write killed downstream fan-out

- `starter_scheduled_flows.last_run_at` advanced every 5s (cron IS
  firing).
- SSE at `GET /api/v1/flows/com.rubix.tick-counter/events` emitted
  **exactly one** `NodeEmitted` frame per *boot*, only for the trigger
  (`{node: "com.rubix.tick", slot: "schedule", value: "*/5 * * * * *"}`).
- `node_state` row for `com.rubix.count` froze at `count=5 v5` despite
  4+ further cron fires.

`trigger.schedule` at `crates/starter-flow-nodes/src/trigger_schedule.rs`
emitted exactly one slot, `SCHEDULE_SLOT`, carrying the *constant*
cron expression. The graph-store R3 short-circuit at
`crates/starter-flow/src/graph.rs:168` (`prev_was_equal && !opts.force`)
suppressed every emit after the first → propagator never fanned out
to `count.in` → counter never ran on subsequent ticks → no SSE
downstream, no `node_state` update.

The `schedule` slot is semantically *configuration enumeration*, not
a per-tick *signal*. It was being abused as both.

### Root cause 2 — `flow_ops` verbs were not wired to Postgres

`rubix-agent/src/registry.rs:112` constructed the shared `FlowDefStore`
as `Arc::new(seed_flow_store())` — an `InMemoryFlowDefStore` seeded
from the bundled YAMLs at boot. Both `flow_ops.deploy` and
`flow_ops.list` target this single in-memory store. The PG
`flows_definitions` table (seeded separately by
`boot::flows_seed::seed_and_load` for the engine-side `FlowRegistry`)
was never touched by either verb.

Empirical proof: `flow_ops.deploy` with a unique `# MARKER:<ts>` body
returned a fresh `revision_id`; `flow_ops.list` returned the same id
and the marker; `SELECT count(*) FROM flows_definitions` stayed at
**1** and the row's `revision_id` was unchanged from the boot-time
seed.

This single hole produced three of the four reported symptoms (edge
add lost across refresh, drag position lost, deleted node resurrected
after `make restart`). The "bundled reseed shadows deletes"
hypothesis from the handover brief was **a false alarm** — the
`flows_seed.rs` "skip if live row exists" guard is correct; it never
shadowed user edits because no user edit ever reached PG.

## What was fixed in this pass

### Phase 1 — `trigger.schedule` per-tick `fire` slot (R3 fix)

`starter.flow.trigger.schedule` now emits **two** outputs:

- `FIRE_SLOT = "fire"` — `SlotValue::Int(unix_ms)` of the
  invocation timestamp. Per-tick varying, defeats R3, this is what
  downstream nodes wire to for fan-out.
- `SCHEDULE_SLOT = "schedule"` — unchanged constant cron expression
  for inspection / enumeration surfaces (`FlowAsService` flow
  introspection).

The split is the semantically-correct shape: cron expression is
config, tick is event. Both deserve their own slot.

Touched files:

- `crates/starter-flow-nodes/src/trigger_schedule.rs` — add
  `FIRE_SLOT` + emit `SlotValue::Int(now_ms)` alongside `schedule`.
  Tests assert both outputs and that successive `invoke` calls
  produce *distinct* `fire` values.
- `rubix/crates/rubix-flows/flows/tick-counter.yaml` — link source
  changed from `tick.schedule → count.in` to `tick.fire → count.in`.
- `rubix/frontend/src/lib/flow-nodes/starter-builtins.tsx` —
  `STARTER_FLOW_TRIGGER_SCHEDULE_SPEC.outputs` lists both
  `{ name: "fire", kind: "trigger" }` (primary, used by canvas
  edges) and `{ name: "schedule", kind: "data" }` (inspection).
- `rubix/crates/rubix-flows/tests/load_test.rs` — asserts
  `tick.fire → count.in` is the bundled link.

### Phase 2 — `flow_ops` reads + writes against Postgres

- **Moved** `FlowDefStore` trait + value types (`FlowRevisionRow`,
  `FlowDefChange`, `FLOW_DEFINITION_KIND`) from
  `rubix-tools::flow_ops::store` into a new
  `rubix-spi::flow_def::store` module. Mirrors the existing
  `rubix_spi::dashboard::store` separation: trait + value types live
  in `rubix-spi`, in-memory test impl + `Reversible` glue stay in
  `rubix-tools`. Back-compat re-exports remain in
  `rubix_tools::flow_ops::store` so call sites are untouched.
- **Added** `PgFlowDefStore` in `rubix-store-postgres/src/flows/`,
  modelled on `PgDashboardStore`. Targets the existing
  `flows_definitions` table — same table `flows_seed::seed_and_load`
  has been seeding for the engine-side `FlowRegistry`. The two paths
  now agree on a single source of truth. Single-active invariant
  (`superseded_at IS NULL` for exactly one row per `flow_id`) is
  enforced by the writer in a transaction.
- **Threaded** the shared PG pool through
  `crate::registry::build_tool_registry(ch_client, insights_threshold,
  pg_pool)`. When the pool is `Some`, the verb registry binds
  `PgFlowDefStore`; when `None` (laptop / no-DB path) it falls back
  to the in-memory store **without** the boot-time bundled re-seed
  (the laptop user can deploy directly; bundled flows are still
  served by the engine-side `FlowRegistry` via `flows_seed` on the
  PG path).
- The old `seed_flow_store()` helper is gone — the PG-side
  `flows_seed::seed_and_load` already handles first-boot seeding
  correctly and is now the only seeder.

### Effects

| symptom | status |
|---|---|
| No live SSE values for `count` / `emit` (root cause 1) | **fixed** by Phase 1 |
| Edge add lost across page refresh (root cause 2) | **fixed** by Phase 2 |
| Drag position lost across refresh (root cause 2) | **fixed** by Phase 2 |
| Deleted node resurrected after `make restart` (root cause 2) | **fixed** by Phase 2 |

## What this pass deliberately did NOT do

**Live structural hot-reload (the "no restart needed" contract from
the handover brief).** Canvas edits now persist to PG correctly, so:

- Page refresh shows the new body (`flow_ops.list` reads PG ✓).
- `make restart` correctly picks them up — `flows_seed`'s
  skip-if-live-row guard now actually fires on user-edited rows ✓.

But for the *currently-running engine* to swap a revision without a
restart requires:

1. `starter_flow_surfaces::FlowRegistry::replace(flow_id, new_rev)`
   semantics (today `register` rejects duplicates by
   `(flow_id, revision)`, and there's no "head per flow_id"
   indirection).
2. `starter_mcp::registry::ToolRegistry` rebind so the cached
   `FlowAsTool` swaps in the new body.
3. Wiring the `flow_notify` `on_reload` hook (currently a logging
   stub at `rubix-agent/src/main.rs:170`) to drive both of the
   above.

That work touches `starter-` upstream crates and is out of scope for
this pass. The brief's referenced `DefinitionManager::publish`
chokepoint is the right long-term home for this, but rubix-agent
does not use `DefinitionManager` today — it uses `FlowRegistry`
directly. Adopting `DefinitionManager` properly is the follow-up.

For the live-tick *value* demo this gap doesn't matter: the demo is
about live values via SSE, not live structural edits.

## Verification

```bash
# Login
curl -s -c /tmp/c.jar -H 'content-type: application/json' \
  -X POST http://127.0.0.1:5173/api/v1/auth/login \
  -d '{"email":"op@example.com","password":"rubix-dev-passwd"}' \
  | tee /tmp/login.json
CSRF=$(jq -r .csrf_token /tmp/login.json)

# Phase 1: SSE now emits frames for tick + count + emit every 5s.
timeout 22 curl -sN -b /tmp/c.jar -H "x-csrf-token: $CSRF" \
  http://127.0.0.1:5173/api/v1/flows/com.rubix.tick-counter/events

# Phase 1: node_state advances.
PGPASSWORD=rubix-dev psql -h 127.0.0.1 -p 5433 -U rubix -d rubix \
  --no-psqlrc -P pager=off -A -c \
  "SELECT node_id,key,encode(value,'escape'),version,updated_at \
     FROM node_state WHERE flow_id='com.rubix.tick-counter' \
     ORDER BY node_id,key;"

# Phase 2: deploy lands in PG.
BODY=$(PGPASSWORD=rubix-dev psql -h 127.0.0.1 -p 5433 -U rubix -d rubix \
  --no-psqlrc -P pager=off -A -t -c \
  "SELECT body_yaml FROM flows_definitions \
     WHERE flow_id='com.rubix.tick-counter' AND superseded_at IS NULL;")
MUTATED=$(printf '%s\n# MARKER:%s\n' "$BODY" "$(date +%s)")
jq -n --arg fid 'com.rubix.tick-counter' --arg b "$MUTATED" \
  '{flow_id:$fid, body_yaml:$b}' > /tmp/dep.json
curl -s -b /tmp/c.jar -H "x-csrf-token: $CSRF" \
  -H 'content-type: application/json' --data-binary @/tmp/dep.json \
  http://127.0.0.1:5173/api/v1/tools/rubix.flow_ops.deploy | jq .
PGPASSWORD=rubix-dev psql -h 127.0.0.1 -p 5433 -U rubix -d rubix \
  --no-psqlrc -P pager=off -A -c \
  "SELECT count(*) AS revs, count(*) FILTER (WHERE superseded_at IS NULL) AS live \
     FROM flows_definitions WHERE flow_id='com.rubix.tick-counter';"
```
