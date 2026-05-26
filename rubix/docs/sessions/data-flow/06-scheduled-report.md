# Stage 06 — Scheduled analytics report

## Scope

**In:**

- Wire `rubix.analytics.report` into the production tool registry
  (absent from all earlier stages — only `rubix.analytics.query` was
  registered). Requires a `FsBlobStore` rooted at `RUBIX_BLOB_ROOT`
  (default `/tmp/rubix-blobs`), a new `blob_root` field on
  `AgentConfig`, and an ephemeral `PresignKey` at boot.
- Two new **self-contained** SQL templates (no CH params — the report
  tool runs every query with empty params by design):
  - `meter_kwh_site_a_weekly.sql` — 7-day kWh per electricity meter
    for tenant `site-a`, from `rubix.meter_readings_15m`.
  - `meter_litres_site_a_weekly.sql` — same for water/litres.
- A new bundled flow
  `com.rubix.data-flow.weekly-report` that fires Monday 08:00 UTC
  and calls `rubix.analytics.report` with format `html` and queries:
  `meter_kwh_site_a_weekly`, `meter_litres_site_a_weekly`.
- Verification that the HTML blob lands under `$RUBIX_BLOB_ROOT` with
  at least one populated `<table>` section.
- Verification that `com.rubix.data-flow.weekly-report` has a row in
  `scheduled_flows` with a future `next_run_at`.

**Out:**

- Undo round-trip for the report blob — `rubix.undo.last` and the
  `ReversibleRegistry` are not wired in the production tool registry
  in any earlier stage; landing that is a separate follow-up.
- Email / Slack delivery of the rendered artifact.
- PDF format (refused at run time with
  `rubix.analytics.report.format_unsupported` — deferred to the
  frontend export path per the design doc).
- Interactive report viewer in the SDUI dashboard — reports are file
  artefacts, not a new page kind. The operator inspects the HTML file
  directly at `$RUBIX_BLOB_ROOT/reports/<template>/<ulid>.html`.
- Wiring the existing `com.rubix.weekly-report` system flow (it uses
  system templates that have no per-tenant filter and it will grow its
  own stage).

## Why this stage is its own thing

The design doc at `rubix/docs/design/reports/README.md` defines the
full `rubix.analytics.report` pipeline. Stages 01–05 deferred it
explicitly. As of stage 05 the scheduler, the L3 mart, and the
analytics templates are all live; the only missing piece is the tool
registration and a data-flow-specific report flow.

The existing `com.rubix.weekly-report` flow also calls
`rubix.analytics.report`, but it fires against system templates
(`disk_history_weekly`, etc.) that have no meter data; stage 06
does not touch it.

## Pre-flight

- Stage 05 success bar green; `rubix.meter_readings_15m` has at least
  24 h of rows (the templates query a 7-day window so they return
  non-empty data only after the producer has run long enough — the
  report renders even on empty results but the success bar requires at
  least one data row per template).
- Auth cookies valid (`/tmp/smoke-cookies.txt` + `CSRF` exported).
- `RUBIX_BLOB_ROOT` may be unset; the agent defaults to
  `/tmp/rubix-blobs`.

## Steps

### 1. Add `blob_root` to `AgentConfig`

Add `pub blob_root: Option<String>` (env `RUBIX_BLOB_ROOT`) to
`rubix-agent/src/boot/config.rs` with `None` default.

### 2. Wire `AnalyticsReportTool` in the registry

In `rubix-agent/src/registry.rs`, alongside the `analytics_query`
block, open (or create) the blob store and push the report tool:

```rust
let analytics_report: Option<Arc<dyn Tool>> = ch.as_ref().map(|client| {
    let root = blob_root.as_deref().unwrap_or("/tmp/rubix-blobs");
    let store = Arc::new(
        FsBlobStore::open(root, PresignKey::ephemeral())
            .expect("blob store init"),
    ) as Arc<dyn BlobStore>;
    Arc::new(AnalyticsReportTool::new(client.clone(), store)) as Arc<dyn Tool>
});
```

`build_tool_registry` gains a `blob_root: Option<String>` parameter;
`main.rs` passes `cfg.blob_root.clone()`.

### 3. Add the two self-contained weekly templates

`rubix-tools/src/analytics/templates/meter_kwh_site_a_weekly.sql`
`rubix-tools/src/analytics/templates/meter_litres_site_a_weekly.sql`

Both hard-code `tenant_id = 'site-a'` so they work with the report
tool's empty-params contract.

### 4. Add the bundled flow YAML

`rubix-flows/flows/data-flow-weekly-report.yaml`:

```yaml
id: com.rubix.data-flow.weekly-report
trigger: schedule
cron_expr: "0 0 8 * * Mon"
nodes:
  - id: agent
    kind: ai-agent
    config:
      session_policy: fresh
      skill_hint: com.rubix.analytics-reporter
      cost_cap: 0.50_usd
      allowed_tools:
        - analytics.query
        - analytics.report
links: []
```

### 5. Drive the verb by hand

```bash
# boot
cd rubix && make restart && sleep 8

# confirm the verb is registered
curl -s -b /tmp/smoke-cookies.txt \
  http://127.0.0.1:8088/api/v1/tools \
  | jq '[.[].id | select(startswith("rubix.analytics"))]'

# export CSRF token
CSRF=$(curl -s -b /tmp/smoke-cookies.txt \
  http://127.0.0.1:8088/api/v1/auth/me | jq -r '.csrf_token')

# trigger a report manually
curl -s -b /tmp/smoke-cookies.txt -H "x-csrf-token: $CSRF" \
  -X POST http://127.0.0.1:8088/api/v1/tools/rubix.analytics.report \
  -H 'content-type: application/json' \
  -d '{
    "template": "data-flow-weekly",
    "queries": ["meter_kwh_site_a_weekly", "meter_litres_site_a_weekly"],
    "format": "html"
  }'
# → {"blob_id":"reports/data-flow-weekly/<ulid>.html","url":"...","byte_count":N,...}

# inspect the file
ls /tmp/rubix-blobs/reports/data-flow-weekly/
grep -o '<tr>' /tmp/rubix-blobs/reports/data-flow-weekly/*.html | wc -l

# confirm scheduler row
PGPASSWORD=rubix-dev psql -h 127.0.0.1 -p 5433 -U rubix -d rubix \
  -c "SELECT flow_id, cron_expr, next_run_at FROM scheduled_flows \
      WHERE flow_id = 'com.rubix.data-flow.weekly-report';"
```

## Success bar

Stage 06 is done when **all four** are true, verified after a cold
`make restart`, repeated twice:

1. `rubix.analytics.report` appears in `GET /api/v1/tools` (not a
   404).
2. The curl in step 5 returns a JSON object with `blob_id` and
   `byte_count > 0` (not an error body).
3. The rendered HTML file exists at
   `$RUBIX_BLOB_ROOT/reports/data-flow-weekly/<ulid>.html` and
   `grep -o '<tr>' <file> | wc -l` returns ≥ 4 (one header + at
   least one data row per template; the HTML is single-line so
   `-c` undercounts — use `-o`).
4. The PG query in step 5 returns one row for
   `com.rubix.data-flow.weekly-report` with a future `next_run_at`.

## If it fails

In order:

1. **`analytics.report` returns 404** — the tool is not in the
   registry. Check `registry.rs` push block; confirm
   `RUBIX_BLOB_ROOT` is writable (`ls -la /tmp/rubix-blobs` after
   boot — the agent creates the directory on first use).
2. **Report returns error `rubix.analytics.query.unknown_template`**
   — the new `.sql` file didn't compile in. Confirm it lives under
   `rubix-tools/src/analytics/templates/` and the binary was rebuilt
   (`make restart` triggers a rebuild).
3. **`grep -c '<tr>'` returns 0 or 1** — the templates returned zero
   rows. L3 probably has no data in the 7-day window. Drive a manual
   rollup: `curl … rubix.warehouse.rollup_15m {"lookback_minutes":60}`
   then retry.

Write follow-up notes as
`rubix/docs/sessions/data-flow/YYYY-MM-DD-data-flow-06-<topic>.md`
and stop.

## Decisions taken

- Report format: `html` (human-readable, operator inspects file
  directly).
- Blob root: `RUBIX_BLOB_ROOT` env var, default `/tmp/rubix-blobs`.
  Presign key is ephemeral (random at boot); presigned URLs expire on
  restart but the file remains on disk. A durable key
  (`RUBIX_BLOB_PRESIGN_KEY`) is a follow-up.
- Templates in the report: `meter_kwh_site_a_weekly` +
  `meter_litres_site_a_weekly` (self-contained, hard-code
  `tenant_id = 'site-a'` to satisfy the empty-params contract).
  The param-bound dashboard templates (`meter_kwh_last_24h`, etc.)
  are **not** included in the report — they would fail with an
  unbound `{tenant_id:String}` substitution error at runtime.
- `meter_value_30d_15m` excluded — it returns one row per 15-min
  bucket per meter (up to ~8640 rows); too verbose for an HTML table.
- Undo path not wired in this stage — `rubix.undo.last` and
  `ReversibleRegistry` are absent from the production registry in all
  prior stages; that consolidation is a separate follow-up.
- Flow cron: Monday 08:00 UTC (matches the system `weekly-report`
  convention).
