# 2026-05-25 — `com.rubix.tick-counter` shows no live values

## Symptom

On `http://127.0.0.1:5173/flows/com.rubix.tick-counter` the canvas
renders the three nodes (`tick → count → emit`) but **no slot value
badges appear** on the `count` node, no matter how long the page is
open. Restarting the backend does not fix it.

The SSE endpoint
`GET /api/v1/flows/com.rubix.tick-counter/events` connects with
`content-type: text/event-stream` but emits zero events.

## Root causes (four, in a stack)

The bundled tick-counter flow is supposed to fire every 5s,
increment a counter, and emit the new value on SSE. Four
independent bugs each blocked that pipeline; they only became
visible one at a time as each was removed.

### 1. Scheduler swallowed every dispatch error as `"internal error"`

`boot/scheduler.rs::ToolRegistryRunner::run` invoked the
`FlowAsTool` then logged `error = %e`. The underlying
`SpiError::Internal` has a `Display` impl that prints the literal
string `"internal error"` and stashes the real failure on
`source()`. The log line therefore always read:

```
WARN rubix.boot.scheduler: scheduled flow dispatch failed
  flow_id=com.rubix.tick-counter error=internal error
```

…hiding the actual cause. Fixed by walking `Error::source()` and
joining the chain before logging.

### 2. `trigger.schedule` node never received its `cron_expr` slot

The unfolded error turned out to be:

```
flow run failed: node com.rubix.tick returned
trigger.schedule input missing `cron_expr` slot
(must be SlotValue::String carrying the cron expression)
```

The node body reads `cron_expr` from its **input SlotMap**, not
from `config:`. Until `TopologyResolver` lands (HR5) the surface
has to project the YAML `cron_expr` into the runtime input slot
itself.

Fixed by:

- `boot/mcp/register.rs::register_one` — when the root node kind
  is `starter.flow.trigger.schedule`, capture
  `settings.cron_expr` at register time and have the seed
  adapter emit it as a `SlotValue::String` on
  `(root_id, "cron_expr")`.
- `rubix-flows/src/convert.rs` — list `cron_expr` in the
  trigger.schedule node's `triggers:` so the propagator copies
  the seeded value into the node's input map.

### 3. `FlowAsTool`'s engine used noop NodeStateStore and had no event sink

`boot/mcp/mod.rs::build_engine()` built an `Engine` from a
fresh in-memory `GraphStore` with **no** `NodeStateStore` and
**no** event hook. That meant:

- the counter node ran against
  `NoopNodeStateStore`, so persisted `count` was always
  `None` → no progress, but no error either;
- the per-run `FlowEvent` broadcast lived inside the
  `RunHandle` and was dropped when the run ended → the
  always-on `FlowSubscriptionRegistry` the SSE route reads
  never received a single frame.

Fixed by plumbing the existing `FlowRuntime` (built in
`boot::build_flow_runtime`) into the MCP engine:

- `starter-flow::engine::Engine` — added
  `with_node_state_store()` and `with_event_sink()` builders +
  getters.
- `starter-flow::lib.rs` — added a tiny `FlowEventSink` trait
  with a sync `publish(&FlowId, FlowEvent)`.
- `starter-flow::run::FlowRunner` — added
  `with_node_state_store()` and threaded the store into
  `propagator::spawn_with_checkpoint`.
- `starter-flow-surfaces::FlowAsTool` — pulls
  `Engine::node_state_store()` + `Engine::event_sink()` and
  (a) hands the state store to `FlowRunner::with_node_state_store`,
  (b) extends the existing per-run event watcher to also forward
  every `FlowEvent` to the sink.
- `rubix-agent::boot::flow_runtime::FlowSubscriptionRegistry` —
  switched the per-flow channel map from `tokio::sync::RwLock`
  to `std::sync::RwLock` and implemented `FlowEventSink` so
  `publish` can run synchronously from the tokio task without
  an `await`.
- `rubix-agent::main.rs` — build `flow_runtime` **before**
  `build_mcp_surface` and pass it through
  `build_mcp_surface → build_tool_registry → build_flow_registry → build_engine`.

### 4. Counter / log nodes had no inbound triggers

