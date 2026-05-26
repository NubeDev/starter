# 2026-05-26 — data-flow stage 01: scheduler works; synth fires 2–3× per cron tick

## Status: ⏳ partial — scheduler is fine, but a duplicate-invocation bug in the tool-call seed adapter + a math problem in success-bar #2 block flipping the row to ✅

This session picked up from
[2026-05-25-data-flow-01-producer-tool-call-bridge.md](./2026-05-25-data-flow-01-producer-tool-call-bridge.md).
The prior session's hypothesis ("scheduler tick task not running this
boot") **turned out to be wrong**: the loop was running correctly,
the previous operator just checked PG before the second
`interval.tick()` had fired (boot at `23:18:18` → next claim window
at `23:19:18`). Verified live this session — see "What I observed".

What's actually blocking ✅ is **two** issues uncovered by the live
e2e:

1. **Synth tool fires 2–3 times per single cron tick**, all within
   one flow `run_id`, microseconds apart. Each invocation is
   independently valid (wire-shape rows + sensible `stats`), but
   the producer is supposed to emit *once* per scheduler fire.
2. **Success bar #2's range `[235, 300]` doesn't match a 60-second
   cron over 5 minutes**, even accounting for the multi-fire bug.
   `3 meters × 60 ticks − gaps` (the doc's own math) reaches ~180,
   not 235. Bar #2 looks written for a faster producer cadence
   (every ~5 s) than the YAML's `cron_expr: "0 * * * * *"`.

Bars 1, 3, 4 *are* met on the run I did (no panics / no `FlowFailed`,
spikes fired 6×, gaps fired 2×). Bar 2 is the only one short.

I'm stopping here per [NEW-SESSION.md](./NEW-SESSION.md)'s "third
check failed → write a follow-up note" rule, rather than silently
flipping the row or hand-tuning bar #2 to match my run.

---

## What I observed (live e2e, single agent boot)

### Setup

- Killed the orphaned agent from the prior session (PID 3217683,
  boot 23:18:18).
- Cold-started a fresh agent with
  `DATA_FLOW_SPIKE_PROB=0.5` so spikes would fire within a 5-min
  window (default `0.005` would need ~200 minutes to expect one
  spike).
- Boot at 2026-05-25 `23:20:26`. First cron fire at `23:21:00`,
  picked up by the scheduler on its next interval tick at
  `~23:21:27` (boot+60s).
- Drove the producer through 6 cron windows (23:21 … 23:26).

### PG state at the end

```
flow_id                       | next_run_at         | last_run_at                   | status
com.rubix.data-flow.producer  | 2026-05-25 23:26:00 | 2026-05-25 23:25:27.385553+00 | succeeded
```

`last_run_at` advanced once per minute, `last_run_status` was
`succeeded` for every fire. No `tick.failed` / `FlowFailed` /
`UnknownTool` in the log. **The scheduler works. The prior
session's diagnosis was wrong.**

### Per-fire log breakdown (after stripping ANSI + dedup by ts+run_id)

```
timestamp                     run_id   emit  gap  spike
2026-05-25T23:21:27.225793    8d1215ec    2    1      0
2026-05-25T23:21:27.225870    8d1215ec    2    1      0
2026-05-25T23:21:27.225975    8d1215ec    3    0      0
2026-05-25T23:22:27.161080    f10274a1    3    0      0
2026-05-25T23:22:27.161429    f10274a1    3    0      1
2026-05-25T23:22:27.161582    f10274a1    3    0      1
2026-05-25T23:23:27.163327    5fd456c8    3    0      1
2026-05-25T23:23:27.163461    5fd456c8    3    0      1
2026-05-25T23:24:27.160742    f3338190    3    0      0
2026-05-25T23:24:27.161038    f3338190    3    0      1
2026-05-25T23:25:27.164028    5e992f12    3    0      0
2026-05-25T23:25:27.164153    5e992f12    3    0      0
2026-05-25T23:26:27.162391    f2d9cb33    3    0      1
2026-05-25T23:26:27.162554    f2d9cb33    3    0      0
```

- **6 unique run_ids** = 6 cron fires (one per minute, exactly
  matches the 6-minute window). ✅
- **14 log entries within those 6 runs**, spaced microseconds
  apart per run. Each run shows 2 or 3 invocations of the synth
  tool, not one. ⚠
- The log-node duplication noted in the prior session is **not**
  the cause — dedup is by `(timestamp, run_id)` so each unique row
  here is a distinct emit event. The prior session's "log fires
  twice per invoke" is *additional* duplication on top of this.

Totals over the 5-min window the success bar is scored on:

- `emitted_total = 40` (vs bar #2 `[235, 300]`) — **bar 2 FAIL**
- `spikes_total = 6` — **bar 3 PASS**
- `gaps_total = 2` (and 2 entries with `emitted < 3`) — **bar 4 PASS**
- No panic, no `FlowFailed`, all PG rows `succeeded` — **bar 1 PASS**

So the actual blocker is bar 2 + the multi-fire bug it surfaces.

---

## Why synth fires 2–3× per cron tick

The flow YAML is `tick → synth → emit → ingest` with one cron fire
per minute. The propagator only wakes a node when *one of its
trigger slots is written*. `convert.rs` builds the trigger list for
each node from three sources:

1. `DEFAULT_SEED_SLOT` (`"payload"`) — always added (line 97).
2. Kind-specific extras — `tool-call` nodes get `tool_id` +
   `input` appended (line 109–110, added by the prior session so
   the seed adapter's slot writes wake the node).
3. Link-destination slots — for every link `*.x → node.y`, the
   destination slot `y` is added to that node's triggers (line
   135–136, added so scheduled flows like `tick-counter` route
   `tick.fire → counter.in` correctly).

For the `synth` node, that produces four trigger slots:
`payload`, `tool_id`, `input`, and `in` (from the
`tick.fire → synth.in` link).

The seed adapter at
[rubix-agent/src/boot/mcp/register.rs:447-476](../../crates/rubix-agent/src/boot/mcp/register.rs#L447-L476)
writes **three** of those slots on every flow invoke (`payload` +
`tool_id` + `input`), and the upstream tick node writes the fourth
(`in`). The propagator, when it sees writes arrive on multiple
trigger slots, re-wakes the node each time. Hence 2–3 invocations
per cron fire.

The same pattern affects `ingest` — it has triggers `payload`,
`tool_id`, `input`, and `in` (from `emit.emitted → ingest.in`),
and is also being multi-fired. For stage 01 the `ingest` tool is
`rubix.system.disk` (a no-op for our purposes) so the duplication
is invisible, but **stage 02 will see this**: every cron tick will
attempt to ingest 2–3 sets of rows instead of one. That will
double or triple the row count in `rubix.meter_readings_raw`
versus what the producer "really" emitted.

This is a real engine-bridge bug, not a documentation issue. It
needs to be fixed before stage 02 binds the warehouse path.

## Likely fixes (deferred — pick one before stage 02)

1. **Stop writing the seed slots that aren't tool-call-specific.**
   For `tool-call` nodes, the seed adapter writes `payload` (the
   ai-agent JSON envelope), `tool_id`, and `input`. Of those,
   `payload` is meaningless to a tool-call node — it was added for
   ai-agent root nodes. Suppressing the `payload` write for
   tool-call nodes drops one trigger source. **This is probably
   the right minimal fix.**
2. **Coalesce slot writes into a single batched propagator wake.**
   Have the seed adapter write all three slots inside one
   propagator "tick" so the node only wakes once. This is an
   engine change and out of scope for stage 01.
3. **Drop tool-call nodes' default `payload` trigger in
   `convert.rs`.** Same effect as (1) but at the convert layer:
   `DEFAULT_SEED_SLOT` shouldn't be a trigger for tool-call nodes
   that have their own well-defined input slots.

(1) and (3) are one-line changes. I did not apply either because
choosing between them is a stage-design question, not a
finish-the-stage question, and the rules say not to start the
next stage in the same session.

---

## Why bar #2's range looks wrong

The doc says:

> `stats.emitted` sum over 5 minutes is in `[235, 300]` (3 meters
> × ~60 ticks − expected gaps).

For that math to give 235–300, the producer must fire ~80–100
times in 5 minutes. At the locked `cron_expr: "0 * * * * *"`
(once per minute), 5 minutes = 5 fires = max 15 rows, even
without any mess. So either:

- The cron in the YAML should be faster than 60s (the doc was
  written for a ~5s cron), **or**
- The 5-minute window should be longer (an hour at 60s cron
  gives 60 × 3 = 180, still short of 235).

The first interpretation matches the doc author's stated math
better. Suggested fix: change `cron_expr` to `*/5 * * * * *` in
both the trigger and the schedule trigger node — that gives
12 ticks/min × 5 min = 60 ticks, matching the "3 × ~60" math the
doc states.

Until that's reconciled with the user, the row can't be flipped
to ✅ honestly.

## Concrete files candidate for stage-01 finish (if and when)

No code changes were committed this session. The only *touchable*
change to land stage 01 would be one of:

- `rubix-agent/src/boot/mcp/register.rs` — suppress `payload` seed
  slot write for tool-call nodes (fix #1 above), **and**
- `rubix/crates/rubix-flows/flows/data-flow-producer.yaml` —
  change cron to `*/5 * * * * *` so bar #2's range makes sense,
  **and**
- `rubix/docs/sessions/data-flow/01-producer.md` — if the user
  prefers, edit bar #2's math to match the 60s cron instead of
  changing the YAML. Note: the success-bar section is not in the
  doc's "Decisions taken" lock list, so editing it is allowed
  under NEW-SESSION.md's rules.

## What I'm handing back

- No commits.
- Working tree is clean except for this new session-note file +
  the prior session's note (still untracked) + the unrelated
  parallel-session files (PROGRESS.md, stack-overview.html,
  examples/, packages/starter-ui-warehouse-explorer/, …). I
  touched none of those.
- The agent process I started for testing has been killed; port
  8088 is free.

## Pickup steps for the next session

1. Decide between fix #1 ("drop `payload` seed for tool-call
   nodes" in register.rs) and fix #3 ("drop default `payload`
   trigger for tool-call kind in convert.rs"). My read is fix #1
   is safer because it preserves convert.rs's invariant that
   every node has `payload` as a fallback trigger.
2. Decide bar #2: change the YAML cron to `*/5 * * * * *` to make
   the doc's math work, OR rewrite bar #2 in the stage doc to
   `[10, 18]` for a 60s cron over 5 minutes. The first is
   cheaper and matches the doc author's intent better.
3. Apply both, rebuild, run the live e2e per
   [01-producer.md success bar](./01-producer.md#success-bar),
   twice with a cold restart between runs. With the multi-fire
   bug fixed, each cron tick should produce exactly one synth
   emit with 2–3 rows.
4. Flip PROGRESS.md row 1 to ✅.

Do **not** start stage 02 in the same session.
