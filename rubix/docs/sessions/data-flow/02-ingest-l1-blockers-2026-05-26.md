# 02 — two structural blockers prevent rows landing in `rubix.meter_readings_raw`

> Session note. Investigation only — **no code change**. Two
> long-standing seams in the agent boot wiring mean stage 02 cannot
> be completed end-to-end as the stage doc currently describes,
> even with a perfectly correct `rubix.warehouse.ingest`
> implementation. This note enumerates the seams, the failure mode
> each one produces, and the long-term fix for each. It deliberately
> avoids workarounds — the user asked for "rock solid", no hacks.

---

## Context

- **Stage:** 02 — raw landing in ClickHouse (L1).
- **Started from:** commit `3c10414` (master).
- **Trigger:** before writing a single line of `rubix.warehouse.ingest`
  I walked the stage doc's "Steps" against the actual agent boot
  wiring (`rubix-agent/src/registry.rs`,
  `rubix-agent/src/boot/mcp/register.rs`,
  `rubix-flows/flows/data-flow-producer.yaml`) and found two
  blockers that mean the documented path cannot land rows. Neither
  is hypothetical — both are visible directly in `master`.

## Blocker 1 — `rubix.clickhouse.*` verbs are wired to an in-memory writer

### What I found

[`rubix-agent/src/registry.rs:131`](../../../crates/rubix-agent/src/registry.rs#L131):

```rust
let ch_writer: Arc<dyn ChWriter> = Arc::new(InMemoryChWriter::new());
```

This single `Arc` is handed to every one of the seven
`rubix.clickhouse.*` tools at lines 190–196:

- `rubix.clickhouse.rule.list`
- `rubix.clickhouse.rule.write`
- `rubix.clickhouse.mart.list`
- `rubix.clickhouse.mart.create`
- `rubix.clickhouse.mart.drop`
- `rubix.clickhouse.tables.list`
- `rubix.clickhouse.retention.set`

[`rubix-tools/src/clickhouse/store.rs:178`](../../../crates/rubix-tools/src/clickhouse/store.rs#L178)
defines `InMemoryChWriter` — it stores rules / marts / retentions
in a `Mutex<HashMap>` inside the agent process. **Nothing reaches
the live ClickHouse**. Every test in the `clickhouse/*.rs` files
also uses `InMemoryChWriter::new()`, so the unit test suite gives
no signal here.

The registry module docs (lines 34–36) flag this explicitly as a
tracked follow-up:

> Replace `InMemoryChWriter` with a `starter-store-clickhouse`
> `ChClient`-backed impl so the seven `rubix.clickhouse.*` verbs
> land DDL against the live warehouse.

### Failure mode this produces for stage 02

Stage 02 step 1 says:

```bash
curl -X POST .../api/v1/tools/rubix.clickhouse.rule.write \
  -d '{ "rule_name": "meter_readings_raw", "body": "CREATE TABLE rubix.meter_readings_raw ( ... )" }'
```

…returns `HTTP 200` with a payload that looks correct, but the
table never appears in ClickHouse. The downstream
`rubix.warehouse.ingest` insert will fail with
`UNKNOWN_TABLE: meter_readings_raw` and stage 02's success-bar
`SELECT count() …` returns 0 forever. The agent log will be
silent because the verb itself succeeded — the bug is one layer
deeper than a logged error.

Same applies to step 2's `retention.set`: it silently mutates a
process-local map. Even if step 1 worked, retention never reaches
the table.

### Long-term fix (no hacks)

Build a `ChClient`-backed `ChWriter` adapter in
`rubix-tools/src/clickhouse/store.rs` (the trait already exists
and was designed for this swap — see the file-level doc at line 7).
Concretely:

1. Add `struct ChClientWriter { inner: Arc<ChClient>, database: String }`
   alongside `InMemoryChWriter`. Implement every `ChWriter`
   method by translating the verb body into the SQL statements
   `clickhouse/*.rs` already compose. The translation is small
   because the verbs construct SQL today and hand it to the
   writer — keep that boundary, just send the SQL over the wire
   instead of into a HashMap.
2. The `database` field pins all queries to `rubix` (matches
   `rubix-agent/src/boot/clickhouse.rs::RUBIX_CH_DATABASE`). The
   adapter must call `client.inner().query(&sql).execute()` per
   `system::disk::write_history` (no parallel quoting / quoting
   helpers — reuse what works).
3. In `registry.rs`, swap the `let ch_writer = …` line:
   ```rust
   let ch_writer: Arc<dyn ChWriter> = match ch.as_ref() {
       Some(client) => Arc::new(ChClientWriter::new(client.clone(), RUBIX_CH_DATABASE.to_owned())),
       None         => Arc::new(InMemoryChWriter::new()),
   };
   ```
   Mirrors the existing PG fallback at lines 124 and 142.
4. Keep `InMemoryChWriter` — it's load-bearing for the
   `rubix-tools` unit tests that don't have a live CH. The trait
   stays; this is purely additive.
5. Add **one** integration test in `rubix-agent/tests/` that
   stands up CH (the harness already exists for `0002_history`),
   calls `rubix.clickhouse.rule.write` to create a throwaway
   table, and `SELECT`s it back to prove DDL reached the wire.
   Pattern: `goal_3_flow_programmer_test.rs` (mentioned in
   `registry.rs:16`).

Cost: ~300 LOC of adapter + 1 integration test. Blast radius is
the seven `rubix.clickhouse.*` verbs — but those are already
broken in production today (no rows ever land), so the swap can
only improve correctness. Zero schema migration risk because the
trait surface doesn't change.

## Blocker 2 — synth rows have no path to ingest's `tool_input` slot

### What I found

The producer flow YAML
([`rubix-flows/flows/data-flow-producer.yaml`](../../../crates/rubix-flows/flows/data-flow-producer.yaml))
links the three nodes like this:

```yaml
links:
  - { from: "tick.fire",    to: "synth.in"     }
  - { from: "synth.output", to: "emit.value"   }
  - { from: "emit.emitted", to: "ingest.in"    }
```

`ingest.in` is the **trigger** slot for the `tool_call` kind
(`starter-flow-nodes/src/tool_call.rs:68` —
`pub const TOOL_IN_SLOT: &str = "in"`). Triggering a `tool_call`
node fires its body, which reads from two separate *read* slots:

- `tool_id`  (`TOOL_ID_SLOT`, line 78)
- `input`    (`TOOL_INPUT_SLOT`, line 85)

Both are populated **exclusively** by the seed adapter at
[`rubix-agent/src/boot/mcp/register.rs:463`](../../../crates/rubix-agent/src/boot/mcp/register.rs#L463),
which projects the YAML's static `settings.tool_id` and
`settings.tool_input` onto the slots on every fire.

For the `ingest` node in the current YAML,
`settings.tool_input` is the literal object `{}`. Auto-injection
adds `tick_epoch_ms` but nothing else. **Synth's rows never enter
ingest's `input` slot, so the `tool_call` body has no rows to
forward to `rubix.warehouse.ingest`.**

The seed-adapter contract also explicitly states (lines 388–397
of `register.rs`) that `tool_id` and `input` are *read* slots and
"re-writing them here per invoke therefore does **not** wake the
node — only the upstream link (`tick.fire → synth.in`) does." So
even an engine-level link `synth.output → ingest.input` may not
deliver: the seed adapter rewrites `input` to the YAML value on
every fire, racing with whatever the link wrote. Without
verifying the engine's slot-write ordering, the outcome is
undefined.

### Failure mode this produces for stage 02

Two equally bad outcomes depending on how the seed adapter and
the engine link interact:

- **Adapter wins** (most likely, since it writes unconditionally
  in the seed phase): ingest invokes `rubix.warehouse.ingest`
  with `{ "tick_epoch_ms": … }` and no rows. The tool either
  errors on missing `rows`, or successfully writes zero rows.
  CH count stays at 0.
- **Link wins (race)**: works some of the time, fails other
  times, with no stable repro across runs.

Either way, stage 02's success bar `count(*) ≥ 200` cannot land.

### Long-term fix (no hacks)

The right fix is to **make `tool_input` link-driven, not seed-
driven, when an upstream link is wired to it**, and to keep
the YAML projection only as a *default* for nodes with no upstream
link. Two parts:

1. **In `boot/mcp/register.rs`**, change the `tool_call_seeds`
   projection to skip the `TOOL_INPUT_SLOT` write when the flow
   has an inbound link writing to that slot:

   ```rust
   let target_slot = SlotRef::new(node_id.clone(), TOOL_INPUT_SLOT);
   let has_link_into_input = body.links.iter().any(|l| l.to == target_slot);
   if !has_link_into_input {
       // seed the YAML default
   }
   ```

   Same shape for `TOOL_ID_SLOT` if a future flow ever wants a
   dynamic tool id (none today; keep it static for now and
   document the policy).

2. **In the producer YAML**, switch the third link to feed
   ingest's `input` directly:

   ```yaml
   links:
     - { from: "tick.fire",    to: "synth.in"     }
     - { from: "synth.output", to: "emit.value"   }
     - { from: "synth.output", to: "ingest.input" }
   ```

   The `emit` node stays in place for observability; its
   `emitted` output no longer drives ingest. Ingest's `in`
   trigger is implicitly fired by the engine when its
   `input` slot becomes non-stale — verify this on the
   `starter-flow` engine side (see `propagator.rs`); if the
   engine requires an explicit trigger, add a parallel link
   `synth.output → ingest.in` (carries a sentinel) alongside
   the `input` link.

3. **Schema-check at deploy**: `rubix.flow_ops.lint` should
   reject a YAML in which a `tool_call` node has no `tool_input`
   value **and** no inbound link to `input` — that's the bug
   that bit stage 02 today, and it should be impossible to
   reproduce. Add one lint rule + unit test.

Cost: ~50 LOC across `register.rs`, the YAML, and the lint
rule. Blast radius is exactly the `tool_call` kind — no engine
internals change. The contract becomes "static `tool_input` is
a default; an inbound link wins" which is the principle the
rest of the engine already follows (links carry runtime data,
YAML carries defaults).

## What this means for stage 02 implementation

Until **both** blockers above are fixed, the work I can complete
in good conscience is:

- The `rubix.warehouse.ingest` tool itself (~150 LOC of
  `Tool` impl + DTO + descriptor + unit tests). The shape is
  unambiguous: takes `{ tenant_id, rows: [MeterReading] }`,
  iterates rows through a single multi-row `INSERT INTO
  rubix.meter_readings_raw VALUES …`, returns counts. The
  unit test seam matches `system::disk::write_history`.
- The L1 DDL itself, lifted into a bundled migration at
  `rubix-agent/migrations/0003_meter_readings_raw/up.sql`
  (matches `0002_history/up.sql` layout exactly), so the table
  exists at boot without depending on the broken
  `rule.write` verb. This is **not** a workaround for blocker 1
  — it's the right home for warehouse-owned L1/L2/L3 tables
  per `0002_history`'s precedent. `rule.write` / `mart.create`
  are for operator-authored rules and marts, not for the
  hard-coded core schema.

But none of those pieces produce a green stage-02 success bar
without the two blockers being fixed. The success bar's
`SELECT count() FROM rubix.meter_readings_raw` will return 0
until synth rows actually reach the ingest tool.

## What I changed

**No code change.** Investigation only.

## What's left

- [ ] **Fix blocker 1** — `ChClient`-backed `ChWriter` adapter +
      registry swap + integration test. See section above for
      shape and cost.
- [ ] **Fix blocker 2** — make `tool_input` link-driven when an
      inbound link is wired, plus producer YAML edit, plus
      `flow_ops.lint` rule. See section above for shape and cost.
- [ ] Once both blockers are fixed: bind `rubix.warehouse.ingest`
      against the ingest tool's `ChClient` seam, point the
      producer YAML link at `ingest.input`, run 5 min, verify
      bar 1–3 land. Stage doc text in
      [`02-ingest-l1.md`](./02-ingest-l1.md) should also be
      amended to reflect that L1 DDL lives in a bundled
      migration, not in `rule.write`. That edit needs the user
      because the doc's Decisions block has a "Path A / Path B"
      choice that no longer matches the real seam.

The follow-up rows in [PROGRESS.md](./PROGRESS.md) should
reference this note under the stage 02 line once they're added.

## References

- Stage doc: [./02-ingest-l1.md](./02-ingest-l1.md)
- Producer flow YAML: [`rubix-flows/flows/data-flow-producer.yaml`](../../../crates/rubix-flows/flows/data-flow-producer.yaml)
- Registry (in-memory CH writer): [`rubix-agent/src/registry.rs:131`](../../../crates/rubix-agent/src/registry.rs#L131)
- Tool-call node behaviour: [`starter-flow-nodes/src/tool_call.rs:68-85,180-200`](../../../../crates/starter-flow-nodes/src/tool_call.rs#L68)
- Seed adapter (tool_input projection): [`rubix-agent/src/boot/mcp/register.rs:374-497`](../../../crates/rubix-agent/src/boot/mcp/register.rs#L374)
- ChWriter trait + in-memory impl: [`rubix-tools/src/clickhouse/store.rs`](../../../crates/rubix-tools/src/clickhouse/store.rs)
- Warehouse DB pin: [`rubix-agent/src/boot/clickhouse.rs:47`](../../../crates/rubix-agent/src/boot/clickhouse.rs#L47)
- L1/L2/L3 layering rule: [`design/warehouse/README.md`](../../design/warehouse/README.md)
- Prior stage 01 note (multi-fire root cause): [`2026-05-26-data-flow-01-producer-multi-fire.md`](./2026-05-26-data-flow-01-producer-multi-fire.md)
