# 2026-05-25 — Fix 500s on `/api/v1/auth/me` and `/api/v1/tools/rubix.flow_ops.list` after #38 + #39 merge

> **Tier:** session note. Lifetime: days. Per
> [HOW-TO-CODE.md §0a](../../HOW-TO-CODE.md) and
> [NEW-SESSION.md §2](../../NEW-SESSION.md), **source code must
> never reference this file.**

## The bug, as the operator sees it

After merging PR #39 (auth path prefix fix) and PR #38 (flow live-tick demo + NodeStateStore upstream + always-on flow runtime), the rubix frontend at `http://127.0.0.1:5173` cannot reach the agent. Two reproducible 500s in the browser network panel:

```
Request URL    http://127.0.0.1:5173/api/v1/auth/me
Request Method GET
Status Code    500 Internal Server Error
```

```
Request URL    http://127.0.0.1:5173/api/v1/tools/rubix.flow_ops.list
Request Method POST
Status Code    500 Internal Server Error
```

`5173` is the vite dev server; the existing proxy in [`rubix/frontend/vite.config.ts`](../../frontend/vite.config.ts) forwards `/api/v1/*` to `127.0.0.1:8088` (the rubix-agent). The error therefore comes from the agent, not vite. **500 means the request reached the backend and the backend errored**, not a routing miss (404) and not an auth deny (401) — those would surface differently.

This regressed *after* the merges of #38 + #39. The 401 we saw in the previous session note ([`2026-05-24-auth-path-mismatch-fix.md`](./2026-05-24-auth-path-mismatch-fix.md)) was caused by the client calling the wrong path; that was fixed in #39 (paths now land under `/api/v1/auth/*`). The fact that the request now reaches the agent and gets a 500 means **the auth fix landed** — we've traded a 401 for a 500. Progress, but a different bug.

## Read first

Before touching anything:

1. [HOW-TO-CODE.md](../../HOW-TO-CODE.md) — contributor entry point.
2. [SCOPE.md](../../SCOPE.md) — R1–R13.
3. [`rubix/docs/sessions/2026-05-25-handover-flow-crud-and-orientation.md`](./2026-05-25-handover-flow-crud-and-orientation.md) — the current handover with the orientation map (rubix/starter/extensions/codeless), where everything lives, the codeless runbook. Read §1 (orientation) and §5 (codeless runbook) for context.
4. [`rubix/docs/sessions/2026-05-24-auth-path-mismatch-fix.md`](./2026-05-24-auth-path-mismatch-fix.md) — the previous prompt for the 401-on-auth-paths bug. The fix landed in #39; this session is the next layer.
5. [`rubix/docs/sessions/2026-05-24-tool-registry-gap.md`](./2026-05-24-tool-registry-gap.md) — the prior session that fixed missing tool registrations. The rubix-agent's tool dispatch is mounted through the registry; gaps here are a known failure mode.
6. The two recent merges that may have caused this:
   - **PR #39** ([commit `3ef5632`](https://github.com/NubeDev/starter/commit/3ef5632)) — auth path prefix fix + a sweep of "register flow_ops/user/tenant/team tool families" + "close every remaining 404 on the rubix tool surface" commits.
   - **PR #38** ([commit `722f530`](https://github.com/NubeDev/starter/commit/722f530)) — added `NodeCtx.state: &dyn NodeStateStore` field (load-bearing API addition; every existing `NodeCtx` call site had to update), added `boot::flow_runtime` always-on mounter, added SSE flow-events route under `/api/v1/flows/<id>/events`, refactored `flow_ops.list` to carry `body_yaml`, added `flow_ops.kinds` endpoint.
7. **The known rebase note** — during PR #38's rebase onto master after #39 landed, the conflict on `rubix/frontend/src/routes/flows/$flowId.tsx` was resolved by taking codeless's version (loses master's `PageContainer`/`PageHeader` cosmetic touch on that file). This is unlikely to cause 500s on `/api/v1/*` but document it as background.
8. The agent's stderr log — `tail -F /tmp/rubix-agent.log` (path set by [`rubix/Makefile`](../../Makefile)). **Every 500 dropped today should produce a stack-trace or `error` line there.** Read it before guessing.

