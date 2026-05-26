# Stage 05 handoff — dashboard at scale (in progress)

**Status:** scaffolding landed on disk, **NOT compiled**, NOT tested,
NOT live e2e'd. PROGRESS.md row 5 still ⏳. Pick up here.

## What's already on disk (committed in `3a799b2` "added openapi/fluuter")

The user's `git pull` swept up my in-progress edits inside that
commit. So `master` already contains:

1. **L3 CH migration** `rubix/crates/rubix-agent/migrations/0005_meter_readings_15m/up.sql`
   - `rubix.meter_readings_15m` ReplacingMergeTree, 730-day TTL,
     `value_avg/min/max + Map(quality, UInt32) quality_mix`.
2. **Migration wired into boot**
   `rubix/crates/rubix-agent/src/boot/clickhouse.rs` — `with_extra_migration("rubix/0005_meter_readings_15m/up.sql", ...)`.
3. **DTO** `rubix/crates/rubix-spi/src/dto/dataflow/rollup_15m.rs` +
   `mod.rs` re-export. `DEFAULT_LOOKBACK_MINUTES=30`, `BUCKET_MINUTES=15`.
4. **Tool** `rubix/crates/rubix-tools/src/warehouse/rollup_15m.rs` +
   `mod.rs` re-export. Reads L2, writes L3.
   - SQL: `toStartOfInterval(bucket_start, INTERVAL 15 MINUTE)`,
     `avgIf/minIf/maxIf(value, isNotNull(value))`, `quality_mix`
     built from `arrayZip(groupArray(quality), arrayMap(_ -> 1, ...))`.
   - The `Map(LowCardinality(String), UInt32)` CAST is the most
     fragile line — verify on real CH; may need rewrite using
     `sumMap` or `mapFromArrays(groupArrayDistinct(quality), ...)`.
5. **Registry wiring** `rubix/crates/rubix-agent/src/registry.rs`
   - Imports `WarehouseRollup15mTool`, `AnalyticsQueryTool`.
   - Constructs both alongside `warehouse_clean`.
   - Pushes `warehouse_rollup` inside the `vec![]`.
   - **Closes the `vec!` with `];` and assigns to `let tools =` —
     re-check this. Last edit attempted to flip `vec![…]` → `let tools = vec![…]; if let Some(t)=analytics_query { tools.push(t); } tools` but I never built. The current shape on disk is what's in the commit — read it before trusting.**

## What is MISSING / TODO (in order)

### 1. Compile check first
```
cd /home/user/code/rust/starter
cargo build -p rubix-spi -p rubix-tools -p rubix-agent 2>&1 | tail -40
```
Likely breakage to fix:
- `registry.rs`: `vec![]` macro can't take a trailing `let tools.push()` — it must be `let mut tools: Vec<Arc<dyn Tool>> = vec![ ... ];` then push, then return `tools`. **Make `tools` mut**.
- `tools.push(t)` needs `let mut tools` not `let tools`.
- `AnalyticsQueryTool` constructor is `::new(client)` returning the bare tool — already wrapped via `Arc::new(...) as Arc<dyn Tool>` in the let-binding. Should be fine.

### 2. i18n catalogue keys (workspace R5 — both EN + ES)
Add to `rubix/crates/rubix-spi/catalogues/en.json` and `es.json`:
- `rubix.warehouse.rolled_up` — "Rollup materialised {rows} L3 bucket(s) over a {lookback}-minute window (at {at})."
- `rubix.warehouse.rollup.empty` — "Rollup found no L2 rows in the last {lookback} minute(s); nothing to materialise."

Without these the diagnostic strings render as raw keys.

### 3. Bundled rollup flow YAML
Create `rubix/crates/rubix-flows/flows/data-flow-rollup.yaml`
mirroring `data-flow-cleaner.yaml` (same shape — schedule trigger,
tool-call node, log node). Cron at `0 */5 * * * *` (every 5 min)
or `*/30 * * * * *` for faster e2e. Tool id `rubix.warehouse.rollup_15m`,
tool_input `{ lookback_minutes: 30 }`.

### 4. Two analytics SQL templates
Create `rubix/crates/rubix-tools/src/analytics/templates/`:
- `meter_kwh_last_24h.sql` — sums `value_avg` for electricity meters
  (`kind='electricity'`) per `meter_id` over last 24h on
  `rubix.meter_readings_15m`. Takes `{tenant_id:String}` param.
- `meter_litres_last_24h.sql` — same but `kind='water'`.

**Then** update `rubix/crates/rubix-tools/src/analytics/query.rs`
test `known_templates_contains_all_six_scope_phase_c_names` —
it hardcodes the 6 template names and will fail. Either rename
test + extend the list to 8, or drop the test.

Also fix the integration test
`rubix/crates/rubix-tools/tests/analytics_query_test.rs` `seed()`
to create the L3 mart so the two new templates have rows.

