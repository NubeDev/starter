# 2026-05-25 — data-flow stage 01: tool-call bridge + producer e2e

## Status: ⏳ partial — synth tool fully working e2e, scheduled flow still inert

This session landed the synth tool and the host-side bridge that lets
flow YAML drive a `starter.flow.tool-call` node declaratively. The
flow runs end-to-end **when invoked manually via MCP**, but the
durable scheduler is not claiming due rows on the current boot, so
the once-per-minute cron firing isn't observably driving the flow
yet. Likely environmental, not a code regression — see "Open
problem" below.

---

## What landed (committed-quality, builds clean, tests pass)

### 1. Synth tool — `rubix.dataflow.synth.emit`

- DTOs: [rubix/crates/rubix-spi/src/dto/dataflow/synth.rs](../../../crates/rubix-spi/src/dto/dataflow/synth.rs)
  + barrel [mod.rs](../../../crates/rubix-spi/src/dto/dataflow/mod.rs).
  Wire-shape locked per [01-producer.md](./01-producer.md).
- Implementation: [rubix/crates/rubix-tools/src/dataflow/](../../../crates/rubix-tools/src/dataflow/)
  with `synth.rs` (tool + state machine), `meters.rs` (per-meter
  profile + mess eligibility), `mess.rs` (gap/spike/stuck/jitter/NaN
  injectors), `tests.rs` (8 unit tests covering each mess shape +
  determinism + composite over 1000 ticks).
- Registry: tool registered in
  [rubix-agent/src/registry.rs](../../../crates/rubix-agent/src/registry.rs)
  alongside the other primitives. Added test
  `registry_contains_dataflow_synth_emit`.
- Dep: `rand = { workspace = true }` added to rubix-tools.

**Proof:** all 8 dataflow tests pass; `cargo test -p rubix-agent
--lib registry::` 15 tests green. Real REST hit:

```
POST /api/v1/tools/rubix.dataflow.synth.emit
{ "tenant_id": "site-a", "meters": [...], "tick_epoch_ms": ..., "knobs": {"seed":42} }
→ 200 OK
{ "rows": [3 wire-shape rows with jittered hvac epoch_ms],
  "stats": { "emitted":3, "gaps":0, ... } }
```

### 2. Tool-call node wired into the flow engine

The `starter.flow.tool-call` kind was **not** registered in the
rubix agent's `NodeKindRegistry` before this session. Without that,
any flow that referenced `kind: starter.flow.tool-call` would fail
topology resolution at boot. Changes:

- [rubix-agent/Cargo.toml](../../../crates/rubix-agent/Cargo.toml):
  added the `tool-call` feature to `starter-flow-nodes`.
- [rubix-agent/src/boot/mcp/register.rs](../../../crates/rubix-agent/src/boot/mcp/register.rs):
  registers `ToolCall::new(tool_call_registry)` as a builtin kind.
  The registry it dispatches against is a `StaticToolRegistry`
  built from the same `tool_registry_snapshot` the ai-agent node
  already uses — so REST, agent-loop, and flow tool-call all
  share one `Arc<dyn Tool>` per id (no in-memory-store divergence
  across surfaces).

### 3. Host-side YAML→slot bridge for tool-call nodes

The propagator only includes slots listed in `triggers[node]` when
assembling a node's input SlotMap, and the body reads `tool_id` +
`input` (not `tool_input`) from there. There was no mechanism to
project YAML `config:` into those slots. Pattern modeled after the
existing `cron_expr` projection for `trigger.schedule`. Changes:

- [rubix-flows/src/convert.rs](../../../crates/rubix-flows/src/convert.rs):
  for any `starter.flow.tool-call` node, append `tool_id` and
  `input` to `decl.triggers` so the propagator wakes the node and
  passes both values into invoke.
- [rubix-agent/src/boot/mcp/register.rs](../../../crates/rubix-agent/src/boot/mcp/register.rs):
  per-flow seed adapter now also emits one
  `(<node>.tool_id, "<id>")` + one `(<node>.input, <json>)` slot
  write per tool-call node, sourced from the node's
  `settings.tool_id` / `settings.tool_input` (YAML key). The
  `tool_input` JSON object gets `tick_epoch_ms` auto-injected from
  wall-clock if absent so the synth tool receives a fresh per-fire
  timestamp without YAML authors having to wire it.

### 4. Producer flow YAML