## The diagnosis pyramid — work the cheapest layer first

Five plausible classes of cause, ordered cheapest to most expensive to investigate.

### Class 1 — The agent didn't boot cleanly

Stale `target/debug/rubix-agent` binary; the merge of #38 changed Rust contracts (notably `NodeCtx.state`); if you ran `make start` without a fresh `cargo build`, the binary is from before the merge and the boot fails in a way that returns 500 on every request.

**Check.**

```bash
# is the agent actually live and responding?
curl -sv http://127.0.0.1:8088/healthz 2>&1 | head -10
# expect HTTP/1.1 200 with body {"status":"ok"} or similar

# what does the boot log say?
head -120 /tmp/rubix-agent.log
# look for the expected boot-canary lines from the smoke-test note:
#   INFO rubix.boot: rubix migrations applied sources=2
#   INFO rubix.boot: rubix ClickHouse migrations applied
#   INFO rubix_agent::boot::mcp: rubix MCP surface assembled mcp_tools=6
#   INFO rubix_agent::boot::flow_notify: rubix_flows_definitions listener ready
#   INFO rubix_agent::boot::flow_runtime: live FlowRunner mounted flows=N  (NEW post-#38)
#   INFO rubix-agent starting ... i18n_keys=34 mcp_tools=6 ...
#   INFO rubix_agent::health: rubix-agent listening bind=127.0.0.1:8088
```

If `/healthz` 200s but `/api/v1/auth/me` 500s, skip to Class 2. If `/healthz` itself errors or the boot log is missing migrations / flow_notify / flow_runtime lines, the binary is stale or the boot itself failed. Rebuild:

```bash
cd /home/user/code/rust/starter/rubix
make stop                    # if you have a stop target; otherwise pkill -f target/debug/rubix-agent
cargo build -p rubix-agent   # rebuild against new master
make start                   # docker stack up + agent + frontend
tail -F /tmp/rubix-agent.log # watch the boot
```

This alone may resolve both 500s.

### Class 2 — A migration is missing or out of order

PR #38 added a new sqlite-backed `node_state` table (separate from PG; lives at `~/.rubix/node_state.db`). PR #38 also generalised the schedule mounting which uses `scheduled_flows`. PR #39 also added migrations referenced in its body. If the agent boots but `auth/me` 500s, the most likely cause inside the agent is a panic in a route handler that reaches into a missing table.

**Check.**

```bash
# what tables exist in PG?
docker exec docker-rubix_postgres-1 psql -U rubix -d rubix \
  -c "\dt"

# expect (post-#38): users, sessions, csrf, changelog, flows_definitions,
#                    scheduled_flows, undo_snapshots, extensions_enablement, ...
# the auth path needs users + sessions; if either is missing, /auth/me 500s

# is the sqlite node_state DB present?
ls -la ~/.rubix/node_state.db 2>&1

# what does the agent log say at the exact moment of the 500?
# re-trigger the failing request, then:
tail -30 /tmp/rubix-agent.log

# the panic / error / span carrying the request id will be there.
# starter-mcp's RpcError::internal_from_source walks the source chain
# (per the pr30 smoke fix B7) so the message should be informative.
```

If migrations failed silently, the boot log shows `WARN rubix.boot: migrations skipped` or similar. Re-run:

```bash
RUBIX_DSN=postgres://rubix:rubix-dev-passwd@127.0.0.1:5432/rubix \
  cargo run -p rubix-agent --bin rubix-admin -- migrate
```

### Class 3 — A handler panics on a missing column / row shape

The most subtle class. The new `flow_ops.list` returns `body_yaml`; the new `flow_ops.kinds` reads the kind registry. If either is mis-wired against an old DB row shape or an old serialisation, the request reaches the handler and the handler errors.

**Check.**

