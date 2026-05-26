# Stage 01 — synthesis tool + producer flow

> **Supersedes the earlier "flow-only rhai producer" draft.** That
> shape assumed a `script`/rhai node kind in `starter-flow-nodes`
> that does not exist (the engine ships `schedule`, `counter`,
> `log`, `tool_call`, `transform`, `gate`, `branch`, `merge`,
> `sleep`, `http_out`, `subflow`, `ai_agent` — `transform` is a
> registry of host-built Rust closures, not a scripting surface).
> The split below is the long-term framework: **synthesis is a
> tool, delivery is a flow.** It is the same shape every future
> "synthetic / replay / fixture" source should use.

## Why this split (read once, then never again)

Synthesis and delivery are two different jobs and the docs used to
conflate them:

| Job         | What it does                                | Right home                                    |
|-------------|---------------------------------------------|-----------------------------------------------|
| Synthesis   | Produce N wire rows for "this tick" with    | A **tool** in `rubix-tools`. Pure Rust, seed- |
|             | RNG, NaN, jitter, gap/spike/stuck state.    | able, unit-testable, no engine coupling.      |
| Delivery    | Fire every 60 s, hand rows to the warehouse.| A **flow**: `schedule → tool_call(synth)      |
|             |                                             | → tool_call(ingest)`. Declarative, no code.   |

Consequences of this split:

- **Mess injection is `cargo test`-able.** A unit test with a
  fixed seed asserts that after K ticks, spike count / gap count /
  stuck-zero stretches fall in the expected bands. No flow engine,
  no ClickHouse, no agent. CI gets a real regression gate.
- **Flows stay declarative.** No new node kind, no scripting
  language to maintain, no "what's in scope for rhai inside the
  flow" debate.
- **It generalises.** "Replay a Parquet of real meter data" or
  "fuzz the ingest path with bad shapes" is another tool with the
  same interface. Flow YAML barely changes.
- **Survives parallel sessions.** New files live under
  `rubix-tools/src/dataflow/` and one new YAML under
  `rubix-flows/flows/`. No edits to engine internals other
  sessions may be touching.

## Scope

**In**

- A new tool **`rubix.dataflow.synth.emit`** in
  [rubix/crates/rubix-tools/src/dataflow/](../../../crates/rubix-tools/src/dataflow/)
  (new module). Takes per-tick args, returns `0..N` wire-shape rows.
- Seeded RNG + in-process per-meter state for stuck-zero stretches.
- A new flow **`com.rubix.data-flow.producer`** at
  [rubix/crates/rubix-flows/flows/data-flow-producer.yaml](../../../crates/rubix-flows/flows/data-flow-producer.yaml)
  driving three meters at their nominal cadence.
- Unit tests for the synth tool covering all four mess shapes.

**Out**

- Persistence. Stage 02 owns `rubix.warehouse.ingest` and the L1
  table. Until that tool exists, the producer flow points its
  delivery `tool_call` at `rubix.system.disk` (a no-op for our
  purposes) and stage 01's success bar asserts on synth output.
- Real-meter integration. Replay / Parquet / MQTT are future
  siblings of this tool, not part of stage 01.
- Multi-tenancy. One `site-a` tenant only.

## Wire shape (locked — both this tool and the L1 path emit this)

```json
{
  "tenant_id": "site-a",
  "meter_id":  "site-a.elec.main",
  "kind":      "electricity",
  "unit":      "kWh",
  "epoch_ms":  1748275200000,
  "value":     12345.678,
  "quality":   "ok"
}
```

- `kind` ∈ `{electricity, water}`.
- `unit` ∈ `{kWh, L}`.
- `value` is the cumulative meter reading (monotonic in clean
  data; non-monotonic when spike or stuck-zero fires).
- `quality` ∈ `{ok, suspect, missing}`. The synth tool emits
  `suspect` for injected spikes / NaN. Gap intervals **return zero
  rows** (no `missing` placeholder — absence is the signal stage 03
  has to cope with).

