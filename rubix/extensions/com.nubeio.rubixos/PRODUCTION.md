# Adopting an existing Rubix-OS Timescale DB into `com.nubeio.rubixos`

How we connected `rubix-agent` to an **existing** Rubix-OS
PostgreSQL/TimescaleDB instance that already held production BMS data,
without copying any rows — by renaming the source tables in place into
the extension's `com_nubeio_rubixos__*` convention.

> **No secrets in this file.** The DB password is never written to
> config or to this doc — it lives in the age-encrypted secrets store
> and is spliced into the DSN at boot. See [Connection](#1-connection).

Target used during bring-up:

| field    | value                              |
|----------|------------------------------------|
| host     | `timescale-test.nube-iiot.com`     |
| port     | `5432`                             |
| database | `postgres`                         |
| username | `postgres`                         |
| password | *(in the secrets store, key `db:password`)* |
| engine   | PostgreSQL 14.17 + TimescaleDB 2.19.3 |

---

## Current status & the open issue

**Working and verified:**

- ✅ Rename migration applied — all 8 tables now `com_nubeio_rubixos__*`
  with a `tenant_id` column; `histories` is still a hypertable. Data
  intact: **58,291 points** (all `tenant_id='system'`), **~955M**
  history samples, data through 2026-05-30. No rows copied.
- ✅ Connection via the secrets store — password-less DSN in config,
  password spliced in from the age store at boot; proven with a live
  `SELECT 1` against the remote DB.
- ✅ `rubix-agent` boots against the remote DB, runs its 13 native
  migrations, loads the extension (`loaded=1`, 25 templates), and
  recognises the migrated table (`relation
  "com_nubeio_rubixos__histories" already exists, skipping`) instead of
  creating an empty stub.
- ✅ Auth + extension read path proven end-to-end on a separate local
  test DB (token → `warehouse_query` → template SQL).
- ✅ **Read path proven end-to-end against the _remote_ adopted DB**
  (2026-06-04): browser cookie session → `warehouse_query` → template
  SQL → real rows. Networks (`networks_overview`), catalog
  (`points_list`), `meters_list`, and the Usage templates all return
  live data.

---

## Update — 2026-06-04 (what changed this session)

Three distinct issues were found and fixed bringing the dashboard up
against the remote adopted DB. In order:

1. **Boot-blocking / stacked index builds — fixed (host).**
   The background `histories` index build was a bare
   `tokio::spawn` with no single-flight guard. Because the build runs
   for ~100 min on the 955M-row hypertable, every agent restart (and
   every `make restart`) inside that window spawned *another*
   `CREATE INDEX` that serialised behind the first on the table's
   `ShareLock` — a pile-up of redundant multi-hour backends on the
   shared DB, plus orphaned `count(*)` probes. `spawn_index_build`
   (`crate::boot::extension_tables`) now (a) takes a **session-level
   `pg_try_advisory_lock`** so a second builder logs-and-exits, and
   (b) **skips any index already building server-side** (an orphan a
   killed agent left running, which the advisory lock alone can't see).
   Verified against a real `make restart REMOTE=1` mid-build: zero
   duplicate builds. The `com_nubeio_rubixos__idx_histories` build has
   since **completed and is `indisvalid=true`**.
   - Operational scripts added:
     [`scripts/reclaim-stuck-index-builds.sql`](scripts/reclaim-stuck-index-builds.sql)
     (data-safe cancel of duplicate builds / orphan counts, keeps the
     one progressing build) and
     [`scripts/monitor-index-build.sql`](scripts/monitor-index-build.sql)
     (read-only progress view).

2. **UI showed _no data_ — fixed (host).**
   Operator (Admin) sessions bind the **super-admin tenant sentinel
   `tenant_id = '*'`** (see `starter-authz` + `bootstrap_user`), but the
   contributed-template SQL filters `WHERE tenant_id = $caller_tenant_id`
   *literally* — and all adopted rows are `tenant_id = 'system'`, so
   `= '*'` matched **zero rows**. Every dashboard read came back empty.
   Fixed generically in `crate::sdui::contributed_template`: the
   `$caller_tenant_id` placeholder now compiles to a wildcard-aware
   `CASE WHEN <bind> = '*' THEN <lhs-col> ELSE <bind> END` that reuses
   the (possibly alias-qualified) left-hand column — so a `'*'`
   super-admin sees **all** tenants and a concrete-tenant caller stays
   scoped, across every template at once.

3. **Usage page was unusably slow — fixed via the 1-minute continuous
   aggregate (`histories_1m`).**
   The `usage_*` templates aggregate the raw ~955M-row hypertable for
   the meter point-set the page selects — and the page selects **up to
   5000 points** (the `meters_list` limit). Measured raw: ~8 s exec +
   ~2.7 s planning *per* template call, several calls per render → 40 s+.
   The raw scan reads ~1.9M rows/chunk and discards >90 % because the
   `(tenant_id, "timestamp", point_uuid)` index can't seek by point.

   The fix is the continuous aggregate the extension was *designed*
   with — `com_nubeio_rubixos__histories_1m` (1-minute rollup, declared
   in `block.yaml`, installed by `scripts/post-load.sql`). It had never
   been created on the remote DB. **The earlier `usage_daily_cagg`
   experiment refused to refresh** (backend killed server-side,
   `expected to read 5 bytes…`) — but that is specific to a **1-day
   bucket**, whose per-group hash state is huge on this small box
   (~12 MB `work_mem`). The **1-minute** cagg refreshes fine because its
   many small groups fit, **as long as the refresh is run one day at a
   time** (a single 7-day refresh still OOMs). With `histories_1m`
   populated for the window, all three templates now read it
   (re-bucketing the per-minute pre-rollup up to the requested width)
   and use its `(tenant_id, point_uuid, bucket)` index for a real
   point-seek:

   ```
   usage_bucketed   (5000 pts, 7d, 1day): ~5 s
   usage_site_totals(5000 pts, 7d): ~13 s cold → ~1.1 s warm
   ```

   Re-materialise the cagg with the no-`psql` runner (per-day,
   memory-safe):
   [`examples/refresh_histories_1m.rs`](../../crates/rubix-agent/examples/refresh_histories_1m.rs).
   (A `usage_daily_cagg` variant + its runner
   [`examples/install_caggs.rs`](../../crates/rubix-agent/examples/install_caggs.rs)
   exist too, but the daily bucket is the one that won't refresh here —
   prefer `histories_1m`.)

> **No table data was ever modified.** Only runaway DDL / queries were
> cancelled, and only the (empty) continuous aggregate was dropped.

---

## Read performance — mostly fixed; residual TimescaleDB tuning tracked

**Status (2026-06-04).** The Usage page at `?range=7d` went from
unusable (40 s+) to working by routing the `usage_*` templates through
the `histories_1m` continuous aggregate (item 3 above). Current numbers
for the worst case (all 5000 elec meters, 7 days):

```
usage_bucketed    @ 1day : ~5 s
usage_site_totals        : ~13 s cold  →  ~1.1 s warm
```

Warm performance is fine; the **cold-cache first hit** is the residual
pain, and it is fundamentally an **infrastructure / TimescaleDB tuning**
matter on this box, not an app bug. The adopted DB is small:

```
shared_buffers   ≈ 496 MB      work_mem        ≈ 12 MB
effective_cache_size ≈ 1.5 GB  max_parallel_workers_per_gather = 2
PostgreSQL 14.17 + TimescaleDB 2.19.3
```

The working set for a 5000-point/7-day read doesn't fit in cache, so a
cold query pays disk; and the small `work_mem` is exactly why a 1-day
cagg refresh OOMs while a 1-minute one (run per-day) survives.

**TODO — TimescaleDB optimisations worth doing (none blocking now):**

1. **Bump server memory settings** if the box can take it:
   `work_mem` (the single biggest lever for both the aggregates and
   cagg refresh head-room), `shared_buffers`, and
   `max_parallel_workers_per_gather`. These are server-level and were
   **not** changed during bring-up — they need an operator with DB
   admin + a restart window. Even `work_mem` to 64–128 MB would let the
   daily cagg refresh and shrink cold aggregate spills.
2. **Keep the `histories_1m` cagg refreshed going forward.** The hourly
   refresh policy was intentionally **not** installed (a standing
   1-minute background job on a shared prod DB needs explicit sign-off).
   Either install it (`add_continuous_aggregate_policy`, see
   `scripts/post-load.sql`) or run
   `examples/refresh_histories_1m.rs --policy` once authorised; until
   then the cagg only covers windows that were manually refreshed.
3. **Cut the candidate set.** 5000 points per render is the real
   upstream driver — no chart shows 5000 series. Capping `meters_list`
   (or pre-resolving the user's selection) so the aggregate runs over
   tens of points would make even cold reads sub-second and is the
   cheapest high-impact change. App/UI-side.
4. **Reduce 5000-element `ANY(array)` planning cost** (~0.7–2.7 s):
   pass the point-set as a `VALUES`/temp-table join or a server-side
   prepared plan instead of a giant inline array.
5. **Native compression on cold `histories` chunks** (append-mostly,
   read-heavy) to cut IO on the cold path, and confirm
   `chunk_time_interval` lines up with the common query window for good
   chunk exclusion.
6. **Wider cagg cadences** (`_5m` / `_1h` / `_1d`) layered on
   `histories_1m` for long ranges (6 m / 1 y), per `scripts/post-load.sql`'s
   note that storage cost is `unique(keys) × buckets`, so add them only
   when a panel needs them.

See [DB.md](DB.md) §5 for the cagg rationale. Bottom line: the path is
correct and fast warm; the remaining work is **DB-side tuning on a
memory-constrained host**, which needs an operator with server config
access.


**Resolved issue — slow first-boot index build:**

> **Status: resolved 2026-06-04.** The index build no longer blocks
> boot *and* no longer stacks duplicates across restarts (see the
> 2026-06-04 update above, item 1). The `histories` index has finished
> building and is valid. The *current* open issue is read latency at
> scale — see [OPEN ISSUE — 7-day reads are still too slow](#open-issue--7-day-reads-are-still-too-slow-needs-timescaledb-tuning).

On first boot the host builds the `order_by` index
`com_nubeio_rubixos__idx_histories (tenant_id, "timestamp", point_uuid)`.
For a normal fresh install the table is empty and this is instant — but
against the adopted **955M-row** hypertable it takes a long time, and
**boot blocks until it completes** (every statement also round-trips to
the remote host). The agent is otherwise healthy; it just sits in
`CREATE INDEX` before opening the HTTP listener.

> **Status: fixed in the host.** Boot no longer builds this index
> inline — see [The long-term fix](#the-long-term-fix-boot-first-index-in-the-background)
> below. The agent opens its HTTP listener immediately and builds the
> index on a background task behind it. No operator pre-build is
> needed.

> ⚠️ **Do _not_ pre-build with `CREATE INDEX CONCURRENTLY`.** Earlier
> revisions of this doc recommended it; it **does not work** here.
> `com_nubeio_rubixos__histories` is a TimescaleDB hypertable, and
> TimescaleDB 2.19.3 rejects concurrent index creation on hypertables:
>
> ```
> ERROR: hypertables do not support concurrent index creation
> ```
>
> The hypertable-legal, non-blocking equivalent is
> `WITH (timescaledb.transaction_per_chunk)` — which is exactly what
> the host's background builder uses (below).

**Two smaller gotchas hit during bring-up** (see [Gotchas](#gotchas)):
a pre-existing **local-DB dev agent on port 8088** made queries look
like they returned 0 rows (wrong agent); and timed-out `count(*)`
probes left **orphaned server-side queries** running on the shared DB.

---

## The long-term fix: boot first, index in the background

### The root cause (an architectural one, not a tuning knob)

The slow first boot was never really "the index is big." It was that
**data-sized DDL ran on the boot critical path, before the HTTP
listener opened.** At boot the host walks every extension's
`warehouse_tables[]` and, per table, issues `CREATE TABLE` **and** a
`CREATE INDEX` on the manifest's `order_by` columns
(`crate::boot::extension_tables`). For a fresh install the tables are
empty, so both are instant. Against an **adopted** table already
holding ~955M rows, the `CREATE INDEX` runs for a long time — and
because it was `await`ed inline in `create_extension_tables`, which is
called *before* the listener binds, the whole agent sat in
`CREATE INDEX` serving nothing.

Pre-building the index out of band only papered over it (and the
obvious `CONCURRENTLY` form is illegal on a hypertable anyway). The
durable fix is to stop blocking boot on it at all.

### What the host does now

`create_extension_tables` is split into two phases:

1. **Tables — synchronous.** `CREATE TABLE IF NOT EXISTS` is
   metadata-only and cheap, and the warehouse write/read path needs the
   schema to exist before the host serves traffic. So this stays on the
   boot path.

2. **Indexes — background.** Each table's `order_by` index statement is
   *collected*, not executed, during the sweep. After the sweep returns
   (and the listener opens), the host spawns one background task that
   builds them behind the live listener. The build is idempotent
   (`CREATE INDEX IF NOT EXISTS`), so a second boot is a no-op once the
   index exists.

For TimescaleDB hypertables the background statement carries
`WITH (timescaledb.transaction_per_chunk)`:

```sql
CREATE INDEX IF NOT EXISTS com_nubeio_rubixos__idx_histories
  ON com_nubeio_rubixos__histories (tenant_id, "timestamp", point_uuid)
  WITH (timescaledb.transaction_per_chunk);
```

This commits the build chunk-by-chunk instead of holding one lock
across the entire hypertable for the full (long) duration — the
hypertable-native stand-in for `CONCURRENTLY`, which TimescaleDB
refuses on hypertables. The host detects hypertables at runtime
(`timescaledb_information.hypertables`) and only appends the option
there; plain Postgres tables get an ordinary `CREATE INDEX`.

### Why this is the right shape

- **Boot time is bounded by metadata, not by row count.** A host
  adopting a billion-row table comes up in seconds, the same as a fresh
  install. `make start REMOTE=1` no longer races `wait-agent`'s 300s
  ceiling.
- **Correctness is unaffected.** Indexes are a query-plan optimisation,
  not a prerequisite for writes or reads. Queries simply use the
  existing indexes (`histories_timestamp_idx`, the pkey) until the new
  one is ready, then start using it — no wrong answers in between.
- **The shared production DB isn't locked for the whole build.**
  `transaction_per_chunk` keeps the lock window per-chunk.

### Single-flight: one builder, even across restarts (done)

The background builder *was* a bare fire-and-forget `tokio::spawn`. In
practice that bit us: because the `histories` index takes hours on the
adopted 955M-row hypertable, **every agent restart (and every watchdog
bounce) inside that window spawned another `CREATE INDEX`**. They were
idempotent (`IF NOT EXISTS`) so never *wrong*, but they stacked —
serialising behind each other on the table's `ShareLock` — and left a
pile of redundant multi-hour backends churning IO on the shared
production DB. The same restart storm also left orphaned `count(*)`
probes running server-side after their client died.

**Fixed in the host** (`spawn_index_build`,
`crate::boot::extension_tables`). The build now takes a **session-level
Postgres advisory lock** via the non-blocking `pg_try_advisory_lock`
before issuing any `CREATE INDEX`:

- A second builder that can't get the lock **logs and exits** — no
  stacked builds, no thrash.
- The lock is **session-scoped on a dedicated connection held for the
  whole build**, so it spans all the index statements and releases
  automatically when the process dies and the connection closes. A
  killed or watchdog-restarted agent therefore never leaves the lock
  stuck — the next boot reacquires it cleanly and either resumes the
  reconciliation or no-ops if the index already exists.

This makes the build safe under the **watchdog** and under **more than
one instance** against the same DB.

**Operational scripts** (under `scripts/`, applied with the
`pg_probe` / `run_sql` examples or any `psql`):

- [`reclaim-stuck-index-builds.sql`](scripts/reclaim-stuck-index-builds.sql)
  — data-safe cleanup for a DB that already has the pre-fix pile-up:
  cancels duplicate `CREATE INDEX` builds + orphaned `count(*)` probes,
  **keeps the one genuinely-progressing build**, touches no table data.
  Idempotent.
- [`monitor-index-build.sql`](scripts/monitor-index-build.sql) —
  read-only progress view (is the index present yet; live leader +
  parallel-worker backends; `pg_stat_progress_create_index`). Run it
  repeatedly to follow a long build.

### Where this can still go further (tracked, not yet done)

- **A durable, observable job.** This deployment already runs a durable
  scheduler; registering the index reconciliation as a one-shot job
  with a `pending / building / done / failed` status row would let
  `/readyz` and the dashboard show "indexes still building" instead of
  the agent looking healthy-but-unindexed, and add retry/backoff on
  transient errors. The advisory lock above is the core robustness
  fix; this is the observability layer on top.

---

## What the remote DB looked like

The Rubix-OS dump had already been loaded into `public.*` with these
8 tables (plus some `temp_*` staging leftovers we ignore):

```
public.histories          (hypertable on "timestamp", ~955M rows, extra seq_id col)
public.points             (~58k rows, 19 columns)
public.point_tags         (~118k)      public.point_meta_tags   (~360k)
public.device_tags                      public.device_meta_tags
public.network_tags                     public.network_meta_tags
```

These are an almost-exact match for the 8 tables the extension declares
in `block.yaml` (`warehouse_tables[]`). The **only two gaps** versus
what the extension's SQL templates expect:

1. **Names** — templates read `com_nubeio_rubixos__<name>` (the host's
   `com_<id>__<name>` convention).
2. **`tenant_id`** — every template filters
   `WHERE tenant_id = $caller_tenant_id`; the source tables had no
   `tenant_id` column.

Columns are referenced by name (not position), and `histories` stays a
hypertable across a rename, so a rename + add-column is enough — no
data copy.

---

## What we did

### 1. Connection (password via the secrets store)

The agent reads the DB password from an age-encrypted secrets store
(`starter-secrets-file`) instead of from plain config. The DSN in
config is **password-less**; `boot::secrets::resolve_database_url`
looks up the `database_password_secret` key and splices the password
into the DSN before any pool is opened.

Store the password once (value not shown here):

```bash
mkdir -p ~/.config/rubix/secrets
rubix-admin secrets set db:password '<DB_PASSWORD>' --path ~/.config/rubix/secrets
```

> The decryption identity is created at
> `~/.config/rubix/secrets/identity.age-key` — **back this file up**;
> losing it makes `secrets.age` unrecoverable.

Config: [`rubix/dev/agent.remote.toml`](../../dev/agent.remote.toml)

```toml
bind = "0.0.0.0:8099"   # 8099 so it can run alongside a local-DB dev agent on 8088
database_url = "postgres://postgres@timescale-test.nube-iiot.com:5432/postgres"  # no password
database_password_secret = "db:password"
secrets_path = "/home/user/.config/rubix/secrets"
```

`RUBIX_DSN` / `RUBIX_DATABASE_URL` still override this for the simple
CI/local case; the secrets path is only used when
`database_password_secret` is set.

### 2. Rename migration (no data copy)

Two idempotent, transactional SQL scripts under `scripts/`:

- [`migrate-rename-remote.up.sql`](scripts/migrate-rename-remote.up.sql)
  — per source table:
  1. `ALTER TABLE … ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'system'`
     (PG14 **fast default** — metadata only, no heap/chunk rewrite,
     safe even on the 955M-row hypertable),
  2. `ALTER TABLE public.<name> RENAME TO com_nubeio_rubixos__<name>`
     (instant, metadata only).
- [`migrate-rename-remote.down.sql`](scripts/migrate-rename-remote.down.sql)
  — reverse: drop `tenant_id`, rename back to the bare names.

Both are re-runnable: already-migrated tables are skipped, missing
source tables only warn, and the runner wraps the whole thing in one
transaction so a mid-way failure rolls back.

`tenant_id` is stamped `'system'` to match how the host scopes bundled
resources (the `bootstrap-user` admin gets `system`-tenant membership).

### 3. Boot the agent against the remote DB

`rubix-agent` booted with the remote config runs its native
migrations, loads the extension, and — because the tables already
exist — skips creating empty stubs. Then it builds the history index
(see [the open issue](#current-status--the-open-issue)).

---

## How to run it

This environment has no local `psql`, so the SQL scripts are applied
with a tiny sqlx-based runner example,
[`run_sql`](../../crates/rubix-agent/examples/run_sql.rs). Substitute a
real client (`psql -f <file>`) if you have one.

```bash
# From the workspace root: /home/user/code/rust/starter

# DSN with the password URL-encoded (@ -> %40). Not stored anywhere.
export RUBIX_PROBE_DSN='postgres://postgres:<URLENCODED_PWD>@timescale-test.nube-iiot.com:5432/postgres'

# 1. Apply the rename migration (idempotent).
cargo run -p rubix-agent --example run_sql -- \
  rubix/extensions/com.nubeio.rubixos/scripts/migrate-rename-remote.up.sql

# 2. Store the DB password in the secrets store (one time).
rubix-admin secrets set db:password '<DB_PASSWORD>' --path ~/.config/rubix/secrets

# 3. (no manual step) The agent opens its listener immediately and
#    builds the big history index on a background task — see
#    "The long-term fix". Do NOT pre-build it with CONCURRENTLY
#    (illegal on a hypertable).

# 4. Boot the agent against the remote DB (port 8099).
RUBIX_CONFIG=rubix/dev/agent.remote.toml cargo run -p rubix-agent

# 5. Create an operator (lands in tenant `system`, matching the data).
RUBIX_DSN="$RUBIX_PROBE_DSN" \
  rubix-admin bootstrap-user --email op@example.com --password '<PWD>'
```

To roll the rename back:

```bash
cargo run -p rubix-agent --example run_sql -- \
  rubix/extensions/com.nubeio.rubixos/scripts/migrate-rename-remote.down.sql
```

---

## How to test it works

### A. Data was migrated, not copied (fast checks)

```sql
-- All 8 tables now host-prefixed, each with tenant_id:
SELECT c.relname,
       EXISTS(SELECT 1 FROM information_schema.columns
              WHERE table_name=c.relname AND column_name='tenant_id') AS has_tenant
FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
WHERE n.nspname='public' AND c.relname LIKE 'com_nubeio_rubixos__%'
ORDER BY 1;

-- No bare source tables left behind:
SELECT relname FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
WHERE n.nspname='public'
  AND relname IN ('histories','points','point_tags','point_meta_tags',
                  'device_tags','device_meta_tags','network_tags','network_meta_tags');
-- → 0 rows

-- histories is still a hypertable:
SELECT hypertable_name FROM timescaledb_information.hypertables
WHERE hypertable_name = 'com_nubeio_rubixos__histories';

-- Row counts (points is cheap; use approximate_row_count for histories
-- so you don't seq-scan ~955M rows):
SELECT count(*) FROM public.com_nubeio_rubixos__points;                 -- ~58291
SELECT approximate_row_count('public.com_nubeio_rubixos__histories');   -- ~955M
SELECT DISTINCT tenant_id FROM public.com_nubeio_rubixos__points;       -- 'system'
```

Expected at bring-up: 58,291 points (all `tenant_id='system'`), ~955M
history samples, data through 2026-05-30.

### B. End-to-end through the host (auth + extension templates)

The extension never talks SQL from the browser — every read is a
`POST /api/v1/tools/com.nubeio.rubixos.warehouse_query` with
`{template, params}`, gated by an operator session that binds
`$caller_tenant_id`.

```bash
BASE=http://127.0.0.1:8099   # the remote-config agent's port

# 1. Mint a bearer token for the bootstrapped operator (tenant `system`).
TOKEN=$(curl -s -X POST $BASE/api/v1/auth/token \
  -H 'content-type: application/json' \
  -d '{"email":"op@example.com","password":"<PWD>","tenant_id":"system"}' \
  | python3 -c 'import sys,json;print(json.load(sys.stdin)["token"])')

# 2. KPI header card — expect non-zero sample_count / point_count.
curl -s -X POST $BASE/api/v1/tools/com.nubeio.rubixos.warehouse_query \
  -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"template":"com.nubeio.rubixos.histories_summary","params":{}}'

# 3. Catalog read — expect rows.
curl -s -X POST $BASE/api/v1/tools/com.nubeio.rubixos.warehouse_query \
  -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"template":"com.nubeio.rubixos.points_list","params":{"limit":5,"offset":0}}'
```

> **Template names are fully qualified** (`com.nubeio.rubixos.<name>`);
> a bare name returns "outside this extension's namespace".
> Note param names from each `kinds/*.json` schema — e.g.
> `points_search` takes `query`, not `q`.

A `403 "no caller identity (system frame)"` means the call had no valid
operator session (missing/expired token) — the template SQL is fine,
the host just refused to bind a tenant.

---

## Gotchas

**Queries return 0 rows but the SQL is correct → wrong agent.**
A local-DB dev agent (`RUBIX_CONFIG=rubix/dev/agent.toml`, DSN
`…127.0.0.1:5433/rubix`) serves on **8088** and has its own *empty*
`com_nubeio_rubixos__*` tables. The remote-data agent here runs on
**8099**. This is exactly what bit us during bring-up — the empty
results came from the local agent, not the remote one. Confirm which DB
an agent is on:

```bash
ss -tnp | grep rubix-agent      # 5432 to the remote host vs 5433 local
```

**Orphaned server-side queries on the shared DB.**
A timed-out client (e.g. a `count(*)` over the 955M-row hypertable that
exceeds your shell timeout) leaves the query **running on the server** —
the client dies, the backend doesn't. Avoid `count(*)` on `histories`
(use `approximate_row_count`). To inspect / clean up:

```sql
-- inspect
SELECT pid, state, now()-query_start AS runtime,
       left(regexp_replace(query,'\s+',' ','g'),120) AS q
FROM pg_stat_activity
WHERE state <> 'idle' AND pid <> pg_backend_pid()
ORDER BY query_start;

-- cancel one (gentle) / terminate one (hard) — only your own queries
SELECT pg_cancel_backend(<pid>);
SELECT pg_terminate_backend(<pid>);
```