```bash
# what columns are on flows_definitions?
docker exec docker-rubix_postgres-1 psql -U rubix -d rubix \
  -c "\d flows_definitions"

# expect post-#38: id, tenant_id, flow_id, revision_id, body_yaml,
#                  created_by, created_at, superseded_at, ...
# if body_yaml is missing the list handler 500s when it tries to SELECT it.
```

Also worth checking the auth path specifically since `/auth/me` is the surface the operator hits first:

```bash
# what columns are on users + sessions?
docker exec docker-rubix_postgres-1 psql -U rubix -d rubix \
  -c "\d users; \d sessions;"

# the /auth/me handler reads the principal from the session cookie,
# looks up the user. if either query is broken at the column level
# the response is a 500.
```

### Class 4 — The cookie isn't reaching the agent

A 500 from `/auth/me` could also mean the session cookie is present but rejected at parse time. Unlikely (parse failures usually 401 not 500), but worth a glance.

**Check.**

```bash
# what cookies does the browser have for 127.0.0.1?
# open dev tools → Application → Cookies → http://127.0.0.1
# expect: starter_session (HttpOnly), starter_csrf

# if missing: log out via /api/v1/auth/logout, log back in, try again.
# (the previous auth-path-fix session note covered this surface in depth.)
```

### Class 5 — Vite proxy or CORS shape changed

Least likely given the agent log shows the request hitting the backend. Skip unless Classes 1–4 are clean and the 500 persists.

## Resolution flow

Step through the diagnosis pyramid in order. For each class, capture in this note's **Resolution** section at the bottom:

- The exact command run.
- The exact output (truncate long).
- Whether it confirmed or eliminated the class.

The first class that confirms gets a one-paragraph fix narrative + a small PR.

## Likely culprits, ranked

Given what's in #38 + #39 and what could break the auth path specifically:

1. **Stale binary / dirty boot** — `cargo build -p rubix-agent` against the new master may be needed. If you ran `make start` without rebuilding after the merges, the binary is from before NodeCtx grew its `state` field and the post-build boot is undefined. **First thing to try.**

2. **A handler panic carrying through to a 500** — PR #38's `boot::flow_runtime` mounts a `FlowRunner` per deployed flow. If any flow YAML fails to resolve at boot (a kind id mismatch, a missing slot type, etc.), the mounter may panic and a wrapping handler returns 500 on every subsequent request. The pr30 smoke fix B7 ensured `RpcError::internal_from_source` walks the source chain; the agent log should carry the cause.

3. **PG row-shape drift** — if a column added in #38's migration isn't applied (because migrations skipped silently), a SELECT in `auth/me` or `flow_ops.list` errors out. The boot log's `migrations=N migrations_skipped=false` canary is the test.

4. **The rebase resolution dropped something material** — when #38 rebased onto master after #39 landed, 9 commits dropped as duplicates and `$flowId.tsx` was taken from codeless's version. The lost commits were all known duplicates (the same auth-path fixes that came in via #39); the dropped `PageContainer` cosmetic touch on `$flowId.tsx` cannot cause a 500 on `/auth/me`. Documented for completeness; not the cause.

## The work

One PR off `fix/post-merge-500s`. Likely one to three commits depending on what the diagnosis surfaces.

### Stage 1 — diagnose

Run through Classes 1–4 in order. Record findings inline in this session note's Resolution section. Do not fix until the cause is named.

### Stage 2 — fix

Apply the smallest change addressing the diagnosed cause. Commit message follows the convention:

- Class 1 (stale binary): no commit needed; document in Resolution.
- Class 2 (missing migration): `fix(rubix-agent): apply pending migrations + flag on boot`.
- Class 3 (handler panic): `fix(rubix-tools): <verb-name> handles <specific-shape>` with a unit test that fails before the fix.
- Class 4 (cookie): document; usually a logout / login is the user-side fix; if a server-side cookie-parse bug surfaces, `fix(starter-auth-users): <one-line>`.

### Stage 3 — verify

Re-run the smoke loop:

```bash
cd /home/user/code/rust/starter/rubix
make restart                                                  # full reset
# wait for the boot log to show mcp_tools=6 + i18n_keys=34
JAR=/tmp/rubix-smoke-jar
rm -f "$JAR"

# 1. health
curl -sv http://127.0.0.1:8088/healthz 2>&1 | tail -5

# 2. login
curl -sv -c "$JAR" -b "$JAR" -X POST http://127.0.0.1:8088/api/v1/auth/login \
     -H 'content-type: application/json' \
     -d '{"email":"op@example.com","password":"rubix-dev-passwd"}' \
   | tail -5

# 3. /auth/me — the previously-failing GET
curl -sv -b "$JAR" http://127.0.0.1:8088/api/v1/auth/me 2>&1 | tail -10
# expect HTTP/1.1 200 + JSON principal

# 4. flow_ops.list — the previously-failing POST
CSRF=$(awk '$6 == "starter_csrf" {print $7}' "$JAR")
curl -sv -b "$JAR" -X POST http://127.0.0.1:8088/api/v1/tools/rubix.flow_ops.list \
     -H 'content-type: application/json' \
     -H "x-csrf-token: $CSRF" \
     -d '{}' 2>&1 | tail -15
# expect HTTP/1.1 200 + JSON with summary + flows array

# 5. confirm in the browser
# open http://127.0.0.1:5173, log in, no 500s in the network panel
```

If 1–5 all clean, update this note's Resolution section + open the PR.

## Out of scope

- **Fixing the stale CI checks** (cargo fmt drift, pnpm api snapshot drift, starter-spi dep baseline). The operator said they'd handle those later.
- **The `PageContainer`/`PageHeader` cosmetic regression** on `$flowId.tsx` from the rebase resolution. Tracked as a follow-up; not part of this session.
- **PR #40 (dashboards-goal-1)** is still running in codeless. When it finishes it'll rebase onto the new master independently.
- **No backend redesign.** This is a regression fix, not a feature.
- **No `--no-verify`, no `--force`.**

## Hard rules

- R1 — verb per file, ≤ 400 lines.
- R3 — code comments link `docs/design/<area>/README.md` only; this session note (under `docs/sessions/`) is unreferenced from any source file.
- R6 — tests live with the code in the same commit; if Stage 2 lands a fix, the regression test ships in the same commit.

## Bootstrap user (carry-forward)

`op@example.com` / `rubix-dev-passwd` (admin). Created idempotently by `rubix/Makefile`'s `bootstrap` target. Verify exists:

```bash
docker exec docker-rubix_postgres-1 psql -U rubix -d rubix \
  -c "SELECT email, role FROM users ORDER BY created_at LIMIT 5;"
```

## References

