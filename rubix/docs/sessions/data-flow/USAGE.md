# USAGE — how a new AI session drives the data-flow stack

You are a fresh AI session about to work on one stage in this
folder. This doc gets you from "cold repo" to "I can hit a verb
and see a row in ClickHouse" without you having to spelunk.

Read once, top to bottom. Then open
[PROGRESS.md](./PROGRESS.md) to find the next stage, then open
that stage's doc.

---

## 0. Ground rules

- **Do not start a stage until the previous stage's "Success bar"
  is green in [PROGRESS.md](./PROGRESS.md).** If it isn't, your
  job is to land that one — not to skip ahead.
- **One stage per session.** When the success bar goes green,
  update PROGRESS.md and stop. Write a follow-up note (see §6)
  for anything that spilled.
- **Do not edit the stage docs' "Scope" / "Wire shape" / "Schema"
  sections.** Those are locks earlier sessions agreed to. If a
  lock is wrong, file a note and raise it explicitly — don't
  silently change it.

---

## 1. Bring the stack up

From [rubix/](../../../), **always prefer `make restart`** — it is
the only target that guarantees the agent runs off your freshly-
built binary, which the e2e success bar requires:

```bash
make restart
```

If that fails (port conflict, stale state, mid-session crash),
fall back to `make start`:

```bash
make start
```

`make clean` is the nuclear option — it drops the docker volumes
(Postgres + ClickHouse). Do not run it without asking the user;
another session on the same branch may depend on the data.

Either target wraps `mani` + docker compose + bootstrap + agent +
vite. After it returns, two things must be true:

```bash
# backend listening
curl -s -o /dev/null -w "agent HTTP %{http_code}\n" http://127.0.0.1:8088/
# frontend listening
curl -s -o /dev/null -w "vite  HTTP %{http_code}\n" http://127.0.0.1:5173/
```

Both should be `200`. If not, tail the logs:

- `tail -50 /tmp/rubix-agent.log`
- `tail -50 /tmp/rubix-frontend.log`

ClickHouse + Postgres come up as part of `make restart`. Smoke
them directly:

```bash
curl -s http://127.0.0.1:8124/ping     # → "Ok."
PAGER=cat psql 'postgres://rubix:rubix-dev@127.0.0.1:5433/rubix' \
    -c 'SELECT 1' </dev/null            # → "1"
```

---

## 2. Log in (cookie jar)

Every verb in §3 needs an auth cookie. The bootstrapped operator
is fixed:

```
email:    op@example.com
password: rubix-dev-passwd
```

Log in once per session and store the cookie:

```bash
curl -s -c /tmp/smoke-cookies.txt \
    -X POST http://127.0.0.1:8088/api/v1/auth/login \
    -H 'Content-Type: application/json' \
    -d '{"email":"op@example.com","password":"rubix-dev-passwd"}' \
    -w '\nHTTP %{http_code}\n'
```

Expect `HTTP 200` and a JSON body. From here on every verb call
uses `-b /tmp/smoke-cookies.txt`.

If you get `401`, the operator wasn't bootstrapped — run
`make bootstrap` from `rubix/` and retry.

---

## 3. Calling a verb

Every tool is reachable two ways. Prefer REST in a session; MCP
is for AI-agent integration tests.

### 3.1 REST (the one you'll use)

```
POST /api/v1/tools/<tool.id>
Content-Type: application/json
Cookie:       (the session cookie)
Body:         the verb's request DTO, as JSON
```

Optional query strings:

- `?render=server` — server-renders the response for the
  i18n / SDUI path. Only pass this when the doc tells you to
  (the disk-probe smoke uses it; most verb calls don't).

Examples lifted from prior smoke runs (terminal history):

```bash
# system probe (no body needed)
curl -s -b /tmp/smoke-cookies.txt \
    -X POST 'http://127.0.0.1:8088/api/v1/tools/rubix.system.disk' \
    -H 'content-type: application/json' -d '{}'

# dashboard read
curl -s -b /tmp/smoke-cookies.txt \
    -X POST http://127.0.0.1:8088/api/v1/tools/rubix.dashboard.get \
    -H 'content-type: application/json' \
    -d '{"tenant_id":"system","page_id":"dashboard.e2e-test"}'

# flow deploy (lint first!)
curl -s -b /tmp/smoke-cookies.txt \
    -X POST http://127.0.0.1:8088/api/v1/tools/rubix.flow_ops.lint \
    -H 'content-type: application/json' -d @deploy.json
curl -s -b /tmp/smoke-cookies.txt \
    -X POST http://127.0.0.1:8088/api/v1/tools/rubix.flow_ops.deploy \
    -H 'content-type: application/json' -d @deploy.json
```

### 3.2 MCP / agent loop

Only use when the stage explicitly says so. The endpoint is the
agent's stdio MCP server (`rubix-admin mcp`); call it with the
matching test harness in `rubix-agent/tests/`.

---

## 4. Verbs you will actually use in stages 01–05

Cross-reference the design doc for the full DTO; this is just
the lookup table.

| Verb id                               | Used in stage | Design doc |
|---------------------------------------|---------------|------------|
| `rubix.flow_ops.lint`                 | 01            | [flow-programmer](../../design/flow-programmer/README.md) |
| `rubix.flow_ops.deploy`               | 01, 03        | [flow-programmer](../../design/flow-programmer/README.md) |
| `rubix.flow_ops.list`                 | 01, 03        | [flow-programmer](../../design/flow-programmer/README.md) |
| `rubix.clickhouse.rule.write`         | 02            | [clickhouse-rules](../../design/clickhouse-rules/README.md) |
| `rubix.clickhouse.mart.create`        | 03, 05        | [clickhouse-rules](../../design/clickhouse-rules/README.md) |
| `rubix.clickhouse.retention.set`      | 02, 03, 05    | [clickhouse-rules](../../design/clickhouse-rules/README.md) |
| `rubix.warehouse.ingest` (TBD bind)   | 02            | [warehouse](../../design/warehouse/README.md) |
| `rubix.alert.send` (via gate)         | 04            | [insights](../../design/insights/README.md) |
| `rubix.dashboard.page_set`            | 05            | [sdui/tools](../../design/sdui/tools/README.md) |
| `rubix.dashboard.get`                 | 05            | [sdui/tools](../../design/sdui/tools/README.md) |
| `rubix.analytics.query`               | 05            | [reports](../../design/reports/README.md) |
| `rubix.undo.last`                     | any           | [clickhouse-rules](../../design/clickhouse-rules/README.md), [flow-programmer](../../design/flow-programmer/README.md) |

---

## 5. Inspecting state directly (skip the verb)

These bypass the verb layer when you need ground truth.

### ClickHouse (warehouse)

```bash
curl -s -X POST -d \
    "SELECT count() FROM rubix.meter_readings_raw" \
    http://127.0.0.1:8124/

# always qualify with rubix. — the agent writes to that database,
# not default. See design/warehouse/README.md §"Database routing".
```

### Postgres (dimensions + changelog)

```bash
PAGER=cat psql 'postgres://rubix:rubix-dev@127.0.0.1:5433/rubix' \
    -c "SELECT actor_kind, resource_kind, op, at
          FROM starter_changes
         ORDER BY at DESC LIMIT 5" </dev/null
```

The `starter_changes` table is the audit trail every reversible
verb writes to. If a verb returned `HTTP 200` but you can't see
the effect, check this table first — the row tells you what the
verb actually recorded.

### Logs

- Agent: `tail -f /tmp/rubix-agent.log`
- Frontend (vite + tanstack route gen): `tail -f /tmp/rubix-frontend.log`

For verbs that dispatch alerts, raise the log level before the
verb call:

```bash
make stop
RUST_LOG=info,rubix_tools=debug make agent
```

---

## 6. When you finish (or get stuck)

### What "finished" means (read this before anything else)

A stage is **not** finished when:

- the code compiles, or
- `cargo test` passes, or
- the unit / integration test added by the stage is green, or
- a curl against a verb returns `HTTP 200`.

A stage **is** finished only when **live end-to-end testing
against a running stack** meets every bullet in the stage doc's
"Success bar" — the agent built from your commit is running on
`:8088`, the producer (or cleaner, or rollup) flow is actually
ticking, rows are landing in ClickHouse / Postgres, and the
direct-inspection queries from §5 above return what the success
bar says they should.

E2E testing is mandatory. Unit tests prove the shape; only the
live run proves the wiring. Both must pass.

### Finished a stage (success bar green, live e2e done)

1. Confirm the e2e run with the literal commands from §1, §2,
   §3, §5 above — restart the stack from your built binary,
   log in, drive the verbs the stage doc lists, then inspect
   ClickHouse / Postgres directly. Paste the inspection
   output into your session note as proof.
2. Run the e2e drive **twice** (cold restart between runs).
   Stage is not done if it only works once.
3. Update [PROGRESS.md](./PROGRESS.md) — flip that stage's row
   to ✅, fill in date + commit SHA + a one-line evidence
   snippet from the **live** run (e.g.
   `count(*)=237 in rubix.meter_readings_raw after 5min live run`).
4. Tick the stage doc's "Decisions taken" checklist (the
   shape/path choice you actually went with).
5. Stop. Do not start the next stage in the same session.

If the unit tests are green but the e2e run does not meet the
success bar, the stage is **not** done. Treat the gap as a
"Got stuck" case below.

### Got stuck

1. Walk the stage doc's "If it fails" list in order. Do not
   improvise — those three checks are deliberate.
2. If none of them is the cause, create a follow-up note:

   ```
   rubix/docs/sessions/data-flow/<stage-NN>-<topic>-YYYY-MM-DD.md
   ```

   Use the template at [_SESSION-TEMPLATE.md](./_SESSION-TEMPLATE.md).
3. Add a row to PROGRESS.md's "Follow-up notes" section pointing
   at the file.
4. Stop. Do not expand the stage doc itself.

---

## 7. Common foot-guns (from prior sessions)

- **Edit-tool stale buffer** — `read_file` can show your edit
  while `cargo` / `grep` still see the old bytes. After a
  multi-step edit, run `git status`; if a file you "wrote" isn't
  in the diff, re-apply by writing from the terminal.
- **`make agent` skips silently** if `:8088` is already bound.
  Run `make stop` first if you're not sure.
- **Unqualified ClickHouse SELECT** hits `default`, not `rubix`.
  Always write `FROM rubix.<table>`.
- **`mart.create` undo drops the mart**, losing every row
  materialised after the create. Re-run the materialiser flow
  to backfill. See [clickhouse-rules](../../design/clickhouse-rules/README.md#martcreate-undo-data-loss-caveat).
- **Producer/cleaner flow deployed but not firing** → the agent's
  `PgListener` didn't pick up the NOTIFY. See
  [flow-programmer](../../design/flow-programmer/README.md#cross-instance-notify-mechanism).