`rubix-flows::convert` set every node's `triggers` to just the
shared `payload` seed slot. Even with the trigger node now
emitting on its `schedule` output and the YAML link
`tick.schedule → count.in` in place, the propagator ignored the
write because `count` did not list `in` as a trigger input.

Also the original YAML used `tick.fire → count.in`, but the
trigger node actually emits on `schedule` (see
`trigger_schedule::SCHEDULE_SLOT`).

Fixed by:

- `tick-counter.yaml` — link source corrected to
  `tick.schedule → count.in`.
- `rubix-flows::convert` — after building the link list, walk
  each link and add the destination slot to the destination
  node's `triggers` if not already present.

Bonus: had to mark the existing PG row superseded
(`UPDATE flows_definitions SET superseded_at=NOW() WHERE flow_id='com.rubix.tick-counter'`)
because the seeder is idempotent — it skips when a live row
already exists, so YAML edits don't reach the running registry
without a manual bump until the hot-reload listener is wired to
re-publish.

## Files touched

```
crates/starter-flow/src/engine.rs
crates/starter-flow/src/lib.rs
crates/starter-flow/src/run.rs
crates/starter-flow/src/propagator.rs
crates/starter-flow-surfaces/src/lib.rs
rubix/crates/rubix-flows/src/convert.rs
rubix/crates/rubix-flows/flows/tick-counter.yaml
rubix/crates/rubix-agent/src/main.rs
rubix/crates/rubix-agent/src/boot/scheduler.rs
rubix/crates/rubix-agent/src/boot/flow_runtime.rs
rubix/crates/rubix-agent/src/boot/mcp/mod.rs
rubix/crates/rubix-agent/src/boot/mcp/register.rs
rubix/crates/rubix-agent/src/bin/rubix_admin/mcp/serve.rs
```

## Manual verify

```bash
# Force re-seed of the YAML (bundled body is cached in PG).
PGPASSWORD=rubix-dev psql -h 127.0.0.1 -p 5433 -U rubix rubix \
  -c "UPDATE flows_definitions SET superseded_at=NOW()
      WHERE flow_id='com.rubix.tick-counter';"

# Wipe any old half-state.
rm -f ~/.rubix/node_state.db

# Restart agent, wait one full scheduler tick interval (~60s).
pkill -9 -f 'target/debug/rubix-agent'
RUBIX_DSN=postgres://rubix:rubix-dev@127.0.0.1:5433/rubix \
RUBIX_CONFIG=rubix/dev/agent.toml \
RUBIX_CH_URL=http://127.0.0.1:8124 \
./target/debug/rubix-agent &

sleep 75

# Counter state should now have a climbing integer.
sqlite3 ~/.rubix/node_state.db \
  "SELECT flow_id, node_id, key, CAST(value AS TEXT), version
     FROM node_state;"

# SSE should now emit RunStarted / NodeEmitted / RunCompleted frames.
curl -s -b /tmp/c.txt -N \
  "http://127.0.0.1:8088/api/v1/flows/com.rubix.tick-counter/events" \
  --max-time 10
```

In the browser the `.sf-slot__value` badge on the `count` node
should display a climbing integer that survives a hard refresh.

## Lessons

- **Never `Display` an error type whose outer face is a generic
  label** (`SpiError::Internal` prints `"internal error"`). At
  log boundaries always walk `Error::source()` so the chain is
  visible.
- The flow registry is split: bundled YAML → PG `flows_definitions`
  is **idempotent** (skips on existing live row). Edits to the
  bundled file are invisible to the running registry until the
  PG row is superseded by hand or hot-reload republishes. Worth
  noting in `DOCS/flow/scope/hot-reload.md`.
- The propagator's `triggers:` set is load-bearing. Authoring
  flows by writing only `links:` is intuitive but the engine
  ignores writes to slots that aren't listed as triggers on the
  destination — so `convert::yaml → FlowBody` has to derive
  triggers from inbound link targets.
- `FlowAsTool` (and `FlowAsService`) by default use whatever
  `Engine` they're handed. Hosts that want shared state /
  shared event fan-out across both the always-on runtime and
  the on-demand tool surface must wire the `Engine` builder
  hooks — there is no implicit hand-off.
