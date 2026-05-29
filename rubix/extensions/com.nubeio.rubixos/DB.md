# DB & Page-Load Performance Playbook

A working brief for any future AI / engineer session that needs to
investigate why the `usage` (Energy & Water) dashboard or the
`report` page is slow, and where to push next.

This file is the source of truth — keep it updated when you change
something (an index, a SQL file, a timeout, a UI fetch pattern).

---

## 1. What the pages do, end-to-end

Two pages share the same warehouse templates and the same
`fetchTemplate(...)` helper:

| Page                                           | File                                      | Default window |
| ---------------------------------------------- | ----------------------------------------- | -------------- |
| `/extensions/com.nubeio.rubixos/usage`         | `ui-src/dashboard/page.tsx`               | 7d             |
| `/extensions/com.nubeio.rubixos/report`        | `ui-src/dashboard/report-page.tsx`        | 1y             |

Both honour `?kind=…&range=…` query params via
`ui-src/dashboard/url-state.ts` (accepts `elec|energy|water` and
range labels `24h|7d|30d|90d|6m|1y`).

### Templates each load fires

For ONE channel (the dashboard runs only the active kind; the report
runs both elec + water concurrently):

1. `com.nubeio.rubixos.meters_list`        — catalog (fast, ~50 ms)
2. `com.nubeio.rubixos.usage_site_totals`  — KPI + map
3. `com.nubeio.rubixos.usage_bucketed`     — main chart + heatmap
4. `com.nubeio.rubixos.usage_per_meter`    — top-meters leaderboard
5. `com.nubeio.rubixos.usage_site_totals`  — prior-window for delta (skipped at ≥6m)

Inside each channel queries are **chained** (await one before
starting the next) so the page paints progressively. Channels in
the report run **concurrently**.

The SQL lives in `kinds/*.sql`. The HTTP route is
`POST /api/v1/tools/com.nubeio.rubixos.warehouse_query`.

---

## 2. Where slowness can hide (layer map)

```
Browser ──► Vite proxy ──► Axum (rubix-server) ──► warehouse_query tool
                                                      │
                                                      ▼
                                                   sqlx pool
                                                      │
                                                      ▼
                                          Postgres + TimescaleDB
                                       (port 5433, db=rubix, role=rubix)
                                          table: com_nubeio_rubixos__histories
                                          rows : ~62 M
                                          tenant_id : '*'   ← multi-tenant col
```

Bottleneck candidates and how to tell them apart:

| Symptom                                              | Likely layer                           |
| ---------------------------------------------------- | -------------------------------------- |
| Direct `psql` slow                                   | DB / planner / indexes / TimescaleDB   |
| `psql` fast but HTTP slow                            | Backend (serialization, sqlx, pool)    |
| HTTP fast but browser slow                           | Frontend (parse / render / many calls) |
| First hit slow, subsequent fast                      | Cold cache — warm with VACUUM ANALYZE  |
| One request errors with HTTP 408 after ~30 s / 90 s  | `QUERY_STATEMENT_TIMEOUT_MS` (Rust)    |

---

## 3. Repro / measurement recipes

### 3a. End-to-end HTTP timing per template

A new session can paste this block to get a clean per-template
latency table for any window. Adjust `FROM`/`TO`.

