# Feature: Extension Lifecycle & the Admin / Data-Access APIs

> Verified live on 2026-06-12 against a running stack (API `127.0.0.1:4780`,
> UI `127.0.0.1:4790`), admin `admin@nexus.local`. Worked examples:
> [`com.nexus.demo`](../../../extensions/com.nexus.demo) (builtin, `main`-slot
> page), [`com.nexus.hello`](../../../extensions/com.nexus.hello) (process, panel)
> and [`com.acme.devices`](../../../extensions/com.acme.devices) (process; owns a
> nexus-DB table — WS-17).
>
> This is the **runtime / operations** companion to
> [`../EXTENSIONS.md`](../EXTENSIONS.md) (the "how do I *build* one" guide).
> Where that doc covers authoring `block.yaml`, slots and federation, this one
> covers: the **lifecycle state machine**, the **complete `/api/v1/extensions/*`
> admin surface**, what **install / enable / disable / restart / uninstall**
> actually do (and the sealed-registry "pending restart" / "uninstalled"
> semantics), and the **WS-17 data-access host methods** an extension calls from
> a node/tool body.

---

## 1. The two flavours, the one rule that governs everything

| Flavour | Has a process? | Lifecycle | Example |
|---------|----------------|-----------|---------|
| **builtin** | no | enable/disable is the only "is it on?" toggle; nothing to spawn | `com.nexus.demo` |
| **process** | yes — a supervised child binary | spawn → init handshake → health-checked → restarted per `supervision:` | `com.nexus.hello`, `com.acme.devices` |

**The one rule:** the in-memory **extension registry is sealed at boot.** It is
built once, from a scan of the bundle directories, and is immutable for the life
of the process. Everything confusing about install and uninstall follows from
this:

- **Install** persists the bundle + its DB rows but **cannot hot-mount** it into
  the sealed registry → it answers `pending_restart: true` and only goes live on
  the next boot.
- **Uninstall/purge** deletes the persisted state (DB rows, caches, owned tables)
  and the on-disk bundle, but **cannot pull the record out** of the sealed
  registry → the row lingers as `validated` until the next boot. The API marks it
  `uninstalled: true` so the UI can show it as dead/stale rather than healthy.

### Where bundles live (decides what purge can delete)

| Source | Env var | `purge` behaviour |
|--------|---------|-------------------|
| In-repo (dev, read-only) | `NEXUS_EXTENSIONS_DIR` | deletes DB rows/caches/tables; **keeps the source dir** → a restart *re-discovers* the bundle |
| Uploaded installs | `NEXUS_EXTENSIONS_INSTALLS_DIR` | also `remove_dir_all`s the bundle → a restart clears it for good |

The cleanup dry-run (`GET …/cleanup`) tells you which case you're in via
`bundle.will_delete` (`false` = dev bundle, kept).

---

## 2. The lifecycle state machine

`LifecycleState`
([`starter-ext-spi/src/lifecycle.rs`](../../../../starter-extensions/crates/starter-ext-spi/src/lifecycle.rs)):

```
Discovered ─▶ Validated ─▶ Starting ─▶ Running ─▶ Stopping ─▶ Stopped
                              │            │                      │
                              ▼            ▼                      ▼
                           Crashed ◀───────┴── (health timeout / abnormal exit)
                              │
                              ▼ (restart-intensity cap exceeded)
                           Failed   ← terminal, no auto-restart
```

| State | Meaning |
|-------|---------|
| `Discovered` | manifest on disk, not yet validated |
| `Validated` | manifest + capabilities valid; **builtin extensions sit here** (no process to run) |
| `Starting` | child spawned, init handshake in flight |
| `Running` | handshake OK, serving requests |
| `Stopping` | graceful shutdown (SIGTERM grace window) |
| `Stopped` | cleanly stopped; can re-enable |
| `Crashed` | abnormal exit / missed health ping → supervisor will restart with backoff |
| `Failed` | **terminal** — too many restarts within the window; no auto-restart |