[rubix-flows/flows/data-flow-producer.yaml](../../../crates/rubix-flows/flows/data-flow-producer.yaml):
schedule(60s) → tool-call(synth.emit) → log(emit) → tool-call(disk
placeholder). Stage 02 flips the last node's `tool_id` to
`rubix.warehouse.ingest` — no other YAML changes needed.

### 5. Stage doc rewritten

[01-producer.md](./01-producer.md) now describes the
"synthesis-is-a-tool, delivery-is-a-flow" framework with the
locked decision and the tool/flow split rationale.

---

## Proof the manual e2e path works

Boot agent, log in, then:

```
POST /api/v1/mcp tools/call name=com.rubix.data-flow.producer
→ 200 (output is null because the flow's root is a schedule node
   whose `out` slot is never written; the output adapter sees
   nothing terminal).
```

Agent log lines (verbatim, from `/tmp/rubix-agent.log` at
`2026-05-25T23:10:05.26`) — three log.invoke spans for a single
run_id `85089ea8-…` showing:

- **Tick 1:** 2 rows (gap fired on `site-a.elec.main`,
  water + hvac emitted, `stats.gaps=1`).
- **Tick 2 (loop within same run):** 3 rows including elec.main,
  hvac jittered by ~11s (`epoch_ms` differs from MAIN's by 11776).
- **Quality flags:** all `ok` for this short run; over 1000-tick
  unit-test runs spikes/stuck/NaN all fire (asserted by
  `defaults_over_1000_ticks_produce_realistic_mess`).

This is the actual "tool reachable through a flow" proof. The
piece that's missing is the **scheduler driving the same path on
its 60-s cadence** — see below.

---

## Open problem — scheduler not claiming due rows this boot

Symptom: with the latest binary running (PID was the foreground
process started 23:09:51 UTC), the durable scheduler's tick task
seems alive (boot log shows `seeded=3`, `durable scheduler running
seeded=3`) but PG state at 23:13:35 still shows:

```
flow_id                       | next_run_at            | last_run_at                   | status
com.rubix.data-flow.producer  | 2026-05-25 23:10:00+00 | (null)                        | (null)
com.rubix.tick-counter        | 2026-05-25 23:09:55+00 | 2026-05-25 23:09:13.235853+00 | succeeded  (← stale, from previous boot)
```

`next_run_at` for both is in the past; `last_run_at` doesn't
advance. No `flow_as_service.tick.fired` log line, no
`flow_as_service.tick.failed` line either. The tick task simply
isn't observably running on this boot.

Earlier today (22:55 boot), tick-counter was firing every 60s
correctly through the same code path. The current boot's only
difference is my code (registered the tool-call kind + extended
the seed adapter). But:

- Tick-counter doesn't reference `tool-call` at all and should be
  unaffected.
- Manual MCP invocation of `com.rubix.data-flow.producer` works
  fine — meaning `FlowAsTool::invoke`, the seed adapter, the
  tool-call node, and the propagator all behave correctly.
- The scheduler's `FlowAsService::start` returns a `JoinHandle`
  that's then **dropped** by `boot::scheduler::spawn` returning
  `SchedulerHandle { task, .. }` — the caller has to keep the
  handle alive. If something in main() is dropping the handle on
  this boot, the tick task gets aborted. That's the next thing to
  check.

**First lead for the next session:** look at how
`SchedulerHandle` is consumed in
`rubix/crates/rubix-agent/src/boot/mod.rs` (or wherever
`scheduler::spawn` is called). On the prior successful boot the
handle was held in scope; on this boot it may be getting dropped.
Search for `SchedulerHandle` or `scheduler::spawn` and confirm the
task is kept alive for the agent's lifetime.

Other diagnostics:

- `cargo test -p rubix-tools --lib dataflow` → 8/8 green.
- `cargo test -p rubix-agent --lib registry::` → 15/15 green.
- `cargo build -p rubix-agent` → clean.
- The scheduler dispatch path through `ToolRegistryRunner` IS
  correct (proved by 22:55 boot, where data-flow.producer ran
  successfully twice end-to-end and its row's `last_run_status`
  was `succeeded` per the PG snapshot earlier in this session).

So the e2e *flow logic* is proven. The remaining gap is purely
"why isn't the tick task ticking on this boot."

---

## Other things noticed but not changed

- `boot/mcp/register.rs` is a long, dense file with several
  closures captured from `body.nodes`. My addition (`tool_call_seeds`,
  `tool_call_snapshot_for_kind`) follows the same shape but
  the file is now harder to read than it was — worth a small
  refactor pass later, not in this session's scope.
- `convert.rs` uses literal strings (`"starter.flow.tool-call"`,
  `"tool_id"`, `"input"`) where it could use
  `starter_flow_nodes::tool_call::{KIND_ID, TOOL_ID_SLOT,
  TOOL_INPUT_SLOT}`. I kept literals to avoid adding a
  workspace-level dep edge from rubix-flows to starter-flow-nodes.
  If that's acceptable architecturally, prefer the symbol
  references.
- Log-node duplication: each `log.invoke` event appears twice in
  the agent log per node invocation. Not new — was already there
  for tick-counter in pre-existing log lines. Out of scope here.
- The dirty tree has ~40 modified/untracked files from parallel
  AI sessions (rubix-agent routes, rubix-tools/dashboard,
  rubix-flows other YAMLs, mobile docs, frontend, …). I touched
  ONLY the files listed in "What landed" above. Commit must use
  explicit paths, not `git add -A`.

---

## Concrete files to commit when this stage is finally ✅

```
rubix/crates/rubix-spi/src/dto/mod.rs              (one-line add: pub mod dataflow;)
rubix/crates/rubix-spi/src/dto/dataflow/mod.rs     (new)
rubix/crates/rubix-spi/src/dto/dataflow/synth.rs   (new)
rubix/crates/rubix-tools/Cargo.toml                (added rand workspace dep)
rubix/crates/rubix-tools/src/lib.rs                (one-line add)
rubix/crates/rubix-tools/src/dataflow/mod.rs       (new)
rubix/crates/rubix-tools/src/dataflow/synth.rs     (new)
rubix/crates/rubix-tools/src/dataflow/meters.rs    (new)
rubix/crates/rubix-tools/src/dataflow/mess.rs      (new)
rubix/crates/rubix-tools/src/dataflow/tests.rs     (new)
rubix/crates/rubix-flows/src/convert.rs            (tool-call triggers)
rubix/crates/rubix-flows/flows/data-flow-producer.yaml  (new)
rubix/crates/rubix-agent/Cargo.toml                (tool-call feature flag)
rubix/crates/rubix-agent/src/registry.rs           (one-line tool registration + one test)
rubix/crates/rubix-agent/src/boot/mcp/register.rs  (tool-call kind + seed-adapter bridge)
rubix/docs/sessions/data-flow/01-producer.md       (rewritten)
rubix/docs/sessions/data-flow/2026-05-25-…-bridge.md  (this file)
```

Do NOT touch anything in the dirty list outside these paths.

---

## Pickup steps for the next session

1. Read this note + [01-producer.md](./01-producer.md) so you
   have the framework.
2. **Find out why the scheduler tick task isn't firing on the
   current boot.** Start by searching for `SchedulerHandle` /
   `scheduler::spawn` in `rubix/crates/rubix-agent/src/boot/`.
   Confirm the `JoinHandle` is held for the process lifetime
   (`std::mem::forget` or stored on a long-lived struct). If the
   handle is being dropped, fix that.
3. Once the scheduler is ticking, prove the producer fires on
   the cron:
   - `PAGER=cat psql … -c "SELECT flow_id, last_run_at, last_run_status FROM starter_scheduled_flows WHERE flow_id LIKE '%data%'"`
     should show `last_run_at` advancing each minute and
     `last_run_status = succeeded`.
   - `grep 'site-a' /tmp/rubix-agent.log | tail -10` should show
     wire-shape rows landing every ~60s.
4. Run for 5 minutes and tally against [01-producer.md "Success
   bar"](./01-producer.md#success-bar):
   - 5 producer runs (one per minute), each emitting 2–3 rows.
   - At least one tick shows `stats.gaps > 0` or `stats.emitted <
     3` (gap fired).
   - At least one tick shows `stats.spikes > 0` over the full run
     (need ~200 minutes at default `spike_prob = 0.005` to expect
     1 spike — for a 5-min run, force one by setting
     `DATA_FLOW_SPIKE_PROB=0.5` in the agent's env on restart).
5. Flip PROGRESS.md row 1 to ✅, fill date/commit/evidence per
   the checklist, commit with
   `feat(data-flow/01): producer e2e + tool-call YAML bridge`,
   and stop.

If the scheduler-task drop hypothesis is wrong, the next
diagnostic is to add a `tracing::info!` inside the tick loop in
`crates/starter-flow-surfaces/src/service.rs::FlowAsService::start`
before the `interval.tick().await` so you can prove the loop is
or isn't running. Don't bypass that with a `make restart`
without checking who else is on the stack — see
[USAGE.md §7](./USAGE.md#7-common-foot-guns-from-prior-sessions).