```bash
COOKIE=/tmp/com.nubeio.rubixos.cookies
FROM="2024-12-06T00:00:00Z"
TO="2025-12-06T00:00:00Z"

# Build a uuid CSV for each channel.
ELEC=$(curl -sS -b $COOKIE -H 'content-type: application/json' \
  -d '{"template":"com.nubeio.rubixos.meters_list","params":{"kind":"elec","secondary_tag":"power","limit":2000}}' \
  http://127.0.0.1:8088/api/v1/tools/com.nubeio.rubixos.warehouse_query \
  | jq -r '[.rows[].uuid] | join(",")')
WATER=$(curl -sS -b $COOKIE -H 'content-type: application/json' \
  -d '{"template":"com.nubeio.rubixos.meters_list","params":{"kind":"water","secondary_tag":"reading","limit":2000}}' \
  http://127.0.0.1:8088/api/v1/tools/com.nubeio.rubixos.warehouse_query \
  | jq -r '[.rows[].uuid] | join(",")')

for tpl in usage_site_totals usage_bucketed usage_per_meter; do
  for ch in elec water; do
    U=$([ "$ch" = elec ] && echo $ELEC || echo $WATER)
    BODY="{\"template\":\"com.nubeio.rubixos.${tpl}\",\"params\":{\"point_uuids\":\"$U\",\"from\":\"$FROM\",\"to\":\"$TO\"$([ "$tpl" = usage_bucketed ] && echo ',"bucket":"1 day"')$([ "$tpl" = usage_per_meter ] && echo ',"limit":200')}}"
    T0=$(date +%s.%N)
    curl -sS --max-time 120 -b $COOKIE -H 'content-type: application/json' \
      -d "$BODY" -o /tmp/_resp \
      http://127.0.0.1:8088/api/v1/tools/com.nubeio.rubixos.warehouse_query
    T1=$(date +%s.%N)
    printf "%-22s %-6s  %6.2fs  count=%-6s bytes=%s\n" \
      "$tpl" "$ch" "$(echo "$T1 - $T0" | bc)" \
      "$(jq -r '.count' /tmp/_resp)" "$(wc -c </tmp/_resp)"
  done
done
```

Login first (if cookie expired):
```bash
curl -sS -c /tmp/com.nubeio.rubixos.cookies \
  -H 'content-type: application/json' \
  -d '{"username":"ops@nube-io.com","password":"<dev-password>"}' \
  http://127.0.0.1:8088/api/v1/auth/login
```

Known baselines (warm cache, 1 y window, May 2026):

```
usage_site_totals      elec      2.76s   count=9     bytes=1365
usage_site_totals      water     1.07s   count=9     bytes=1355
usage_bucketed         elec      8.46s   count=1069  bytes=116892
usage_bucketed         water     0.52s   count=943   bytes=102910
usage_per_meter        elec      0.99s   count=200   bytes=49928
usage_per_meter        water     0.22s   count=68    bytes=17366
```

### 3b. Direct DB timing (isolates Postgres)

```bash
PGPASSWORD=rubix-dev psql -h 127.0.0.1 -p 5433 -U rubix -d rubix \
  --pset=pager=off -c "\timing" \
  -c "EXPLAIN (ANALYZE, BUFFERS) <your select here>;"
```

Key facts about the histories table:

```sql
-- table: com_nubeio_rubixos__histories  (TimescaleDB hypertable, chunked by time)
-- columns: tenant_id text, point_uuid text, host_uuid text, value numeric, "timestamp" timestamptz
-- size   : ~62 M rows in dev
-- tenant_id used by warehouse fixture data is '*' (literal asterisk)
-- indexes:
--   (tenant_id, point_uuid, "timestamp" DESC)   ← preferred for our templates
--   ("timestamp" DESC)
--   (tenant_id, "timestamp", point_uuid)
```

The `(tenant_id, point_uuid, timestamp DESC)` index is the one our
`usage_*` SQL must hit. If `EXPLAIN` shows a Seq Scan or chooses a
different index, that's the problem — see §5.

### 3c. Inspect the SQL the templates actually run

Templates resolve `kinds/<name>.sql`. Read them straight from disk —
they are simple parameterised SQL, no Rust string-building. The
heaviest one today is `usage_per_meter.sql`.

---

## 4. Current knobs & defaults

