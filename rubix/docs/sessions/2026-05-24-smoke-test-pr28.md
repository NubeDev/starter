# Session — 2026-05-24: PR #28 smoke test results

> **Tier:** session note. Lifetime: days. Per
> [HOW-TO-CODE.md §0a](../../HOW-TO-CODE.md) and
> [NEW-SESSION.md §2](../../NEW-SESSION.md), **source code must
> never reference this file.**

Ran the six-step thin-slice demo end-to-end after
`codeless/rubix-demo-wiring` landed as commit `8d84235`.

---

## Summary

| Step | Verdict |
|------|---------|
| 1. Boot (`mani run demo`) | **FAIL** |
| 2. Login | **PASS** (with workaround password) |
| 3. Disk via REST (es/en) | **FAIL** |
| 4. Claude Desktop MCP | **SKIPPED** (stdio substitute partial pass) |
| 5. Audit trail (Postgres) | **PASS** |
| 6. ClickHouse history | **FAIL** |

**Verdict: the demo is 3 steps short of real.**

---

## Blocking bugs (fix before re-running)

### B1 — `mani.yaml` compose path is doubled

`rubix/mani.yaml` tasks `dev-deps`, `dev-deps-down`, and `demo`
reference `rubix/docker/docker-compose.dev.yaml`, but `mani`
executes `cmd` from the *project* directory (`rubix/`), producing:

```
open /home/user/code/rust/starter/rubix/rubix/docker/docker-compose.dev.yaml: no such file or directory
```

**Fix:** change to `docker/docker-compose.dev.yaml` (relative to
project root) in all three tasks.

**File:** `rubix/mani.yaml` lines ~40–52.

---

### B2 — Bootstrap password too short

The `bootstrap` and `bootstrap-user` tasks pass
`--password rubix-dev` (9 chars). The auth layer rejects anything
< 12 characters:

```
Error: create_admin failed: validation error: password must be at least 12 characters
```

Additionally, neither task exports `RUBIX_DSN`, causing:

```
Error: RUBIX_DSN unset; bootstrap-user requires a Postgres DSN
```

**Fix:** Either (a) change the documented password to
`rubix-dev-passwd` (13 chars) and export
`RUBIX_DSN=postgres://rubix:rubix-dev@127.0.0.1:5433/rubix`, or
(b) lower the min-password rule for the `bootstrap-user` code path
(the user is a machine-generated operator).

**File:** `rubix/mani.yaml` (tasks `bootstrap`, `bootstrap-user`,
`demo`), `rubix/README.md` §"Local demo".

---

### B3 — ClickHouse 24 rejects TTL on DateTime64(3)

Boot with `RUBIX_CH_URL` set crashes:

```
Code: 450. DB::Exception: TTL expression result column should have
DateTime or Date type, but has DateTime64(3). (BAD_TTL_EXPRESSION)
```

The rubix-owned `0002_history/up.sql` does NOT use TTL — the
failure is in the **upstream** `starter-store-clickhouse` shared
migrations (`0001_raw_events.sql`, `0002_samples.sql`,
`0003_events.sql`) which declare `TTL ts + INTERVAL …` on a
`DateTime64(3)` column.

**Two possible fixes (pick one):**
1. Cast the TTL expression: `TTL toDateTime(ts) + INTERVAL …`
2. Pin the compose image to ClickHouse 23.x which allows it.

**Files:**
- `crates/starter-store-clickhouse/migrations/0001_raw_events.sql`
  (line 32)
- `crates/starter-store-clickhouse/migrations/0002_samples.sql`
  (line 37)
- `crates/starter-store-clickhouse/migrations/0003_events.sql`
  (line 31)
- `rubix/docker/docker-compose.dev.yaml` line 41 (image tag)

---

### B4 — Disk tool 500s when CH table doesn't exist

Even with `RUBIX_CH_URL` unset, `cfg.clickhouse_url` is `Some`
(loaded from `agent.toml`), so `main.rs` wires a `ChClient` into
`DiskTool::with_history(client)`. The tool then attempts
`INSERT INTO system_disk_history …` which fails because the table
was never created (blocked by B3). The error propagates as
`Error::Internal` → HTTP 500.

**Fix:** Either:
- Unify the gate: use the same `RUBIX_CH_URL` env check for both
  migration + client wiring, or
- Make `write_history` swallow insert failures gracefully (the verb
  result is already computed; the history row is a side-effect).

**Files:**
- `rubix/crates/rubix-agent/src/main.rs` lines 51-55
- `rubix/crates/rubix-agent/src/boot/clickhouse.rs` lines 37-44
- `rubix/crates/rubix-tools/src/system/disk.rs` lines 187-195

---

## Non-blocking observations

| # | Observation | Severity |
|---|-------------|----------|
| N1 | MCP stdio `tools/call` returns a hard-coded fixture, not a live disk probe | Medium — misleading demo |
| N2 | `_meta.acceptLanguage` in MCP `tools/call` is ignored; rendered text always English | Low — locale passthrough not wired |
| N3 | Changelog records `op = custom:invoke` rather than `op = tool.invoke`; thin-slice doc says `kind = tool.invoke` (which *is* the `resource_kind`) | Cosmetic |
| N4 | `starter_auth_users` emits `dead_code` warning for `SUPER_ADMIN_TENANT` | Lint noise |

---

## What worked

- Docker compose pulls and starts Postgres 16 + ClickHouse 24
  healthy in ~25 s.
- `wait-for-deps.sh` exits 0 on first try.
- `rubix-agent` boots with Postgres-only mode, migrations applied,
  i18n_keys=26.
- Auth login returns 200 with session + CSRF cookies.
- Authz gate allows admin principal; changelog middleware records
  every tool dispatch before the handler runs.
- MCP stdio transport framing (LSP-style `Content-Length`) works
  correctly when caller conforms.

---

## Recommended fix order for next session

1. **B1** (mani path) — one-line fix per task, unblocks the
   aggregate `mani run demo` command.
2. **B2** (password + DSN) — two-line fix in mani.yaml, unblocks
   bootstrap.
3. **B3** (TTL migration) — cast to `toDateTime(ts)` in three
   files. Re-test with a fresh volume.
4. **B4** (DiskTool 500) — unify the `RUBIX_CH_URL` / config gate
   so the client is only wired when migrations succeeded.
5. Re-run the full six steps with no workarounds.