## Tool contract — `rubix.dataflow.synth.emit`

**Request** (`SynthEmitRequest`):

```json
{
  "tenant_id":  "site-a",
  "meters":     ["site-a.elec.main", "site-a.water.main", "site-a.elec.hvac"],
  "tick_epoch_ms": 1748275200000,
  "knobs": {
    "seed":         42,
    "gap_prob":     0.02,
    "spike_prob":   0.005,
    "stuck_prob":   0.001,
    "jitter_ms":    20000,
    "nan_prob":     0.0005
  }
}
```

All `knobs` fields are optional; missing fields fall back to the
defaults below. `knobs.seed` is the **only** stateful input — same
seed + same tick sequence ⇒ same output (modulo per-meter stuck
state held in-process; tests reset it explicitly).

**Response** (`SynthEmitResponse`):

```json
{
  "rows": [ /* zero or more wire-shape rows */ ],
  "stats": {
    "emitted":      3,
    "gaps":         0,
    "spikes":       0,
    "stuck_active": 0,
    "nans":         0
  }
}
```

`stats` is for observability + the success bar; downstream nodes
key off `rows` only.

**Knob defaults** (env-fallback for ops; request args win):

| Knob           | Env var                 | Default | Effect                                         |
|----------------|-------------------------|---------|------------------------------------------------|
| `seed`         | `DATA_FLOW_SEED`        | `42`    | RNG seed (repeatable test runs)                |
| `gap_prob`     | `DATA_FLOW_GAP_PROB`    | `0.02`  | per-tick prob of dropping a meter's row        |
| `spike_prob`   | `DATA_FLOW_SPIKE_PROB`  | `0.005` | per-tick prob of a ×50 spike (`quality=suspect`)|
| `stuck_prob`   | `DATA_FLOW_STUCK_PROB`  | `0.001` | per-tick prob of *starting* a 10–30 min stretch|
| `jitter_ms`    | `DATA_FLOW_JITTER_MS`   | `20000` | uniform ± jitter on `epoch_ms`                 |
| `nan_prob`     | `DATA_FLOW_NAN_PROB`    | `0.0005`| per-tick prob of NaN (`elec.hvac` only)        |

Per-meter behaviour:

- `site-a.elec.main` — electricity, kWh, eligible for gap + spike.
- `site-a.water.main` — water, L, eligible for stuck-zero.
- `site-a.elec.hvac` — electricity, kWh, eligible for jitter + NaN.