| Knob                                                     | File / where                                                    | Value     |
| -------------------------------------------------------- | --------------------------------------------------------------- | --------- |
| `QUERY_STATEMENT_TIMEOUT_MS`                             | `crates/starter-warehouse-explorer/src/lib.rs`                  | `90_000`  |
| `EXTENSION_TOOL_REQUEST_TIMEOUT` (MCP dispatch)          | `rubix/crates/rubix-agent/src/boot/mcp/mod.rs`                  | `95s`     |
| `EXTENSION_REST_REQUEST_TIMEOUT` (REST dispatch)         | `rubix/crates/rubix-agent/src/extensions/rest_dispatcher.rs`    | `95s`     |
| Default warehouse pool size                              | `crates/starter-store-warehouse/src/*` (search `max_connections`) | tune as needed |
| Dashboard default range                                  | `ui-src/dashboard/page.tsx` (`useState(1)`)                     | 7d        |
| Report default range                                     | `ui-src/dashboard/report-page.tsx` (`useState(5)`)              | 1y        |
| Range presets                                            | `ui-src/dashboard/presets.ts` (`RANGES`)                        | 24h…1y    |
| Per-meter row cap                                        | call site in `page.tsx` / `report-page.tsx`                     | 50 / 50   |
| Series-overlay cap                                       | `presets.ts` `SERIES_TOP_N`                                     | 5         |
| Skip prior-window delta when `range.hours > 2160`        | `page.tsx` & `report-page.tsx` `prevWin` memo                   | yes       |

Backend must be restarted (`make restart` in `rubix/`) for any Rust
constant change to take effect. UI hot-reloads via Vite.

---

## 5. Optimisations to try (ordered by expected impact)

### 5.1 TimescaleDB continuous aggregates (biggest win)

`usage_bucketed @ '1 day'` over 1 y currently aggregates ~60 M raw
rows live, every call. A continuous aggregate (CAGG) refreshed
hourly would turn that into a < 100 ms scan.

Skeleton:

```sql
CREATE MATERIALIZED VIEW com_nubeio_rubixos__usage_bucketed_1d
WITH (timescaledb.continuous) AS
SELECT tenant_id,
       point_uuid,
       host_uuid,
       time_bucket('1 day', "timestamp") AS bucket,
       AVG(value)::float8  AS avg_value,
       MIN(value)::float8  AS min_value,
       MAX(value)::float8  AS max_value,
       COUNT(*)            AS sample_count
FROM   com_nubeio_rubixos__histories
GROUP  BY tenant_id, point_uuid, host_uuid, bucket;

SELECT add_continuous_aggregate_policy(
  'com_nubeio_rubixos__usage_bucketed_1d',
  start_offset      => INTERVAL '7 days',
  end_offset        => INTERVAL '1 hour',
  schedule_interval => INTERVAL '1 hour');
```

Then update `kinds/usage_bucketed.sql` to read from the CAGG when
`bucket = '1 day'`, and keep the live table for finer buckets
(`15 minutes`, `1 hour`, `6 hours`).

Same trick for `usage_per_meter` over long windows.

Ship this behind a migration in `crates/starter-store-warehouse/`
or as an extension-owned schema bump. Validate with §3a before /
after.

### 5.2 Cap the data the long ranges fetch

At 6m / 1y a daily bucket still returns up to
`hosts × days ≈ 9 × 365 = 3285` rows for `usage_bucketed`. Today
we return ~1069 because many `(host, day)` pairs have no data —
fine. But `usage_per_meter` at 1y returns up to 200 rows × 50 KB
JSON for elec. Consider:

- Reduce `limit` from 200 → 50 for elec leaderboard. UI only shows
  top 10, so the rest is dead weight.
- Drop the `MIN/MAX` columns when not rendered.

### 5.3 Backend round-trip cost

`usage_per_meter` direct `psql`: 932 ms. Via HTTP: ~1 s warm — the
HTTP layer adds only ~70 ms, so backend is not the bottleneck.
`usage_bucketed` elec: psql ~? vs HTTP 8.46 s warm. **Re-measure
both** when investigating — if psql is also slow, the work is the
DB; if psql is fast and HTTP is slow, look at sqlx fetch_all
overhead for ~1000 rows × 9 cols (numeric → JSON conversion is
known-expensive — try casting to `float8` in SQL, not Rust).

### 5.4 Reduce per-page request count

