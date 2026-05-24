# 2026-05-24 — Full-stack e2e smoke test against PR #32

> **Tier:** session note. Lifetime: days. Per
> [HOW-TO-CODE.md §0a](../../HOW-TO-CODE.md) and
> [NEW-SESSION.md §2](../../NEW-SESSION.md), **source code must
> never reference this file.**

Re-run the thin-slice + goals-broadening smoke end-to-end on
`master` post-merge of
[PR #32](https://github.com/NubeDev/starter/pull/32) (commit
`c1186f9`, broadens Phase 3+4 to land goals 2, 4, 3 as **real**
flows with undo + PG flow definitions + cross-instance NOTIFY).

The previous smoke note
[`2026-05-24-smoke-test-pr30.md`](./2026-05-24-smoke-test-pr30.md)
covered Goal 5 (`com.rubix.scheduled-system-check`) only. This
session note extends the coverage to **the four real goals**
landed by #32 — system-check, user-admin, clickhouse-ruler,
flow-programmer — plus the two stub flows (dashboard-assistant,
weekly-report) which must keep returning
`Diagnostic { code: rubix.goal.not_wired }` without 500-ing or
returning `null`.

The closing session note from #32 is
[`2026-05-24-goals-2-4-3-landed.md`](./2026-05-24-goals-2-4-3-landed.md)
— it captured per-goal verification *inside* the codeless job's
worktree against testcontainers. This run verifies the same
surface from an operator-side terminal against the `mani run
demo` stack, the way a fresh contributor would experience it.

---

## Prerequisites

`master` at `c1186f9` or later; `docker` daemon running;
`mani` on PATH; `cargo` toolchain; `curl`, `jq`, `python3` for
the response asserts.

```bash
cd /home/user/code/rust/starter
git fetch origin master && git log --oneline -1
# expect: c1186f9 (or newer) Merge pull request #32 ...
```

Reset the world to known-empty so the smoke is reproducible:

```bash
mani run dev-deps-down --all
docker volume rm docker_rubix_postgres_data docker_rubix_clickhouse_data 2>/dev/null
mani run demo --all
```

The volume names are the docker-compose-default form (`docker_<service>_<volume>`),
fixed in PR #31 from the stale `rubix-dev-*` names. If `mani run demo`
aborts on the 8088 port pre-flight, kill the stale agent (the
prompt hints at `pkill -f target/debug/rubix-agent`) and re-run.

