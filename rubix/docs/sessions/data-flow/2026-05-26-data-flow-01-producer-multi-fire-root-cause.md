# 2026-05-26 — stage 01 multi-fire: root cause is in the rubix surface seed adapter, not the engine

## Status: ⏳ partial — root cause located and proven; one focused surface-layer fix remains

This note supersedes the diagnosis half of
[2026-05-26-data-flow-01-producer-multi-fire.md](./2026-05-26-data-flow-01-producer-multi-fire.md).
The bug is not in `convert.rs`, the propagator, or the in-memory
graph store. It is in the rubix host-side seed adapter at
[rubix-agent/src/boot/mcp/register.rs:407-486](../../crates/rubix-agent/src/boot/mcp/register.rs#L407-L486),
which writes three slots (`payload`, `tool_id`, `input`) on **every
flow invoke** for each tool-call node — and the engine's slot-change
event stream legitimately turns each distinct write into a wake.

The earlier "engine fix via `write_slot_batch`" change in this
session [crates/starter-flow-spi/src/graph.rs](../../../crates/starter-flow-spi/src/graph.rs)
+ [crates/starter-flow/src/graph.rs](../../../crates/starter-flow/src/graph.rs)
+ [crates/starter-flow/src/run.rs](../../../crates/starter-flow/src/run.rs)
helps a different case (seed writes for *one* node coalescing into
one wake) but is not sufficient because the link-fanout path
(`tick.fire → synth.in`) is a *separate* `write_slot` call after the
seed batch, and so still wakes the node a second time.

## The proof: engine works, surface doesn't

New integration test
[crates/starter-flow-nodes/tests/twenty_schedules_drive_counters.rs](../../../crates/starter-flow-nodes/tests/twenty_schedules_drive_counters.rs)
builds 20 isolated `(trigger.schedule → counter)` pairs into a
single `FlowTopology`, drives the propagator directly (no rubix
surface, no MCP, no Postgres, no scheduler), and fires each
schedule 50 times by writing fresh `cron_expr` values. Result:
every counter ends at exactly 50. No double-counts, no drops, no
cross-talk between the 20 sub-pipelines.

So the propagator's wake semantics are correct *when there is one
trigger slot written per fire*. The production producer flow
breaks because there are **three** slot writes targeting the synth
node per fire:

1. `synth.tool_id` — seeded by the surface every invoke.
2. `synth.input` — seeded by the surface every invoke.
3. `synth.in` — written by the propagator's link-fanout from
   `tick.fire → synth.in`.

The propagator wakes the node once per distinct write that
matches a trigger slot. Each of those three slots is in synth's
trigger set (see [crates/starter-flow/src/convert.rs:108-111](../../../crates/starter-flow/src/convert.rs#L108-L111)
which adds `tool_id` + `input` to every tool-call node's triggers,
and [convert.rs:130-138](../../../crates/starter-flow/src/convert.rs#L130-L138)
which adds every link-destination slot). Three writes ⇒ three
wakes ⇒ three synth invocations per cron tick. That matches the
14-emit / 6-run-id count in the prior session's log analysis.

A note on the trigger-set double duty: the propagator at
[crates/starter-flow/src/propagator.rs:538-545](../../../crates/starter-flow/src/propagator.rs#L538-L545)
builds the node's input `SlotMap` by reading **every** slot listed
in `triggers` from the store before invoking the body. So
"triggers" is both the wake set and the input-read set. We cannot
just drop `tool_id` / `input` from the trigger set — the node body
reads them. They have to stay readable; what has to change is the
adapter writing them only when their value actually changed
between invokes.

## What the engine-side change in this session did

`GraphStore::write_slot_batch` (new SPI method,
[crates/starter-flow-spi/src/graph.rs:37-78](../../../crates/starter-flow-spi/src/graph.rs#L37-L78))
+ in-memory override
([crates/starter-flow/src/graph.rs:192-283](../../../crates/starter-flow/src/graph.rs#L192-L283))
coalesces *multi-slot writes to the same node* within one batch
into a single `SlotChanged` event. The run coordinator at
[crates/starter-flow/src/run.rs:854-871](../../../crates/starter-flow/src/run.rs#L854-L871)
now writes the seeds for a new run as one batch.

That is real progress — for a flow whose seeds populate three
slots of one node, the engine now wakes that node exactly once
during seed delivery. The three new tests in
`crates/starter-flow/src/graph.rs::tests::write_slot_batch_*`
nail down the contract (coalesce per node, honour replay, honour
idempotent short-circuit).

But because the seeds for a *new* `FlowAsTool::invoke` are a new
run (a fresh `InMemoryRunStore` per invoke; see
[crates/starter-flow-surfaces/src/lib.rs:170-181](../../../crates/starter-flow-surfaces/src/lib.rs#L170-L181)),
every invoke writes synth's `tool_id` + `input` for the first
time **in that run**. The R3 idempotent-write short-circuit can't
help across invokes. So even with the seed batch, every cron tick
delivers fresh writes that legitimately wake synth — and then the
upstream link delivery wakes it again.

In other words: the engine fix is a real improvement (and stays),
but it does not by itself fix stage 01's multi-fire.

## The actual fix (one place, ~30 lines)

[rubix-agent/src/boot/mcp/register.rs:387-485](../../crates/rubix-agent/src/boot/mcp/register.rs#L387-L485)
builds a fresh `SeedAdapter` closure that returns these writes per
invoke:

1. `seed_slot = root.payload` — the JSON envelope (locale, prefs,
   primary tool, skill body, nonce). Needed every invoke because
   the AI-agent body reads `payload` and the nonce defeats
   intra-run dedup.
2. `root.cron_expr` (when root is `trigger.schedule`) — the cron
   string. Constant per registration.
3. For each tool-call node, two slot writes: `tool_id` (a
   constant string from YAML) and `input` (the YAML
   `tool_input` JSON, optionally with `tick_epoch_ms`
   auto-injected from wall clock).

Of those, only items that genuinely change between invokes need
to be live writes:

- `tool_id` is YAML config. **Never changes across invokes for a
  given flow registration.** Should be written **once** at
  registration time, not per invoke.
- `input` is YAML config + auto-injected `tick_epoch_ms`. The
  YAML part never changes; `tick_epoch_ms` *does* change every
  invoke. But it's only meaningful for the cron-driven case
  (it's the wall-clock at fire). For everything else the YAML
  body is constant.
- `payload` legitimately changes (per-invoke nonce, principal,
  enriched input) — this is the real kick.

### Recommended split

In `register_one`, after `topology` is built, perform a one-shot
config-origin write of the **constant** parts to the engine's
graph store:

- For each tool-call node: write `tool_id` and the YAML
  `tool_input` (without `tick_epoch_ms`) to the node's slots,
  with `WriteSlotOpts::config()` (origin = Definition, replay =
  false, force = false). This populates the slots *before* any
  run starts; subsequent runs that re-write the same value hit
  the R3 idempotent short-circuit and emit no event.

Then in the `SeedAdapter` closure, return only:

- `payload` to the root (kick).
- `cron_expr` to the schedule root (kick — the schedule body
  needs to read it).
- For each tool-call node: the `tick_epoch_ms`-augmented
  `tool_input` *only*. The `tool_id` is already in the store
  from the one-shot write above and re-writing the same value
  is a no-op.

That eliminates the `tool_id` write per fire, dropping synth's
per-fire wakes from 3 to 2 (the `input` write + the link
delivery). To get to 1, the `input` write needs to be either
suppressed (use a different propagator path that doesn't wake
on `input` slot changes when only `tick_epoch_ms` was injected
without other changes) or eliminated (move `tick_epoch_ms` out
of `input` into a separate slot the body reads but does not
trigger on).

The simplest clean answer is the second option: declare a
`tick_epoch_ms` slot on the tool-call node body that is
read-but-not-triggering. The propagator's "triggers" list
governs *both* read and wake today, so adding a true
read-only slot needs an SPI change (a node descriptor can
declare `read_slots` separately from `trigger_slots`). That's
a bigger lift — leave it for a follow-up.

### Minimal fix that flips stage 01 green

Just suppressing the per-fire `tool_id` re-write is enough for
stage 01's success bar 2 + the "no double-rows in stage 02"
constraint. Concretely:

1. In `register_one`, after registry insertion but before
   `FlowRegistration` is returned, write each tool-call node's
   constant `tool_id` to its slot via the engine's
   `GraphStore::write_slot` with `WriteSlotOpts::config()`.
   One call per tool-call node, at startup, once.
2. In the `SeedAdapter` closure
   ([register.rs:452-475](../../crates/rubix-agent/src/boot/mcp/register.rs#L452-L475)),
   delete the `tool_id` write from `tool_call_writes`. Keep
   only the `input` write (with auto-injected
   `tick_epoch_ms`).

This drops synth's per-fire wakes from 3 to 2. The remaining
`input` wake fires synth once; the upstream link delivery also
fires synth once = **2 fires per cron tick**, not 3.

To eliminate the second fire too: either change the engine to
distinguish read-only inputs from trigger inputs (real fix), or
have the seed adapter also stop writing `input` per fire and
instead have the node body read `tool_input` from the seeded
config slot lazily (workaround). The former is the right
long-term shape; the latter is faster.

Either way: the place to fix is the **rubix surface adapter**,
not the engine. The engine's `write_slot_batch` change in this
session is correct and stays.

## Suggested follow-up commit shape

```
rubix-agent: stop re-seeding tool-call tool_id per invoke

Register-time one-shot config-origin write per tool-call node;
SeedAdapter no longer writes tool_id per fire. Drops synth's
per-cron-tick wakes from 3 to 2 by removing the redundant slot
write. The remaining `input` + link-delivery wakes need a
read-vs-trigger split on the propagator's trigger set to
collapse to 1 — tracked separately.
```

After that lands, re-run the live e2e from
[2026-05-26-data-flow-01-producer-multi-fire.md](./2026-05-26-data-flow-01-producer-multi-fire.md)
and look for 1-2 emit events per run_id instead of 2-3. With
spike_prob/gap_prob still elevated the success bar should now
read:

- bars 1, 3, 4: PASS (already passed last run).
- bar 2: `emitted_total` over 5 min ≈ 2 × 3 meters × 5 fires =
  ~30 with the 60s cron. The doc's `[235, 300]` range is wrong
  for a 60s cron; reconcile it (either change cron to
  `*/5 * * * * *` and accept the 5s cron — note this still
  requires the durable scheduler's `tick_interval_seconds`
  setting to allow sub-minute claims, which today is locked at
  60 — or rewrite bar 2 to `[10, 30]` for the 60s cron).

## What I'm handing back

- Engine-side: `write_slot_batch` SPI + in-memory override + run
  coordinator change + 3 new tests in
  `crates/starter-flow/src/graph.rs`. All workspace tests pass
  except the pre-existing `workspace_dep_tree_gates` baseline
  drift (unrelated, also fails on master).
- Test that proves the engine works: 20 schedule → counter
  pipelines × 50 fires, every counter at exactly 50. See
  `crates/starter-flow-nodes/tests/twenty_schedules_drive_counters.rs`.
- The producer YAML cron was changed to `*/5 * * * * *` earlier
  this session; revert that to `0 * * * * *` if you decide to
  keep the bar 2 range fix in the doc instead.
- No commits yet.

## Pickup steps for the next session

1. Apply the "minimal fix" above to
   [register.rs:387-485](../../crates/rubix-agent/src/boot/mcp/register.rs#L387-L485)
   (one-shot `tool_id` config write at registration; drop the
   per-invoke `tool_id` slot from `tool_call_writes`).
2. Rebuild rubix-agent; re-run the live e2e per
   [01-producer.md success bar](./01-producer.md#success-bar).
3. If synth fires twice per run_id but not three times, the
   surface-side fix is done and the remaining duplication is the
   "input vs trigger" engine question above. Decide whether to
   land the propagator's read-vs-trigger split now or accept
   `emit_count = 2 × clean_count` in stage 01's bar 2.
4. Reconcile bar 2's range with whatever cron + fire-count maths
   the run actually produces.
5. Flip PROGRESS.md row 1 to ✅ only after the bar reads true on
   two consecutive cold-restart runs.

Do **not** start stage 02 in the same session.