Cheap wins already in place: prior-window skip, channel
concurrency on report, kind sequencing on dashboard. Further:

- Memoise the meters_list response in `fetchTemplate` (TTL 60 s).
  Currently every channel re-fetches it on every range change.
- Combine `usage_site_totals` + the totals derivable from
  `usage_bucketed` — they're redundant when bucket is `1 day`.

### 5.5 Hard knobs

- Bump `QUERY_STATEMENT_TIMEOUT_MS` if a single legit query
  exceeds 90 s — but treat that as a smell, not a fix.
- Increase warehouse pool size if `pg_stat_activity` shows queued
  queries (`state = 'idle in transaction'` or waits).

---

## 6. How to drive a profiling session (suggested order)

1. **Reproduce.** Open the page with the slow combo (`?range=1y&kind=elec`).
   Confirm it's slow.
2. **Measure HTTP per template.** Run §3a. You now have a per-template
   latency budget.
3. **Find the worst template.** Re-run §3b in `psql` with EXPLAIN
   ANALYZE. Compare DB-side time to HTTP time:
   - `DB ≈ HTTP` → push on §5.1 / §5.2 (cut data).
   - `DB ≪ HTTP` → push on §5.3 (backend serialization).
4. **Validate the fix** with §3a again. Record before/after in this
   doc under the "Baselines" section so the next session has fresh
   numbers.
5. **Watch out** for warm-vs-cold cache. Always run a template
   twice and report the warm number; first-hit cold can be 3-5×
   slower and is fixed by `VACUUM ANALYZE` + Timescale chunk
   warmup.
6. **Browser side:** open DevTools → Performance, record a load.
   If main-thread time inside `UsageTimeSeries` / `RegionRollup`
   dominates, optimise the React layer (memo, virtualised lists).

---

## 7. Files you'll most likely touch

| Concern                       | File(s)                                                                  |
| ----------------------------- | ------------------------------------------------------------------------ |
| SQL templates                 | `kinds/*.sql`, `kinds/*_params.json`, `block.yaml`                       |
| Statement timeout / pool      | `crates/starter-warehouse-explorer/src/lib.rs`                           |
| Fetch wiring (UI)             | `ui-src/api.ts`, `ui-src/dashboard/page.tsx`, `ui-src/dashboard/report-page.tsx` |
| Range presets                 | `ui-src/dashboard/presets.ts`                                            |
| URL share state               | `ui-src/dashboard/url-state.ts`                                          |
| New migrations / CAGGs        | `crates/starter-store-warehouse/migrations/` or an extension bootstrap   |

After any SQL change, rebuild and reload the extension:

```bash
cd rubix/extensions/com.nubeio.rubixos
make install        # registers updated block.yaml + SQL with the backend
# UI rebuild only when you changed ui-src/*:
make ui-build
```

After Rust changes, restart the backend:

```bash
cd rubix && make restart
```

---

## 8. Baselines log (append here when you measure)

Format: `YYYY-MM-DD  | scenario | wall-time | notes`

- 2026-05-29 | 1y, elec, dashboard, warm | ~12 s | site_totals 2.8s + bucketed 8.5s + per_meter 1.0s
- 2026-05-29 | 1y, water, dashboard, warm | ~2 s | all three < 1.1 s each
- 2026-05-29 | 1y, report, both channels concurrent, warm | ~12 s | water finishes first, paints early
- 2026-05-29 | 1y, elec, cold | 30 s + timeout (pre-90 s bump) | fixed by raising `QUERY_STATEMENT_TIMEOUT_MS`
- 2026-05-29 | 1y, intermittent ~1/5 timeouts at 30 s | dispatcher ceiling, not Postgres | bumped `EXTENSION_TOOL_REQUEST_TIMEOUT` + `EXTENSION_REST_REQUEST_TIMEOUT` 30 s → 95 s to align with `QUERY_STATEMENT_TIMEOUT_MS`; also dropped dashboard `usage_per_meter` limit 200 → 50 (UI only renders top 10)