These eligibilities match the scenario table in
[README.md "The scenario"](./README.md#the-scenario). The tool
encodes them so callers can't get the mess wrong.

## Internal layout

```
rubix/crates/rubix-tools/src/dataflow/
├── mod.rs              # exports SynthEmitTool, registers DTOs
├── synth.rs            # SynthEmitTool + Tool impl + DI wiring
├── meters.rs           # per-meter state machine (cumulative value, stuck timer)
├── mess.rs             # gap / spike / stuck / jitter / NaN injectors, seedable
└── tests.rs            # unit tests under #[cfg(test)]
```

State held by the tool itself:

- `seed` + `rand_chacha::ChaCha8Rng` (deterministic, not the OS RNG).
- A `Mutex<HashMap<MeterId, MeterState>>` where `MeterState`
  carries `{ cumulative: f64, stuck_until: Option<i64> }`.

Why `Mutex`: this tool is called once per tick (60 s) for ~3 meters.
Contention is zero. Keeping it simple beats a lock-free thing nobody
needs.

DTOs live in `rubix-spi/src/dto/dataflow/synth.rs` so other crates
can speak to it without depending on `rubix-tools`.

## Producer flow

[rubix/crates/rubix-flows/flows/data-flow-producer.yaml](../../../crates/rubix-flows/flows/data-flow-producer.yaml):

```yaml
id: com.rubix.data-flow.producer
description: |
  Stage 01 messy energy + water producer. Schedule fires every 60s;
  synth tool returns 0..N wire rows; ingest tool persists them.
  Until stage 02 binds rubix.warehouse.ingest, the second tool_call
  points at rubix.system.disk (no-op) and the success bar is read
  off the synth node's stats slot.
trigger: schedule
cron_expr: "0 * * * * *"

nodes:
  - id: tick
    kind: starter.flow.trigger.schedule
    config:
      cron_expr: "0 * * * * *"
  - id: synth
    kind: starter.flow.tool_call
    config:
      tool: "rubix.dataflow.synth.emit"
      args:
        tenant_id: "site-a"
        meters: ["site-a.elec.main", "site-a.water.main", "site-a.elec.hvac"]
        # tick_epoch_ms injected by the engine from the schedule fire
  - id: ingest
    kind: starter.flow.tool_call
    config:
      tool: "rubix.system.disk"   # placeholder; flip to rubix.warehouse.ingest in stage 02

links:
  - { from: "tick.fire",   to: "synth.in" }
  - { from: "synth.out",   to: "ingest.in" }
```

Stage 02's first action will be to flip `ingest.config.tool` to
`rubix.warehouse.ingest`. That is the *only* change to this YAML
across stages 01 → 02.

## Pre-flight

- ClickHouse + Postgres are up:
  `mani run dev-deps` from [rubix/](../../../). Verify with
  `docker ps | grep rubix-`.
- `cargo build -p rubix-tools -p rubix-agent` is clean.
- Other AI sessions: `git status` shows files you do not own
  modified. **Leave them alone.** Your stage owns only:
  - `rubix/crates/rubix-tools/src/dataflow/**` (new)
  - `rubix-spi/src/dto/dataflow/**` (new)
  - `rubix/crates/rubix-flows/flows/data-flow-producer.yaml` (new)
  - Tool-registry insertion site in
    [rubix-agent/src/registry.rs](../../../crates/rubix-agent/src/registry.rs)
    — add **one** line; do not refactor surrounding code.

## Steps

1. **DTOs.** Add `SynthEmitRequest` / `SynthEmitResponse` /
   `SynthKnobs` / `SynthStats` under `rubix-spi/src/dto/dataflow/`
   with `serde` + reverse-DNS message keys matching the rest of
   `rubix-spi`.

2. **Mess injectors.** Implement each shape in `mess.rs` against
   a `ChaCha8Rng` handle. Each injector is a pure function of
   `(rng, knobs, meter_state)`. Unit-test each one in isolation
   before composing.

3. **Tool.** Implement `SynthEmitTool: Tool` in `synth.rs`. Pattern
   to follow: [rubix/crates/rubix-tools/src/system/disk.rs](../../../crates/rubix-tools/src/system/disk.rs)
   for the `Tool` impl shape (Default, Debug, async invoke, MessageKey
   taxonomy). The body composes meter state + mess injectors and
   returns rows + stats.

4. **Register.** Add the tool to the registry in
   `rubix-agent/src/registry.rs` next to the other `rubix.*` tools.
   One line. Resist the urge to clean up the file.

5. **Flow.** Drop `data-flow-producer.yaml` above into
   `rubix-flows/flows/`. Confirm the bundled flows loader picks it
   up (other YAMLs in that directory are auto-discovered at agent
   start).

6. **Unit tests** in `tests.rs`:
   - `seed=42, 60 ticks, gap_prob=1.0` → every tick returns 0 rows
     for the gap-eligible meter.
   - `seed=42, 60 ticks, spike_prob=1.0` → every elec.main row has
     `quality="suspect"` and `value ≥ 50× the prior clean value`.
   - `seed=42, 1000 ticks, stuck_prob=1.0` → water.main produces a
     run of identical `value` of length ∈ [10, 30].
   - `seed=42, 1000 ticks, nan_prob=1.0` → every elec.hvac row has
     `value.is_nan() && quality=="suspect"`.
   - `seed=42, 1000 ticks, all knobs default` → emitted ∈ [2700,
     3000] (3 meters × 1000 ticks − gaps − stuck overlap),
     spike_count ≥ 1, gap_count ≥ 1.

7. **End-to-end.** Start the agent (`cargo run -p rubix-agent`),
   wait 5 minutes, then assert on the success bar below.

## Success bar

Stage 01 is done when **all four** are true:

1. The producer flow runs 5 minutes without panicking
   (`rubix.flow_ops.list` shows it with non-null `revision_id` and
   no `FlowFailed` events in the agent log).
2. The `synth` node's `stats.emitted` sum over 5 minutes is in
   `[10, 18]` (3 meters × ~5 fires − expected gaps). The durable
   scheduler claims at a 60-s `tick_interval_seconds` regardless
   of the YAML cron, so a `*/5 * * * * *` cron still fires the
   producer once per minute. See
   [`2026-05-26-data-flow-01-producer-multi-fire.md`](./2026-05-26-data-flow-01-producer-multi-fire.md)
   for the derivation; the prior `[235, 300]` range assumed a
   sub-second claim cadence that does not exist.
3. `synth.stats.spikes` is `> 0` (spike injection fired at least
   once across the run).
4. At least one tick across the run shows `stats.emitted < 3`
   (gap injection fired at least once).

Read the stats off the agent log (the engine logs each
`tool_call.invoke` span at info) or off the SSE feed at
`/api/v1/flows/com.rubix.data-flow.producer/events` if it is
mounted in your branch. Unit-test pass alone is necessary but not
sufficient — the flow must actually fire.

## If it fails

In order, check:

1. **Tool not registered.** `rubix.flow_ops.list` will surface the
   flow but the agent log shows `tool_call` failing with
   `UnknownTool("rubix.dataflow.synth.emit")`. Re-check the
   registry edit in step 4.
2. **Schedule node not firing.** Drop a trivial
   `schedule → log` flow alongside; if that also doesn't fire the
   bug is in `starter-flow-nodes::trigger_schedule`, not here.
3. **Mess knobs all default but mess never appears.** Set one knob
   to `1.0` via the env var and rerun. If mess still doesn't
   appear, the RNG is being re-seeded every tick (the tool is
   constructing a new RNG instead of holding one across calls);
   fix that before anything else.

If none of those is the cause, write a follow-up session note as
`YYYY-MM-DD-data-flow-01-producer-<topic>.md` per the convention
in [PROGRESS.md "Follow-up notes"](./PROGRESS.md#follow-up-notes-spillover)
and stop. Do not start stage 02 until stage 01's success bar is
green.

## Decisions taken

- [x] Synthesis as a **tool**, delivery as a **flow** (this doc).
- Tool id: `rubix.dataflow.synth.emit`.
- Flow id: `com.rubix.data-flow.producer`.
- Wire shape: above (locked once stage 02 starts).
- RNG: `rand_chacha::ChaCha8Rng`, seeded from `knobs.seed`
  (default 42), held across calls by the tool instance.
- Stuck-zero state: in-process `Mutex<HashMap<MeterId, MeterState>>`
  on the tool. Not persisted — process restart resets stuck
  timers. Acceptable for v0; revisit if stage 04 needs replayable
  state.

## Open decisions (next session decides)

| # | Choice                                                          | Notes |
|---|-----------------------------------------------------------------|-------|
| 1 | Per-meter cadence: model 5-min water + 60-s elec as two timer ticks vs one 60-s tick that skips water 4/5 of the time | Latter is simpler; former is more honest. Defaulting to latter unless stage 03's bucketiser objects. |
| 2 | Where stuck-state lives long-term: tool-local Mutex (now) vs `NodeStateStore` (engine-managed) | Mutex for v0. Promote to NodeStateStore the moment a second tool needs the same pattern. |
| 3 | Replay/Parquet sibling tool: same module (`dataflow/`) vs separate (`dataflow_replay/`) | TBD when the second source lands; not part of stage 01. |