- [`rubix/Makefile`](../../Makefile) — `make start` / `make restart` targets.
- [`rubix/crates/rubix-agent/src/main.rs`](../../crates/rubix-agent/src/main.rs) — boot wiring + router assembly.
- [`rubix/crates/rubix-agent/src/boot/`](../../crates/rubix-agent/src/boot/) — every boot module incl. `flow_runtime.rs` (new in #38).
- [`rubix/crates/rubix-tools/src/flow_ops/list.rs`](../../crates/rubix-tools/src/flow_ops/list.rs) — flow_ops.list handler (touched in #38).
- [`crates/starter-auth-users/src/`](../../../crates/starter-auth-users/) — `/auth/me` and the session lookup path.
- [`rubix/frontend/vite.config.ts`](../../frontend/vite.config.ts) — the proxy config; confirms `/api/v1` forwards to `127.0.0.1:8088`.
- [`rubix/docs/sessions/2026-05-25-handover-flow-crud-and-orientation.md`](./2026-05-25-handover-flow-crud-and-orientation.md) — current handover + codeless runbook.
- [`rubix/docs/sessions/2026-05-24-auth-path-mismatch-fix.md`](./2026-05-24-auth-path-mismatch-fix.md) — previous auth-related session note.
- PR #38 + PR #39 — recent merges to suspect.

## Resolution

### 1. Diagnosis pyramid walkthrough

**Class 1 (boot).** `curl http://127.0.0.1:8088/healthz` failed with connection refused. `tail /tmp/rubix-agent.log` showed boot abort:

```
Error: register `com.rubix.tick-counter`: topology resolve failed for com.rubix.tick-counter
  revision …: unknown node kind `starter.flow.trigger.schedule` for node `com.rubix.tick`
  — kind is not registered
```

The agent never reached `listening`; every `/api/v1/*` call therefore got proxied to a dead socket and the vite dev server surfaced that as 500. Classes 2–5 never needed checking.

### 2. Root cause

PR #38 added the bundled `com.rubix.tick-counter` flow (the first multi-node, multi-link bundled flow). Three latent gaps in the surrounding wiring tripped at the same boot step:

1. **`boot::mcp::register::build_flow_registry`** built a fresh `NodeKindRegistry` carrying only the `com.rubix.ai-agent` kind. The bundled flows had only ever needed that one kind; tick-counter is the first to reference `starter.flow.{trigger.schedule, counter, log}` and topology resolution fails when those aren't registered.
2. **`rubix-flows::convert`** prefixes every YAML node id with `com.rubix.` (so `id: tick` becomes `NodeId("com.rubix.tick")`) but copied link endpoints through verbatim. `tick.fire` therefore parsed as the (invalid) `NodeId("tick")` and the resolver rejected the endpoint as malformed.
3. **`tick-counter.yaml`'s log node** declared `message_template: "tick {value}"`. `LogSettings` is `#[serde(deny_unknown_fields)]` and only accepts `level` — schema validation rejected the unknown field. (`message_template` is a wishlist field; not implemented in `starter-flow-nodes::log` today.)

Any one of these would have crashed the boot the same way, masking the others. They had to be fixed together.

### 3. Files changed

- [rubix/crates/rubix-agent/src/boot/mcp/register.rs](../../crates/rubix-agent/src/boot/mcp/register.rs) — register `counter` + `log` + `trigger_schedule` into the `NodeKindRegistry` (via `register_builtin`, since they live under the reserved `starter.flow.*` prefix).
- [rubix/crates/rubix-flows/src/convert.rs](../../crates/rubix-flows/src/convert.rs) — new `qualify_endpoint` helper that prefixes link endpoints with `NODE_ID_PREFIX` to match the prefix already applied to node ids. Idempotent: endpoints already in the qualified form pass through unchanged.
- [rubix/crates/rubix-flows/flows/tick-counter.yaml](../../crates/rubix-flows/flows/tick-counter.yaml) — drop the unsupported `message_template` from the log node's config (the `log` kind only takes `level` today).

Also reseeded the bundled row in PG so the corrected YAML body lands (the seed is idempotent on `(tenant, flow_id, revision_id)` and the existing row carried the stale body):

```sql
DELETE FROM flows_definitions WHERE flow_id='com.rubix.tick-counter';
```

### 4. Smoke evidence

```
=== /healthz ===           200
=== POST /auth/login ===   HTTP 200 (csrf_token returned)
=== GET  /auth/me ===      HTTP 200 (subject/email/role for op@example.com)
=== POST /tools/rubix.flow_ops.list === HTTP 200 (count=7, flows array w/ body_yaml)
```

Boot log canaries:

```
INFO rubix.boot.scheduler: seeded scheduled flow flow_id=com.rubix.tick-counter cron_expr=*/5 * * * * *
INFO rubix.boot.scheduler: durable scheduler running seeded=2
INFO rubix.boot.flow_runtime: NodeStateStore: SQLite (durable) path=~/.rubix/node_state.db
INFO rubix_agent::health: rubix-agent listening bind=127.0.0.1:8088
```

### 5. Follow-ups surfaced

- `log` node has no `message_template` support. The wishlist field is referenced by the tick-counter design narrative in PR #38; either implement it (extend `LogSettings` + `Log::invoke`) or remove the design reference. Tracked as a small UX follow-up; not blocking.
- The kind set in `build_flow_registry` and `registry::builtin_kind_behaviors` are maintained separately and must stay in sync. A future cleanup could share a single source of truth.
- CI drift (cargo fmt, pnpm api snapshot, starter-spi dep baseline) per the original "out of scope" list still pending.