Expected boot log line (the post-#32 canary):

```
INFO rubix_agent::boot::mcp: rubix MCP surface assembled mcp_tools=6
INFO rubix-agent starting ... database_url_set=true clickhouse_url_set=true
                          migrations_skipped=false ch_migrations_skipped=false
                          i18n_keys=34 mcp_tools=6 skills=6 flows=6
INFO rubix_agent::boot::flow_notify: rubix_flows_definitions listener ready
INFO rubix_agent::health: rubix-agent listening bind=127.0.0.1:8088
```

Watch for **`i18n_keys=34`** (was 26 pre-#32; +6 user-admin +6
clickhouse-ruler +6 flow-programmer +1 `rubix.goal.not_wired` —
six entries are deltas vs the EN catalogue baseline; check the
exact count when the run starts, the digit matters as a
catalogue-integrity canary). The `flow_notify` listener line is
new in #32 — its absence means NOTIFY won't propagate
cross-instance flow deploys.

---

## Surface under test

| Step | Goal | Verb | Real / Stub | Asserts |
|---|---|---|---|---|
| 1 | — | boot / migrations / MCP surface assembly | — | mcp_tools=6, i18n_keys=34, flow_notify listener live, both PG + CH migrations applied |
| 2 | — | login + `/auth/me` | — | session + csrf cookies; admin principal returned |
| 3 | 5 | MCP `tools/call com.rubix.scheduled-system-check` (en-US + es-AR) | real | structural payload, `tool.summary.code = rubix.system.disk.*`, numeric `params.percent`/`free`, CH history row written |
| 4 | 2 | MCP `tools/call com.rubix.user-admin` "create user ada" | real | row in PG `users`, Diagnostic `code = rubix.user.created`, snapshot row in undo_snapshots; **then undo**, assert `disabled_at` set + changelog `kind=undo` |
| 5 | 4 | MCP `tools/call com.rubix.clickhouse-ruler` "set retention 30 days" | real | ALTER applied (CH `system.tables` shows new TTL), snapshot row, Diagnostic `code = rubix.clickhouse.retention.set`; **then undo**, assert prior TTL restored |
| 6 | 3 | MCP `tools/call com.rubix.flow-programmer` "duplicate scheduled-system-check as com.example.copy" | real | new revision row in `flows_definitions`, `tools/list` surfaces both; **then undo**, assert revision superseded + `tools/list` shrinks |
| 7 | 3 (cross-instance) | Insert a row in `flows_definitions` via SQL from a second psql session; assert FlowRegistry reloads within 5s | real | NOTIFY → listener → reload; `tools/list` reflects the new flow without a restart |
| 8 | 1 | MCP `tools/call com.rubix.dashboard-assistant` | stub | Diagnostic `code = rubix.goal.not_wired`, params carry `goal`+`design_doc` link to `docs/design/sdui/` |
| 9 | 6 | MCP `tools/call com.rubix.weekly-report` | stub | same shape; link to `docs/design/reports/` |
| 10 | 4 | Snapshot sweep — fire 101 retention.set ops against a throwaway CH table; assert `undo_snapshots` count caps at `RUBIX_UNDO_SNAPSHOT_CAP` (default 100) | real | bounded retention enforced; oldest row gone, newest retained |
| 11 | — | CLI parity — `cargo run -p rubix-agent --bin rubix-admin -- system disk` | real | localised render, percent_used matches REST within ~MB |
| 12 | — | i18n round-trip on a goal-2 verb (es-AR) | real | `rubix.user.created` resolves to ES catalogue text in the boot-logged i18n surface |

Each row's "Real" or "Stub" expectation must hold. **Stub flows
must not 500.** Any `null` body, `-32603 internal error`, or
500 status is a regression vs the smoke-pr30 baseline and must
be filed as a fresh B-number bug in this note.

---

## Step-by-step protocol

Reuse the cookie jar across steps so the session + csrf cookies
persist.

```bash
JAR=/tmp/rubix-smoke-jar
rm -f "$JAR"
BASE=http://127.0.0.1:8088
```

### Step 1 — Boot

Captured during the `mani run demo` watch. Record:

- Commit SHA of `master` at boot time.
- The startup line verbatim (see canary above).
- `i18n_keys=N` exact number.
- `flow_notify` listener line present?
- Both `migrations_skipped=false` AND `ch_migrations_skipped=false`.

PASS / FAIL with one-sentence justification. FAIL = a fresh
B-number bug; do not advance.

### Step 2 — Login

```bash
curl -c "$JAR" -b "$JAR" -X POST "$BASE/api/v1/auth/login" \
     -H 'content-type: application/json' \
     -d '{"email":"op@example.com","password":"rubix-dev-passwd"}' \
   | jq .                                                    # expect {"csrf_token":"..."}

curl -b "$JAR" "$BASE/api/v1/auth/me" | jq .                 # expect role:admin
CSRF=$(awk '$6 == "starter_csrf" {print $7}' "$JAR")
echo "CSRF=$CSRF"
```

### Step 3 — Goal 5 (system-check) en-US + es-AR

```bash
# tools/list canary first
curl -b "$JAR" -X POST "$BASE/api/v1/mcp" \
     -H 'content-type: application/json' \
     -H "x-csrf-token: $CSRF" \
     -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' \
   | jq '.result.tools | length'                             # expect 6

# en-US invoke
curl -b "$JAR" -X POST "$BASE/api/v1/mcp" \
     -H 'content-type: application/json' \
     -H "x-csrf-token: $CSRF" \
     -d '{"jsonrpc":"2.0","id":2,"method":"tools/call",
          "params":{"name":"com.rubix.scheduled-system-check",
                    "arguments":{},
                    "_meta":{"acceptLanguage":"en-US"}}}' \
   | jq '.result.structuredContent.tool.summary'             # expect {code:"rubix.system.disk.warn|ok|full", params:{...}}

# es-AR — same payload, different locale
# (repeat with _meta.acceptLanguage:"es-AR")

# CH history check
docker exec docker-clickhouse-1 clickhouse-client \
  --query "SELECT count() FROM system_disk_history"          # expect ≥ 2 (one per invoke)
```

Assert: numeric `params.percent.i64`, numeric `params.free.i64`,
non-empty `reply` (the AgentLoop narration canary added in PR
#31). FAIL if either locale returns `null` or `-32603`.

### Step 4 — Goal 2 (user-admin) write + undo

```bash
# Create
curl -b "$JAR" -X POST "$BASE/api/v1/mcp" \
     -H 'content-type: application/json' \
     -H "x-csrf-token: $CSRF" \
     -d '{"jsonrpc":"2.0","id":3,"method":"tools/call",
          "params":{"name":"com.rubix.user-admin",
                    "arguments":{},
                    "_meta":{"acceptLanguage":"en-US"}}}' \
   | jq '.result.structuredContent'                           # expect code = rubix.user.created (or whatever the agent picks)

# Pull the actor-id from the boot bootstrap user (op@example.com)
docker exec docker-rubix_postgres-1 psql -U rubix -d rubix \
  -c "SELECT email, disabled_at FROM users ORDER BY created_at DESC LIMIT 3;"

# Undo
curl -b "$JAR" -X POST "$BASE/api/v1/mcp" \
     -H 'content-type: application/json' \
     -H "x-csrf-token: $CSRF" \
     -d '{"jsonrpc":"2.0","id":4,"method":"tools/call",
          "params":{"name":"rubix.undo.last","arguments":{}}}' \
   | jq '.result.structuredContent.code'                      # expect rubix.user.disabled (the reverse) or rubix.undo.applied

# Verify undo
docker exec docker-rubix_postgres-1 psql -U rubix -d rubix \
  -c "SELECT email, disabled_at FROM users WHERE disabled_at IS NOT NULL ORDER BY disabled_at DESC LIMIT 1;"
```

**Caveat:** the agent picks the verb based on the prompt. The
flow YAML carries `allowed_tools` so the loop can dispatch any
of `user.create`/`disable`/`list`/`team.create`/`team.assign`/
`tenant.list`/`undo.last`. The test asserts *any* Diagnostic
with a `rubix.user.*` or `rubix.team.*` code (the agent might
choose differently than the literal "create user ada" prompt).
If the agent goes off-script (e.g. asks for clarification
instead of acting), that's a skill / cost-cap issue and a
fresh B-number — not a fatal smoke failure.

### Step 5 — Goal 4 (clickhouse-ruler) write + undo

```bash
# Read current TTL on a target table (use system_disk_history as the canary)
docker exec docker-clickhouse-1 clickhouse-client \
  --query "SELECT name, engine_full FROM system.tables WHERE database='default' AND name='system_disk_history'"

# Invoke: agent should run retention.set
curl -b "$JAR" -X POST "$BASE/api/v1/mcp" \
     -H 'content-type: application/json' \
     -H "x-csrf-token: $CSRF" \
     -d '{"jsonrpc":"2.0","id":5,"method":"tools/call",
          "params":{"name":"com.rubix.clickhouse-ruler",
                    "arguments":{},
                    "_meta":{"acceptLanguage":"en-US"}}}' \
   | jq '.result.structuredContent'

# Verify TTL changed
docker exec docker-clickhouse-1 clickhouse-client \
  --query "SELECT engine_full FROM system.tables WHERE name='system_disk_history'"

# Snapshot row inserted?
docker exec docker-rubix_postgres-1 psql -U rubix -d rubix \
  -c "SELECT resource_kind, resource_id, created_at FROM undo_snapshots
      WHERE resource_kind LIKE 'clickhouse_%' ORDER BY created_at DESC LIMIT 3;"

# Undo
curl -b "$JAR" -X POST "$BASE/api/v1/mcp" \
     -H 'content-type: application/json' -H "x-csrf-token: $CSRF" \
     -d '{"jsonrpc":"2.0","id":6,"method":"tools/call",
          "params":{"name":"rubix.undo.last","arguments":{}}}'

# Verify TTL reverted
docker exec docker-clickhouse-1 clickhouse-client \
  --query "SELECT engine_full FROM system.tables WHERE name='system_disk_history'"
```

### Step 6 — Goal 3 (flow-programmer) duplicate + undo

```bash
# Pre-count
curl -b "$JAR" -X POST "$BASE/api/v1/mcp" \
     -H 'content-type: application/json' -H "x-csrf-token: $CSRF" \
     -d '{"jsonrpc":"2.0","id":7,"method":"tools/list"}' \
   | jq '.result.tools | length'                              # expect 6

# Invoke
curl -b "$JAR" -X POST "$BASE/api/v1/mcp" \
     -H 'content-type: application/json' -H "x-csrf-token: $CSRF" \
     -d '{"jsonrpc":"2.0","id":8,"method":"tools/call",
          "params":{"name":"com.rubix.flow-programmer",
                    "arguments":{},
                    "_meta":{"acceptLanguage":"en-US"}}}'

# Post-count + new revision row
curl -b "$JAR" -X POST "$BASE/api/v1/mcp" \
     -H 'content-type: application/json' -H "x-csrf-token: $CSRF" \
     -d '{"jsonrpc":"2.0","id":9,"method":"tools/list"}' \
   | jq '.result.tools | length'                              # expect 7 if agent duplicated

docker exec docker-rubix_postgres-1 psql -U rubix -d rubix \
  -c "SELECT flow_id, revision_id, superseded_at FROM flows_definitions
      ORDER BY created_at DESC LIMIT 5;"

# Undo
curl -b "$JAR" -X POST "$BASE/api/v1/mcp" \
     -H 'content-type: application/json' -H "x-csrf-token: $CSRF" \
     -d '{"jsonrpc":"2.0","id":10,"method":"tools/call",
          "params":{"name":"rubix.undo.last","arguments":{}}}'

# tools/list shrinks back
curl -b "$JAR" -X POST "$BASE/api/v1/mcp" \
     -H 'content-type: application/json' -H "x-csrf-token: $CSRF" \
     -d '{"jsonrpc":"2.0","id":11,"method":"tools/list"}' \
   | jq '.result.tools | length'                              # expect 6 again
```

### Step 7 — Cross-instance NOTIFY canary

The post-#32 acid test: insert a `flows_definitions` row by
hand and assert the agent reloads its FlowRegistry without a
restart.

```bash
# In a second terminal, psql in directly and insert a row:
docker exec -it docker-rubix_postgres-1 psql -U rubix -d rubix
```

```sql
-- Inside psql:
INSERT INTO flows_definitions
  (id, tenant_id, flow_id, revision_id, body_yaml, created_by)
VALUES
  ('01K000NOTIFYCANARY00000000', '00000000-0000-0000-0000-000000000000',
   'com.example.notify-canary',
   '01K000NOTIFYREV0000000000',
   $$id: com.example.notify-canary
description: |
  smoke-test canary; NOTIFY triggers FlowRegistry::reload
trigger: explicit
nodes:
  - id: agent
    kind: ai-agent
    config:
      session_policy: fresh
      skill_hint: com.rubix.system-checker
      cost_cap: 0.10_usd
      allowed_tools:
        - rubix.system.disk
links: []
$$,
   (SELECT id FROM users WHERE email='op@example.com'));
```

Within 5s of the INSERT, `tools/list` should surface the new
flow id without an agent restart:

```bash
curl -b "$JAR" -X POST "$BASE/api/v1/mcp" \
     -H 'content-type: application/json' -H "x-csrf-token: $CSRF" \
     -d '{"jsonrpc":"2.0","id":12,"method":"tools/list"}' \
   | jq '.result.tools[].name | select(startswith("com.example"))'   # expect "com.example.notify-canary"
```

The agent's log should show:

```
INFO rubix_agent::boot::flow_notify: reload flow_id=com.example.notify-canary
```

Cleanup: `DELETE FROM flows_definitions WHERE flow_id='com.example.notify-canary';`
and confirm the reverse — `tools/list` shrinks back within 5s.

### Step 8 — Stub flow Goal 1 (dashboard-assistant)

```bash
curl -b "$JAR" -X POST "$BASE/api/v1/mcp" \
     -H 'content-type: application/json' -H "x-csrf-token: $CSRF" \
     -d '{"jsonrpc":"2.0","id":13,"method":"tools/call",
          "params":{"name":"com.rubix.dashboard-assistant",
                    "arguments":{},
                    "_meta":{"acceptLanguage":"en-US"}}}' \
   | jq '.result.structuredContent'
```

Expect a Diagnostic with `code = rubix.goal.not_wired`, params
carrying at minimum a `goal` field (`1`) and a `design_doc`
field (a link to `docs/design/sdui/`).

**This must not 500. It must not return `null`.** If it does
either, that's a regression on the closing-docs commit from #32
that wired the stub.

### Step 9 — Stub flow Goal 6 (weekly-report)

Identical to step 8; expect link to `docs/design/reports/`.

### Step 10 — Snapshot sweep canary

Force the `undo_snapshots` cap by running 101 retention.set ops
in a tight loop. The simplest path is the REST tool surface
rather than the MCP one (no agent-loop nondeterminism).

```bash
# Create a throwaway table first
docker exec docker-clickhouse-1 clickhouse-client \
  --query "CREATE TABLE IF NOT EXISTS default.smoke_sweep (id UInt32, at DateTime) ENGINE=MergeTree() ORDER BY id"

# Hit retention.set 101 times via REST
for i in {1..101}; do
  curl -s -b "$JAR" -X POST "$BASE/api/v1/tools/rubix.clickhouse.retention.set" \
       -H "x-csrf-token: $CSRF" -H 'content-type: application/json' \
       -d "{\"table\":\"smoke_sweep\",\"days\":$i}" > /dev/null
done

# Count snapshots for the table
docker exec docker-rubix_postgres-1 psql -U rubix -d rubix \
  -c "SELECT count(*) FROM undo_snapshots WHERE resource_id = 'smoke_sweep';"
```

Expect: ≤ 100 (the default `RUBIX_UNDO_SNAPSHOT_CAP`). If 101
or higher, the sweep didn't fire — fresh B-number bug.

Cleanup: `DROP TABLE default.smoke_sweep;` and
`DELETE FROM undo_snapshots WHERE resource_id = 'smoke_sweep';`.

### Step 11 — CLI parity

```bash
cargo run -p rubix-agent --bin rubix-admin -- system disk
```

Expect: localised render of the same `percent_used` that step
3's REST/MCP returned (within ~MB free-bytes drift for a fresh
probe).

### Step 12 — i18n round-trip on a goal-2 verb

The MessageKey-to-locale-string resolution surface. Pick one
goal-2 verb and assert it renders ES vs EN distinct prose.

```bash
# Force the agent path to dispatch user.list (read-only, idempotent)
for lang in en-US es-AR; do
  echo "=== $lang ==="
  curl -b "$JAR" -X POST "$BASE/api/v1/tools/rubix.user.list" \
       -H "x-csrf-token: $CSRF" -H "accept-language: $lang" \
       -d '{}' \
     | jq '.summary'
done
```

Expect: `code` stays `rubix.user.listed` (locale-independent on
the wire); a future MCP client renders distinct EN vs ES prose
per its own locale via `starter-i18n`. Verify the catalogue has
both entries by `grep rubix.user.listed rubix/crates/rubix-spi/catalogues/{en,es}.json`.

---

## Result table

Run executed 2026-05-24 against `master @ b731062` (14 commits
past `c1186f9`, the PR #32 merge). Stack: `mani run dev-deps`
brought up `docker-postgres-1` (5433) + `docker-clickhouse-1`
(8124); volumes wiped clean (`docker_rubix_postgres_data`,
`docker_rubix_clickhouse_data`) before the run.

| Step | What | First-run verdict |
|---|---|---|
| 1 | Boot | **FAIL** (initial) — sqlx INT8/INT4 mismatch in `flows_seed::seed_and_load` (B13). **PASS after B13 fix.** Canaries: `flows_definitions seed-and-load complete inserted=6 loaded=6`, `mcp_tools=6`, `flow_notify` listener active, PG + CH migrations applied. Note: boot logs `i18n_keys=66`, not the `34` the protocol expected — catalogue grew post-#32; canary number in the protocol is stale (file separately if exactness matters). |
| 2 | Login | **PASS** — `csrf_token` returned, `/auth/me` reports `role=admin email=op@example.com`, `tools/list` returns all 6 flows. |
| 3 | MCP scheduled-system-check (en-US + es-AR) | **PASS** — both locales return `rubix.system.disk.ok` with numeric `params.percent`/`free`, non-empty `reply`; CH `rubix.system_disk_history` count = 2 (one per invoke). Reply prose is identical EN vs ES (locale-distinct rendering tracked under step 12; protocol asserts only non-null + code + numerics). |
| 4 | MCP user-admin write + undo | **FAIL** (initial) — dispatched to the wrong flow's primary tool (every flow returned `weekly-report` Diagnostic), root cause B14. **After B14 fix: still FAIL** — dispatch now routes to the real flow but the primary tool itself doesn't exist (`primary tool "rubix.user.create" not in registry`), root cause **B15**. |
| 5 | MCP clickhouse-ruler write + undo | **FAIL** — same shape as step 4 post-B14: `primary tool "rubix.clickhouse.rule.write" not in registry` (B15). |
| 6 | MCP flow-programmer duplicate + undo | **FAIL** — same shape: `primary tool "rubix.flow_ops.deploy" not in registry` (B15). |
| 7 | Cross-instance NOTIFY canary | **FAIL** — NOTIFY listener receives the signal (`flow_notify: reload signal received flow_id=com.example.notify-canary`) but the `on_reload` callback in `main.rs` is a TODO stub that only logs; `tools/list` stays at 6 (B16). |
| 8 | Stub dashboard-assistant | **PASS** — returns `code = rubix.goal.not_wired`, `params.goal="dashboard-assistant"`, `params.design_doc="docs/design/sdui/README.md"`; not null, not 500. |
| 9 | Stub weekly-report | **PASS** — returns `code = rubix.goal.not_wired`, `params.goal="weekly-report"`, `params.design_doc="docs/design/reports/README.md"`; not null, not 500. |
| 10 | Snapshot sweep canary | **BLOCKED** on B15 — `POST /api/v1/tools/rubix.clickhouse.retention.set` returns HTTP 404 (verb is not registered in `build_tool_registry`). |
| 11 | CLI parity | **PASS** — `rubix-admin system disk` renders `"Disk usage is normal (72% used, 58454630400 free, probed at 05/24/2026, 05:08)."` matching the REST `percent_used=70..72` from step 3 (within free-bytes drift across the seconds between invokes). |
| 12 | i18n round-trip | **PASS (catalogue side)** — both `rubix.user.listed` and the three `rubix.system.disk.{ok,warn,full}` keys are present in `rubix/crates/rubix-spi/catalogues/{en,es}.json` with distinct prose. The MCP-side EN/ES `reply` parity gap observed in step 3 is the *renderer* side (the reply string is generated by the tool, not by `MessageKey::render`); file as a follow-up when the agent prose itself is meant to localise. |

**Final verdict:** 6 / 12 PASS, 4 / 12 FAIL, 1 BLOCKED, 1
(step 12) partial. Three new blockers landed against PR #32's
claim that goals 2/3/4 are "real" + cross-instance NOTIFY
works: **B14** (now fixed in-tree), **B15** (the Goal 2/3/4
tools were never wired into `build_tool_registry` — the YAMLs
reference verbs that don't exist), and **B16** (the NOTIFY
listener is a signal-only stub, never reloads the
`FlowRegistry`). Fixed in this session: **B13** + **B14**.
Still open: **B15** (must wire `rubix.user.*`, `rubix.team.*`,
`rubix.clickhouse.{rule,retention}.*`, `rubix.flow_ops.deploy`,
`rubix.undo.last` into `build_tool_registry`) and **B16** (must
plumb `FlowRegistry` + `NodeKindRegistry` + `Engine` +
`ToolRegistry` mutably into the `flow_notify` `on_reload`
callback so it can call `FlowRegistry::register` and refresh
the MCP tool list).

---

## Bugs found

(Fill in during the run. Use the format from
[`2026-05-24-smoke-test-pr30.md`](./2026-05-24-smoke-test-pr30.md):
B-number, one-paragraph reproduction, file pointers, proposed
fix shape. Carry "blocking" vs "non-blocking" classification.)

### B13 — `flows_seed::seed_and_load` decodes `SELECT 1` as `i64`, but PG returns `INT4`

**Classification:** **blocking** (step 1 boot abort — nothing
downstream can run).

**Reproduction (clean volumes):**

```bash
cd /home/user/code/rust/starter/rubix
mani run --all dev-deps-down
docker volume rm docker_rubix_postgres_data docker_rubix_clickhouse_data
mani run --all dev-deps
./scripts/wait-for-deps.sh
RUBIX_DSN=postgres://rubix:rubix-dev@127.0.0.1:5433/rubix \
  cargo run -p rubix-agent --bin rubix-admin -- bootstrap-user \
  --email op@example.com --password rubix-dev-passwd
cd /home/user/code/rust/starter
RUBIX_DSN=postgres://rubix:rubix-dev@127.0.0.1:5433/rubix \
RUBIX_CONFIG=rubix/dev/agent.toml \
RUBIX_CH_URL=http://127.0.0.1:8124 \
  cargo run -p rubix-agent
```

Boot reaches the migrations + undo-sweep ticks, then aborts:

```
INFO rubix.boot: rubix migrations applied sources=4
INFO rubix.boot: undo_snapshots sweep (boot tick) complete deleted=0 max_rows_per_resource=50 max_age_days=90
INFO rubix.boot: rubix ClickHouse migrations applied
Error: flows_seed::seed_and_load: error occurred while decoding column 0:
       mismatched types; Rust type `i64` (as SQL type `INT8`)
       is not compatible with SQL type `INT4`
```

The agent never reaches the `rubix-agent listening` line, so
the canaries from step 1 (`mcp_tools=6`, `i18n_keys=34`,
`flow_notify` listener) are all un-verifiable in this run.

**File pointers:**

- [rubix/crates/rubix-agent/src/boot/flows_seed.rs](rubix/crates/rubix-agent/src/boot/flows_seed.rs#L61-L72)
  binds `Option<i64>` to a `SELECT 1 FROM flows_definitions ...`
  probe. PostgreSQL types an integer literal as `INT4` by
  default, so sqlx refuses to decode it into `i64`.

**Proposed fix shape:**

Change the probe's column type to `i32`, or cast the literal so
PG returns `INT8`. The minimal patch (one line):

```rust
// rubix/crates/rubix-agent/src/boot/flows_seed.rs:61
-        let exists: Option<i64> = sqlx::query_scalar(
+        let exists: Option<i32> = sqlx::query_scalar(
             "SELECT 1 FROM flows_definitions
```

(Equivalent alternative: `SELECT 1::bigint`.) Add a regression
test in the same commit that boots a clean PG, exercises the
`miss → hit` seed path twice, and asserts no decode error —
the same shape as B1–B4's fix commits from the pr28 smoke.

**Applied in-tree (this session):** the `Option<i64> → Option<i32>`
one-line patch landed at
[rubix/crates/rubix-agent/src/boot/flows_seed.rs](rubix/crates/rubix-agent/src/boot/flows_seed.rs#L63).
After rebuild + clean-volume re-boot, the agent now reaches
`rubix-agent listening bind=127.0.0.1:8088` with
`flows_definitions seed-and-load complete inserted=6 loaded=6`,
`mcp_tools=6`, `flow_notify` listener active. A regression test
that boots against a fresh `flows_definitions` is **still
needed** — the patch in-tree is uncovered.

**Why this slipped past PR #32 CI:** the `goals-2-4-3` closing
session note ran inside testcontainers against a worktree that
had already seeded `flows_definitions` from a sibling fixture,
so the `SELECT 1` probe always hit the `fetch_optional → Some`
arm via the `ON CONFLICT` insert above — the `Option<i64>` decode
of an empty row set was never exercised. On the first-boot
operator path here, the table is empty after a volume wipe and
the decode fires on the very first iteration.

**Recommended next step:** open a one-commit codeless job
("fix: rubix flows_seed sqlx INT4/INT8 decode (B13)") that lands
the one-line patch + an integration test that boots against a
freshly-created `flows_definitions` table. Once merged, re-run
this smoke note end-to-end; the rest of the protocol
(steps 2–12) is currently un-evaluated.

---

### B14 — ai-agent primary-tool lookup keyed by `NodeId` collides across flows

**Classification:** **blocking** (steps 4–7 dispatched to the
wrong flow's primary tool until fixed).

**Reproduction (post-B13, before B14 fix):** call any of
`com.rubix.{user-admin,clickhouse-ruler,flow-programmer,
dashboard-assistant,scheduled-system-check}` via MCP
`tools/call`. **Every** flow returns the
`com.rubix.weekly-report` `rubix.goal.not_wired` Diagnostic
instead of dispatching its own primary tool.

**Root cause:**
[rubix/crates/rubix-agent/src/boot/mcp/register.rs](rubix/crates/rubix-agent/src/boot/mcp/register.rs#L132-L149)
built a single `HashMap<NodeId, String>` mapping the root node's
`allowed_tools[0]` to the tool name, and
[rubix/crates/rubix-agent/src/boot/mcp/agent_node.rs](rubix/crates/rubix-agent/src/boot/mcp/agent_node.rs#L103)
looked the primary tool up via `self.primary_tools.get(ctx.node)`.
Every rubix flow's root node uses the same `NodeId` (mostly
`agent`, `scheduled-system-check` uses `check`), so the map
collapses to a single entry per id — last-seeded wins. In the
seed order the bundle ships, `weekly-report` is last on the
`agent` key, so every flow except `scheduled-system-check`
dispatched the weekly-report stub.

**Applied in-tree (this session):** dropped the NodeId-keyed
map and embedded the per-flow primary tool name into the seed
payload (the seed adapter is per-flow, so the closure captures
the right tool). The node body reads `payload.primary_tool`
instead of `self.primary_tools.get(ctx.node)`. Patch spans
[rubix/crates/rubix-agent/src/boot/mcp/register.rs](rubix/crates/rubix-agent/src/boot/mcp/register.rs)
+ [rubix/crates/rubix-agent/src/boot/mcp/agent_node.rs](rubix/crates/rubix-agent/src/boot/mcp/agent_node.rs);
`cargo build -p rubix-agent --bin rubix-agent` clean; `cargo
test -p rubix-agent --lib --no-run` clean.

**Regression test still needed** — an integration test that
registers two flows whose root nodes share a `NodeId` and asserts
that each flow dispatches its own primary tool. The patch in-tree
is currently uncovered.

---

### B15 — Goal 2/3/4 flow YAMLs reference tools that aren't in `build_tool_registry`

**Classification:** **blocking** (steps 4, 5, 6, 10 cannot
pass; the goals-2-4-3 closing claim from PR #32 is an
over-claim at the operator-facing surface).

**Reproduction (post-B13 + B14 fixes, agent booted):**

```bash
JAR=/tmp/rubix-smoke32-jar; BASE=http://127.0.0.1:8088
curl -s -c "$JAR" -X POST "$BASE/api/v1/auth/login" \
  -H 'content-type: application/json' \
  -d '{"email":"op@example.com","password":"rubix-dev-passwd"}' > /dev/null
CSRF=$(awk '$6 == "starter_csrf" {print $7}' "$JAR")
for f in com.rubix.user-admin com.rubix.clickhouse-ruler com.rubix.flow-programmer; do
  echo "=== $f ==="
  curl -s -b "$JAR" -X POST "$BASE/api/v1/mcp" \
    -H 'content-type: application/json' -H "x-csrf-token: $CSRF" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"$f\",\"arguments\":{},\"_meta\":{\"acceptLanguage\":\"en-US\"}}}"
done
```

Each returns `-32603 internal error: flow run failed: node
com.rubix.agent returned backend failure: ai-agent: primary tool
"<verb>" not in registry`, for verbs `rubix.user.create`,
`rubix.clickhouse.rule.write`, `rubix.flow_ops.deploy`.

**File pointers:**

- [rubix/crates/rubix-agent/src/registry.rs](rubix/crates/rubix-agent/src/registry.rs#L28-L51)
  `build_tool_registry` only wires `rubix.system.disk`,
  `rubix.system.db`, `rubix.system.flow_errors`,
  `rubix.alert.send`, `com.rubix.dashboard-assistant`,
  `com.rubix.weekly-report`. None of the verbs the goal-2/3/4
  flows declare in `allowed_tools` exist.
- Goal 2 flow YAML at `rubix/crates/rubix-flows/flows/user-admin.yaml`
  declares `rubix.user.{create,disable,list}`,
  `rubix.team.{create,assign}`, `rubix.tenant.list`,
  `rubix.undo.last`.
- Goal 4 flow YAML at `rubix/crates/rubix-flows/flows/clickhouse-ruler.yaml`
  declares `rubix.clickhouse.rule.write` + retention verbs.
- Goal 3 flow YAML at `rubix/crates/rubix-flows/flows/flow-programmer.yaml`
  declares `rubix.flow_ops.deploy`.

REST surfaces are equally empty:
`POST /api/v1/tools/rubix.user.create`,
`POST /api/v1/tools/rubix.user.list`,
`POST /api/v1/tools/rubix.clickhouse.retention.set`,
`POST /api/v1/tools/rubix.flow_ops.deploy` all return HTTP 404,
which also blocks step 10's snapshot-sweep canary outright.

**Proposed fix shape:** PR #32 shipped the flow definitions and
the `i18n_keys=66` message catalogue but not the underlying
`rubix-tools` `Tool` implementations or their REST/MCP
registrations. The fix is a Phase D follow-up of roughly the
same size as #32 itself:

1. Implement the verbs in `rubix-tools/src/{users,teams,tenants,clickhouse,flow_ops,undo}/`
   with the `Tool::invoke` shape used by `DiskTool`, returning
   a Diagnostic on success and writing a snapshot row to
   `undo_snapshots` for the mutating verbs (the
   `rubix.undo.last` verb reverses the latest snapshot).
2. Register them in
   [rubix/crates/rubix-agent/src/registry.rs](rubix/crates/rubix-agent/src/registry.rs)
   alongside the existing entries.
3. Add integration tests that drive each goal end-to-end
   (MCP `tools/call` write → PG/CH state check → `rubix.undo.last`
   → state reverts) inside testcontainers — the same shape as
   the goal-5 system-check tests already in the workspace.

This is not in-scope for the smoke-blocker fix; it is
essentially "actually wire Phase 3+4 verbs". Recommend opening
a dedicated codeless job ("feat: wire rubix Goal 2/3/4 tool
verbs (B15)") that lands the three goal families together so
the goals-2-4-3 claim from #32 holds at the operator surface.

---

### B16 — `flow_notify` `on_reload` is a TODO stub; NOTIFY never reloads `FlowRegistry`

**Classification:** **blocking** (step 7 — the cross-instance
claim from PR #32 is not actually wired at the binary level;
the listener is observable, the reload is not).

**Reproduction (post-B13 + B14 fixes):** insert a fresh flow
row as in step 7's protocol. Within seconds, the agent log
shows `flow_notify: reload signal received flow_id=com.example.notify-canary`,
proving the LISTEN/NOTIFY plumbing works — but `tools/list`
still returns 6 entries (the original bundled six). The new
row never reaches the in-process `FlowRegistry`.

**File pointer:**
[rubix/crates/rubix-agent/src/main.rs](rubix/crates/rubix-agent/src/main.rs#L91-L109)
passes a closure that *only* `tracing::info!`s the signal and
returns `Ok(())`. The comment is explicit: "The actual
`FlowRegistry::register` reload wires in alongside the goal-3
flow-programmer verbs in a subsequent stage — for now we log
so the channel wiring is observable end-to-end." PR #32 did
not land that subsequent stage.

**Proposed fix shape:**

1. Plumb `Arc<FlowRegistry>`, `Arc<NodeKindRegistry>`,
   `Arc<Engine>`, the `seed`/`output` adapter factory, and an
   `Arc<RwLock<ToolRegistry>>` (the MCP tool registry currently
   is constructed once) into the `on_reload` closure so it can:
     a. call the existing `register_one` helper in
        [rubix/crates/rubix-agent/src/boot/mcp/register.rs](rubix/crates/rubix-agent/src/boot/mcp/register.rs)
        to register the freshly-loaded `(flow_id, revision, body)`,
     b. build the corresponding `FlowAsTool` via
        `FlowAsTool::from_registry`, and
     c. swap it into the MCP `ToolRegistry` (today
        `Arc<ToolRegistry>` is immutable — either make it
        `Arc<RwLock<…>>` or add a `ToolRegistry::insert` method
        that uses interior mutability).
2. Add a multi-instance integration test (the optional
   verification at the bottom of this note already specs the
   shape): boot two `rubix-agent` processes against the same
   PG, INSERT a `flows_definitions` row from a third
   connection, assert **both** agents' `tools/list` surfaces
   the new flow within 5s.

This is also out of scope for a smoke-blocker fix — it touches
the `ToolRegistry` mutability story — but it's mechanically
simpler than B15 and worth landing as the very next PR after
B15 so the goals-2-4-3 surface is honest end-to-end.

---

## What's still stale (post-run)

Once the run completes, scan these for staleness vs reality:

- `docs/scope/THIN-SLICE.md` "Goals lit up beyond the thin
  slice" table — confirm the 4 real / 2 stub split is accurate;
  any row mis-labelled = a fresh small docs PR.
- `docs/sessions/2026-05-24-handover-codeless-orchestration.md`
  §2 — verify the reset commands still work as written; the
  volume names should already be the docker-compose-default
  form from PR #31.
- `docs/design/agent/README.md` — present-tense description
  should mention `flow_notify` (the NOTIFY listener added in
  #32); if it doesn't, that's a docs gap to file.
- `docs/design/{user-admin,clickhouse-rules,flow-programmer,undo}/README.md`
  — confirm each is present-tense and the verb lists match
  what the smoke saw. The closing-#32 commit was supposed to
  ship these; missing or stub = a docs gap.

---

## Cross-instance NOTIFY — extended verification (optional)

For an honest multi-instance test: boot a **second** rubix-agent
on a different port against the same PG + CH:

```bash
RUBIX_BIND=127.0.0.1:8089 \
RUBIX_DSN=postgres://rubix:rubix-dev-passwd@127.0.0.1:5432/rubix \
RUBIX_CH_URL=http://127.0.0.1:8124 \
RUBIX_CONFIG=rubix/dev/agent.toml \
  cargo run -p rubix-agent
```

Then re-run step 7's INSERT and assert **both** agents (8088 and
8089) reflect the new flow in `tools/list` within 5s. Each
agent's log should show its own `flow_notify` reload line.

This is the acid test for PR #32's multi-instance claim; the
goals-2-4-3 closing session note verified it inside
testcontainers but operator-side this is the first time
two real agent processes share state via NOTIFY.

If both agents converge: PR #32 holds at multi-instance scale.
If only one reloads: NOTIFY listener is broken in one of the
boot paths, fresh B-number bug, file against `boot/flow_notify.rs`.

---

## Recommended fix order if the run finds blockers

Same shape as the pr30 smoke session note:

1. Boot-blockers first (anything that breaks step 1 — config /
   migration / wiring issues). These prevent the rest from
   running, so they get fixed before any other class.
2. MCP transport / dispatch issues (step 3 `-32603` returns,
   `null` bodies, missing `reply` canary).
3. Per-goal write-path bugs (steps 4/5/6 mutations not
   landing, snapshot rows not being inserted).
4. Undo-path bugs (step 4/5/6 reverses not restoring).
5. NOTIFY / cross-instance bugs (step 7).
6. Stub regressions (steps 8/9 returning null instead of
   `rubix.goal.not_wired`).
7. Bounded-retention bugs (step 10 sweep).
8. Ergonomics (steps 11/12).

A bug in class N blocks any class > N. Fix N before N+1.

---

## Closing — what to update if the run is clean

If 12 / 12 PASS:

- Flip `docs/scope/THIN-SLICE.md` "Goals lit up" rows to
  `**real** — verified-on-<today>` for the 4 real goals.
- Add a one-paragraph "verified-on-<today>" note at the top of
  this file referencing the result table.
- Update `docs/sessions/2026-05-24-handover-codeless-orchestration.md`
  §2 "What's running" if the demo procedure differs from the
  one this note used.
- Open a small docs PR with those three edits — a clean smoke
  is worth recording in the handover so the next operator
  doesn't have to re-derive the verification protocol.

If anything is FAIL or PARTIAL:

- Open a fresh codeless job along the lines of the pr29
  smoke-blocker pattern — one PR per bug-class, integration
  test added in the same commit as the fix.
- Do not flip THIN-SLICE rows until 12 / 12 PASS.