**Two orthogonal axes** — don't conflate them:

- **`state`** (above) is the *runtime* of the process (or `Validated` for builtin).
- **`enabled`** (`enabled` / `disabled`) is the *persisted* intent — a row in the
  `extensions_enablement` table. At boot the host spawns only `enabled` records.

So "is this extension on?" = `enabled`. "Is its child alive right now?" = `state`.

---

## 3. The complete admin API surface

Kernel-mounted under `/api/v1/extensions/*`, **cookie-authed** (and **not** in
`openapi.json` — that spec covers the app routes only). All mutating `POST`/
`DELETE` calls also require the **CSRF header** `X-CSRF-Token: <csrf_token>` from
the login response.

| Route | Method | Make target | Purpose |
|-------|--------|-------------|---------|
| `/extensions` | GET | `status` | list (one flat row per extension) |
| `/extensions/overview` | GET | — | list **+ live metrics** (process, restarts, tool/rest counters) in one call |
| `/extensions/{id}` | GET | — | full detail incl. the parsed manifest |
| `/extensions/{id}/events` | GET | — | event ring snapshot, or an **SSE stream** (`?after=<seq>`) |
| `/extensions/{id}/issues` | GET | — | derived issues (crash loops, capability violations) |
| `/extensions/{id}/process` | GET | — | live child stats (pid/uptime/rss/cpu); `404 ext.process.not_running` for builtin/stopped |
| `/extensions/{id}/metrics` | GET | — | merged process gauges + counters |
| `/extensions/{id}/cleanup` | GET | `cleanup-preview` | **dry-run** of what `purge` would remove |
| `/extensions/{id}/enable` | POST | `load` | persist `enabled`; spawn if process |
| `/extensions/{id}/disable` | POST | `unload` | persist `disabled`; gracefully stop the child |
| `/extensions/{id}/restart` | POST | — | stop + respawn the child (**409 if disabled** — won't resurrect) |
| `/extensions/install` | POST (multipart) | `install` | upload a `.tar.gz` → `pending_restart: true` |
| `/extensions/{id}` | DELETE | `uninstall` | `?purge=true` runs every cleanup provider |
| `/extensions/{id}/ui/{*path}` | GET | — | serve the federation bundle (strong ETag, `304` on revalidation) — **no auth** |
| `/extensions/{id}/i18n/{lang}` | GET | — | serve an i18n catalog — **no auth** |

### Login (cookie + CSRF)

```sh
BASE=http://127.0.0.1:4780; JAR=$(mktemp)
login=$(curl -s -c "$JAR" -X POST $BASE/auth/login -H content-type:application/json \
  -d '{"email":"admin@nexus.local","password":"change-me-admin"}')
csrf=$(printf '%s' "$login" | sed -n 's/.*"csrf_token":"\([^"]*\)".*/\1/p')
# GET = -b "$JAR";  mutating = add -H "X-CSRF-Token: $csrf"
```

> **Gotcha:** the `Makefile` login uses a plain cookie jar and works for the GET
> probes, but `install`/`uninstall`/`enable`/`query` are `POST`/`DELETE` and need
> the CSRF header — without it they return **403**. (This is why
> `make install` / `make test`'s query step 403 against a stock dev server.)

### The list row (`GET /extensions`)

```json
{
  "id": "com.acme.devices",
  "version": "0.1.0",
  "display_name": "Acme Devices",
  "state": "validated",
  "runtime_kind": "process",
  "restart_count": 0,
  "capability_violations": 0,
  "enabled": "enabled",
  "restart_required": false,
  "uninstalled": false,
  "contributes": { "tools": 2, "nodes": 2, "ui": { "...": "..." } }
}
```

| Field | Read it as |
|-------|-----------|
| `state` | runtime lifecycle (above) |
| `enabled` | persisted on/off intent |
| `restart_count` / `capability_violations` | supervisor counters (process flavour) |
| `restart_required` | freshly **installed** *or* **purged** this run — the sealed registry hasn't caught up; restart to reconcile |
| `uninstalled` | **purged this run** but the record still lingers in the sealed registry — its persisted state is already gone, so the row is **dead/stale, not healthy**. The UI dims it, shows a red `uninstalled` badge, and disables its lifecycle actions. |

---

## 4. Each lifecycle action, precisely

### Install (the upload path)

```sh
curl -fsS -b "$JAR" -H "X-CSRF-Token: $csrf" -X POST \
  -F "file=@/tmp/com.nexus.demo.tar.gz;type=application/gzip" \
  $BASE/api/v1/extensions/install | jq '{id, code, pending_restart}'
# → {"id":"com.nexus.demo","code":"install.succeeded","pending_restart":true}
```

Decompress → validate the manifest → atomic-rename into `installs_dir` → persist
an `enabled` row → run the post-install hook (e.g. create owned tables now instead
of waiting for boot DDL) → mark **pending restart**. The bundle goes live on the
**next boot**. (In-repo bundles under `NEXUS_EXTENSIONS_DIR` are already scanned at
boot, so `install` there only exercises the upload path itself.)

### Enable / Disable (the persisted toggle)

- **Enable** — persist `enabled` *first* (so a spawn can't outrun its row), then
  spawn the supervisor if it's a process flavour and not already live (idempotent).
  Also **clears the `uninstalled` mark** — re-enabling brings a purged-this-run
  record back to life.
- **Disable** — pop the supervisor handle (so a racing enable can't spawn against
  a dying child), gracefully shut the child down, then persist `disabled`.

For a **builtin** there's no child, so enable/disable is purely the persisted
toggle.

### Restart (process only)

```sh
curl -fsS -b "$JAR" -H "X-CSRF-Token: $csrf" -X POST \
  $BASE/api/v1/extensions/com.nexus.hello/restart
```

Shut the live child down (SIGTERM → grace → SIGKILL **to the process group**, so
no grandchild leaks) and respawn — **preserving** the `enabled` state. Returns
**409** if the extension is currently `disabled` (restart must not resurrect a
deliberately-off extension).

The supervisor's restart policy (`supervision:` in `block.yaml`):
`restart: always | on_crash(default) | never`, exponential `backoff`
(reset after a clean handshake), and an intensity cap (`max_restarts` within
`within_seconds`) → terminal `Failed`.

### Uninstall + purge (and exactly what it clears)

Always **preview first** — the dry-run is honest about what `purge` removes:

```sh
curl -fsS -b "$JAR" $BASE/api/v1/extensions/com.nexus.demo/cleanup | jq
```
```json
{
  "id": "com.nexus.demo",
  "items": [
    { "kind": "ui_cache", "label": ".../ui/remoteEntry.js", "bytes": 120190 },
    { "kind": "warehouse_table", "label": "query-kind com.nexus.demo.echo" },
    { "kind": "warehouse_table", "label": "query-kind com.nexus.demo.ping" },
    { "kind": "warehouse_table", "label": "insight com.nexus.demo.zscore" }
  ],
  "total_bytes": 120190,
  "bundle": { "path": ".../extensions/com.nexus.demo", "will_delete": false }
}
```

Then purge:

```sh
curl -fsS -b "$JAR" -H "X-CSRF-Token: $csrf" -X DELETE \
  "$BASE/api/v1/extensions/com.nexus.demo?purge=true" \
  | jq '{id, code, removed: (.removed | length), bundle}'
# → {"id":"com.nexus.demo","code":"cleanup.succeeded","removed":4,"bundle":{"will_delete":false}}
```

Purge runs **every registered cleanup provider** (each idempotent — a second purge
is a clean no-op):

| Provider | Removes |
|----------|---------|
| `EnablementRowProvider` | the `extensions_enablement` row (deleted outright, not flipped to `disabled`) |
| UI / i18n cache | cached `remoteEntry.js` / catalogs |
| `QueryKindCleanupProvider` | the extension's `nexus_extension_query_kinds` rows |
| insight cleanup | the extension's `nexus_extension_insights` rows |
| `WarehouseTableCleanupProvider` (WS-17) | `DROP TABLE` each owned `<ext>__<name>` |

> **The critical, non-obvious truth (this is what `uninstalled` exists for).**
> After a *successful* purge, the live registry **still lists the extension as
> `validated`** and its contributed query-kinds **still dispatch** — because the
> registry is sealed at boot and cannot be mutated at runtime. Purge cleared the
> *restart-resolution sources* (the DB rows + the on-disk bundle), so the
> extension is gone on the **next boot** — not the instant purge returns. The API
> now reports `uninstalled: true` + `restart_required: true` on the lingering row
> so the UI shows it as dead/stale instead of a healthy `validated/enabled`
> extension. (Implemented in
> [`starter-ext-server/src/admin.rs`](../../../../starter-extensions/crates/starter-ext-server/src/admin.rs)
> + the list/detail/overview projections in
> [`routes.rs`](../../../../starter-extensions/crates/starter-ext-server/src/routes.rs).)
>
> For an **in-repo dev bundle** (`will_delete:false`) even a restart re-discovers
> it from the scan dir — to truly remove it, move/unset the source dir then
> restart. For an **uploaded** bundle, one restart clears it for good.

`?purge=false` (the default) is gentler: it removes the bundle dir (if uploaded)
and persists `disabled`, leaving the DB rows and owned tables intact.

---

## 5. Accessing the nexus DB & datasources — the host methods (WS-17)

Section 7 of [`EXTENSIONS.md`](../EXTENSIONS.md) covers the **human** read path
(`POST /api/v1/query` with a contributed query-kind). This section covers what an
extension calls **from inside a node/tool body** — the supervised child making
host-method calls back over stdio JSON-RPC. These are **capability-gated** and
**tenant-scoped**; nexus is **Postgres** end-to-end (no ClickHouse).

### Capability grants (`block.yaml`)

```yaml
capabilities:
  - kind: warehouse_write          # warehouse.write / .update / .delete over OWN tables
    tables: [devices]
  - kind: warehouse_read           # warehouse.query used by the read kinds
    tables: [devices]
  # - kind: datasource             # datasource.query / .execute (WS-17 Wave B)
  #   datasources: ["<uuid>", ...]
  #   allow_foreign_tables: false
```

A capability is **never broader than the caller's own grants** (WS-14 §4.3).

### The routed host methods

[`nexus-api/src/extensions/host_methods.rs`](../../../backend/crates/nexus-api/src/extensions/host_methods.rs):

| Host method | Capability | What it does |
|-------------|-----------|--------------|
| `warehouse.query` | `warehouse_read` | run a contributed query-kind's SQL, tenant-/team-scoped (`$caller_tenant_id` / `$caller_team_ids`) |
| `warehouse.write` | `warehouse_write` | INSERT/**upsert** rows into an **owned** table (tenant stamped, upsert on `order_by` PK) |
| `warehouse.update` | `warehouse_write` | UPDATE owned-table rows by key column |
| `warehouse.delete` | `warehouse_write` | DELETE owned-table rows by key column |
| `datasource.query` | `datasource` | read against a configured datasource in a `READ ONLY` txn |
| `datasource.execute` | `datasource` | write/DDL against a datasource (ownership-prefix rule + `allow_foreign_tables`) |
| `authz.check`, `dashboard.read`, `ingest.write` | (resp.) | authz probe / dashboard read / push into a running flow's source |

### Own a table in the nexus DB

Declare it; the host creates `<sanitized_ext_id>__<name>` at boot
(`com.acme.devices` → `com_acme_devices__devices`), **prepending a host-owned
`tenant_id` column** and making `PRIMARY KEY (tenant_id, order_by…)`. The
`<ext>__` prefix is the ownership boundary. Column `type`s are **Postgres types**,
verbatim.

```yaml
warehouse_tables:
  - name: devices
    columns:
      - { name: device_id,  type: text }          # tenant_id prepended by host
      - { name: barcode,    type: text }
      - { name: created_at, type: timestamptz, default: "now()" }
    order_by: [device_id]                          # PK (tenant_id, device_id) + upsert key
```

### Persist from a node body (SDK)

```rust
// device_create node — idempotent upsert on device_id (host stamps tenant_id).
let inserted = ctx.warehouse_write().insert("devices", vec![json!({
    "device_id": stable_id("dev", barcode),
    "barcode":   barcode,
    "location":  location,
})])?;        // → INSERT … ON CONFLICT (tenant_id, device_id) DO UPDATE …
```

The host: checks `devices` is in **both** the `warehouse_write` grant *and* the
`warehouse_tables[]` spec (own-table allowlist), clamps the caller's `tenant_id`
(overwriting any supplied value), validates columns, and upserts. A caller with
no tenant is a **hard deny**. `ctx.datasource(id).query/execute(...)` is the
Wave-B analogue for external datasources.

### Read it back

No new mechanism — a contributed query-kind over the owned table, run via
`warehouse.query` / the human `POST /query`, tenant- and team-scoped:

```sql
SELECT device_id, barcode, location FROM "com_acme_devices__devices"
WHERE "tenant_id" = $caller_tenant_id
  AND ("team" IS NULL OR "team" = '' OR "team" = ANY($caller_team_ids));
```

---

## 6. End-to-end probes (copy/paste)

```sh
# list with the operationally-meaningful fields
curl -s -b "$JAR" $BASE/api/v1/extensions | jq -r \
  '.[] | "\(.id)  state=\(.state) enabled=\(.enabled) uninstalled=\(.uninstalled) restarts=\(.restart_count)"'

# live child stats for a process extension (404 = builtin/stopped)
curl -s -b "$JAR" $BASE/api/v1/extensions/com.nexus.hello/process | jq

# run a contributed kind (proves federation + auth + the dispatcher's 3rd source)
curl -s -b "$JAR" -H "X-CSRF-Token: $csrf" -X POST $BASE/api/v1/query \
  -H content-type:application/json \
  -d '{"sql":"","kind":"com.nexus.demo.ping"}' | jq '.rows[0]'

# the bundled Makefile e2e probe (list → detail → ui 200+304 → run both kinds)
make -C nexus/extensions/com.nexus.demo test     # GET probes pass; POST steps need CSRF
```

---

## 7. Quick reference — confusions this doc resolves

| You see… | It means… | To fix… |
|----------|-----------|---------|
| Installed bundle, `pending_restart:true`, not in the UI live | sealed registry — can't hot-mount | restart `nexus-api` |
| Purged, but row still shows + kinds still answer | sealed registry — record lingers until boot | restart; UI now flags it `uninstalled` so it reads as dead/stale |
| Purged in-repo bundle reappears after restart | dev bundle (`will_delete:false`) is re-scanned | move/unset `NEXUS_EXTENSIONS_DIR` source, then restart |
| `make install` / a `POST` returns 403 | missing CSRF header | add `-H "X-CSRF-Token: $csrf"` |
| `restart` returns 409 | extension is `disabled` | enable it first |
| `/process` returns `ext.process.not_running` | builtin, or the child isn't up | expected for builtin; for process check `state` / events |

See also: [`../EXTENSIONS.md`](../EXTENSIONS.md) (build guide),
[`NEXUS_DB_QUERY.md`](NEXUS_DB_QUERY.md) (the human query path),
[`../reference/API_CHEATSHEET.md`](../reference/API_CHEATSHEET.md).
