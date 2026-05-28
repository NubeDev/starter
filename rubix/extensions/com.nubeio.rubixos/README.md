# `com.nubeio.rubixos` — Nube-iO Rubix-OS BMS data extension

End-to-end demo extension that takes a Nube-iO **Rubix-OS** Postgres
dump (points / devices / networks / tags / histories) and surfaces it
as a live BMS dashboard inside rubix — using only the contributions
the extension SPI already allows (`warehouse_tables`,
`warehouse_templates`, `tools`, federated UI). No fork of
`rubix-agent`, no path-deps into `rubix-domain`.

| surface                              | what's contributed                                                                                                  |
|--------------------------------------|---------------------------------------------------------------------------------------------------------------------|
| `contributes.tools[]`                | `echo` (smoke), `warehouse_query` (browser-facing template proxy)                                                   |
| `contributes.warehouse_tables[]`     | 8 extension-owned tables: `histories`, `points`, `{device,network,point}_tags`, `{device,network,point}_meta_tags`  |
| `contributes.warehouse_templates[]`  | 10 named templates powering the dashboard (lists, overviews, KPIs, `time_bucket()` history aggregate)               |
| `contributes.ui[]`                   | `Main` (dashboard), `Sidebar` (compact summary), `NavTree` (BMS hierarchy links)                                    |

Host-prefixed table names (created by `boot::extension_tables` at
agent start):

```
com_nubeio_rubixos__histories          ← turned into a Timescale hypertable by scripts/load-dump.sh
com_nubeio_rubixos__points
com_nubeio_rubixos__device_tags          com_nubeio_rubixos__device_meta_tags
com_nubeio_rubixos__network_tags         com_nubeio_rubixos__network_meta_tags
com_nubeio_rubixos__point_tags           com_nubeio_rubixos__point_meta_tags
```

`tenant_id TEXT NOT NULL` is prepended on every row by the host —
the dump loader stamps it to `'system'` by default (override with
`--tenant-id`).

## Quickstart

```bash
# from rubix/extensions/com.nubeio.rubixos/
make all                        # vite build + cargo build + install + agent restart + sanity GET

# Load the upstream dump (auto-detects the rubix-postgres container)
make load-dump DUMP=/home/user/Documents/db.dump
# → drops + recreates `com_nubeio_rubixos__histories` as a Timescale hypertable,
#   pg_restores the dump into the `rubixos_import` staging schema,
#   bulk-INSERTs every table into `com_nubeio_rubixos__*` with tenant_id='system'.

# Then open http://127.0.0.1:5173/extensions/com.nubeio.rubixos in the rubix-frontend.
```

## Layout

```
com.nubeio.rubixos/
├── block.yaml                       contributions declaration
├── README.md                        this file
├── Makefile                         build / install / restart / load-dump targets
├── rubix-nubeio-rubixos-extension   installed process binary (runtime.bin)
├── process/                         Rust source for the binary
│   ├── Cargo.toml
│   ├── src/main.rs                  `echo` + `warehouse_query` handlers
│   └── tests/manifest_validates.rs  block.yaml round-trips through host loader
├── kinds/                           input/output JSON Schemas + SQL bodies
│   ├── echo*                        smoke probe
│   ├── warehouse_query*             browser-facing template proxy
│   ├── points_list.sql              catalog read
│   ├── points_search.sql            ILIKE search across name + device + network
│   ├── points_by_device.sql         children of a device
│   ├── hosts_overview.sql           per-host counts
│   ├── networks_overview.sql        per-network counts
│   ├── devices_overview.sql         per-device counts
│   ├── histories_summary.sql        KPI: sample count, point count, min/max ts
│   ├── history_recent.sql           newest N raw samples for one point
│   └── history_bucketed.sql         Timescale `time_bucket()` aggregate
├── scripts/
│   └── load-dump.sh                 pg_restore + INSERT...SELECT loader
└── ui-src/                          TS source for the UI bundle
    ├── package.json                 pnpm workspace member
    ├── vite.config.ts               externalises React, single-file ESM
    ├── remoteEntry.ts               SDK-shape factory
    ├── main.tsx                     dashboard pages (overview/hosts/networks/devices/history)
    ├── sidebar.tsx                  compact sidebar widget
    ├── nav-tree.tsx                 sidebar nav-tree (BMS hierarchy)
    ├── chart.tsx                    pure-SVG line chart
    ├── api.ts                       tool-call helpers
    └── types.ts                     row shapes for each template
```

## How the dump load works

`scripts/load-dump.sh` is **out-of-band** — the extension itself only
declares `warehouse_read` capability. The loader runs server-side and:

1. Verifies the host has booted (so `boot::extension_tables` has
   created the 8 empty `com_nubeio_rubixos__*` tables).
2. Drops + recreates `com_nubeio_rubixos__histories` as a Timescale
   **hypertable** on `"timestamp"` (chunk_time_interval = 1 day).
   Doing this on the empty table is cheap — doing it after the
   bulk INSERT would need `create_hypertable(..., migrate_data => true)`
   and rewrite the whole heap.
3. `pg_restore`s the dump into a staging schema `rubixos_import`
   (never touches `public.*` rubix-native tables). Strips the
   source's own `CREATE EXTENSION timescaledb` and `search_path`
   directives that don't apply to the staging schema.
4. `INSERT INTO public.com_nubeio_rubixos__<table> SELECT '<tenant>',
   ... FROM rubixos_import.<table>` for each table. Re-runnable —
   each table is `DELETE FROM ... WHERE tenant_id=<tenant>` first.
5. `ANALYZE` for fresh planner stats.

Pass `--drop-staging` to nuke `rubixos_import` after ingest.

## How the dashboard reads back

The UI never talks SQL. Every render is a `POST
/api/v1/tools/com.nubeio.rubixos.warehouse_query` call with
`{ template, params }`, which the extension binary forwards to
`ctx.warehouse_read().query(template, params)`. The host

- looks the template up in `TemplateRegistry` (registered from
  this extension's `block.yaml`),
- verifies its `tables[]` are inside `capabilities.warehouse_read`,
- binds `$caller_tenant_id` from the operator session,
- runs the captured `sql_file` body verbatim — per SCOPE R7 the
  SPI does **not** template the SQL at runtime.

## Why this is "long term"

- Tables follow the host's `com_<id>__<name>` convention →
  `tenant_id` stamping, grant gates, and audit-only template
  registry all work out of the box.
- The dump never lands in `public.*` directly → re-ingesting a
  fresh dump is a one-liner with no risk of clobbering rubix-native
  data.
- `histories` is a real Timescale hypertable → the
  `history_bucketed` template scales to billions of rows without a
  table scan.
- Every contribution lives in `block.yaml`, so a downstream tenant
  can disable the extension or fork it without touching
  `rubix-agent`.