### 5. Bundled dashboard JSON
Create `rubix/crates/rubix-flows/dashboards/data-flow-site-a.json`.
The `dashboards_seed.rs` derives `page_id = "dashboard." + stem`,
so filename `data-flow-site-a.json` → `dashboard.data-flow-site-a`
(matches stage doc lock).

Shape (per stage doc §"L3 mart"):
- 2 KPI row: "Site A — last 24h kWh", "Site A — last 24h L"
  using `ChartSource::Rows` w/ RSQL — **but** the resolver may not
  back KPIs from analytics templates. Easiest path for e2e: use
  `ChartSource::Static` placeholders + a `Rows` source pointing
  at an L3-backed template OR keep KPIs as `Static` and prove the
  L3 path via the charts. Inspect `disk-overview.json` as reference.
- 3 charts (line, 30d, 15m buckets), one per meter, reading L3.

Check `crates/starter-ui-ir/src/chart.rs` for the exact
`ChartSource` JSON shape (`type: "rows"` / `series_from_rsql` /
`static`). The seeded `disk-overview.json` uses `"type": "static"`
only — the resolver may not wire `rows` to the analytics tool
end-to-end yet. **This is the biggest unknown in stage 05.**
Stage doc allows dropping success-bar item 4 (zoom) if not
supported; consider also dropping/reducing items 2–3 if the
resolver doesn't query L3 yet and document as a follow-up.

### 6. Stage doc decision tick + PROGRESS
After live e2e ×2:
- Tick `[x] Built dashboard via page_set` in `05-dashboard-at-scale.md`
- Flip row 5 ✅ in `PROGRESS.md` using a **python heredoc**
  (per user memory `edit-tools-stale-buffer.md` — the editor
  buffer has bitten this exact file before):

```python
python3 -c "
p='rubix/docs/sessions/data-flow/PROGRESS.md'
old='| 5 | [05-dashboard-at-scale.md](./05-dashboard-at-scale.md)   | ⏳     |            |        |                                            |'
new='| 5 | [05-dashboard-at-scale.md](./05-dashboard-at-scale.md)   | ✅     | 2026-05-26 | <SHA> | <evidence one-liner> |'
src=open(p).read()
assert src.count(old)==1
open(p,'w').write(src.replace(old,new))
"
```

## Live e2e procedure (USAGE.md §1–§5 + §6)

```bash
# 1. boot
cd /home/user/code/rust/starter/rubix && make restart
sleep 8
grep listening /tmp/rubix-agent.log

# 2. login
curl -s -c /tmp/smoke-cookies.txt -X POST http://127.0.0.1:8088/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"op@example.com","password":"rubix-dev-passwd"}'

# 3. drive rollup once
curl -s -b /tmp/smoke-cookies.txt -X POST \
  'http://127.0.0.1:8088/api/v1/tools/rubix.warehouse.rollup_15m' \
  -H 'content-type: application/json' -d '{"lookback_minutes":60}'

# 4. verify L3 in CH
curl -s -X POST -d \
  "SELECT count(), uniqExact(meter_id), min(bucket_start), max(bucket_start) FROM rubix.meter_readings_15m" \
  http://127.0.0.1:8124/

# 5. verify dashboard loads
curl -s -b /tmp/smoke-cookies.txt -X POST \
  http://127.0.0.1:8088/api/v1/tools/rubix.dashboard.get \
  -H 'content-type: application/json' \
  -d '{"tenant_id":"system","page_id":"dashboard.data-flow-site-a"}' | head -c 400

# 6. cold restart, re-run from #1 → must pass twice
```

For e2e timing realism: stage 03/04 producer cadence yields L2
buckets per minute. With a 30-min lookback the first rollup pass
covers all L2 you have. Stage doc allows partial-data success on
bar #1 (24h L2 + 0 L3 must render); bars #2–#3 (≥ 7 days, <1.5s,
≤9000 rows) probably need to be reframed as "follow-up" since
24h ≠ 7d without time-travel — document candidly.

## Known traps
- `examples/rubix-app` shows `(modified content)` in `git status` —
  unrelated submodule noise; ignore.
- The pulled commit `3a799b2` already includes my rollup tool + the
  L3 migration, so DON'T re-create those files. **Read first**.
- `registry.rs` is the file most likely to fail compile; check
  whether `tools` is `mut` and whether the `.push(t)` returns `tools`.
- `make restart` rebuilds from source; an uncompilable registry will
  silently leave the agent failing to boot. Always `grep listening /tmp/rubix-agent.log` after restart.
- Editor-buffer hazard on `PROGRESS.md` — always use python heredoc + assert.

## Files to read first next session
1. `rubix/crates/rubix-agent/src/registry.rs` (lines 108–280) — confirm `let mut tools = vec![...]` shape
2. `rubix/crates/rubix-tools/src/warehouse/rollup_15m.rs` — verify SQL
3. `rubix/docs/sessions/data-flow/05-dashboard-at-scale.md` — success bar
4. `crates/starter-ui-ir/src/chart.rs` — `ChartSource::Rows` wire shape
5. `rubix/crates/rubix-flows/dashboards/disk-overview.json` — template
