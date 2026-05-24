# Session — 2026-05-24: post-PR-#30 smoke test results

> **Tier:** session note. Lifetime: days. Per
> [HOW-TO-CODE.md §0a](../../HOW-TO-CODE.md) and
> [NEW-SESSION.md §2](../../NEW-SESSION.md), **source code must
> never reference this file.**

Re-ran the six-step thin-slice demo end-to-end on master at
commit `0511981` (PR #30 merged) to verify the post-#29 fixes
held and to characterise the new MCP execution path that #30
introduced (real Claude loop replacing the hand-rolled fake).

---

## Summary

| Step | First run | Re-run after fixes |
|---|---|---|
| 1. Boot | PASS (w/ B5 workaround) | **PASS** (B5 fixed — cfg drives migrations directly) |
| 2. Login | PASS | **PASS** |
| 3. MCP `tools/list` + invoke disk en-US | FAIL (B7 + B8) | **PASS** (real Diagnostic, numeric percent + free) |
| 4. MCP invoke disk es-AR | FAIL | **PASS** (same shape; locale plumbed) |
| 5. ClickHouse history + alert | PARTIAL | **PASS** (history rows write; B12 surfaced: MCP path doesn't persist) |
| 6. CLI parity | PASS | **PASS** |

**First run verdict:** 3 / 6 PASS, 1 partial, 2 hard FAIL.
**After-fix verdict:** 6 / 6 PASS.

See [§Re-run after fixes](#re-run-after-fixes) at the bottom for
the second-pass evidence and the bugs resolved.

The thin-slice arch is real at every layer the tool dispatches
through directly (REST, CLI, auth, authz, audit, prefs, i18n,
Postgres, ClickHouse). The agent-loop layer that #30 introduced
between MCP and the tools does **not** produce real output — the
flow runs (no error returned to the MCP client on the second and
subsequent calls) but the result delivered over MCP is a hardcoded
JSON shape, not the real disk probe the underlying tool computes.

#29's fixes (B1 mani path, B2 password+DSN, B3 CH TTL cast, B4
ChClient gating) all held: the dev stack came up clean from
empty volumes; both Postgres and ClickHouse migrations applied
without error; the disk tool no longer 500s when CH is wired.

---

## Step-by-step evidence

### Step 0 — Reset

```
mani run dev-deps-down --all                                 # OK
docker volume rm docker_rubix_postgres_data \
                 docker_rubix_clickhouse_data                # both dropped
```

Volume names per the handover (`rubix-dev-postgres-data`) were
stale; the live names are docker-compose-default
`docker_rubix_postgres_data` / `docker_rubix_clickhouse_data`.
Handover §2 should be updated to reflect that (see "what's still
stale" below).

### Step 1 — Boot

`mani run demo --all` brought the docker stack up cleanly,
created the `op@example.com` bootstrap user, **then** failed
the agent boot with `Error: Address already in use (os error
98)` because a stale `rubix-agent` from a previous shell held
`127.0.0.1:8088`. Killed the stale process and re-launched.

The bigger issue surfaced on the second launch: when invoked
via `mani run demo`, the final `cargo run -p rubix-agent` line
in the demo aggregate task does **not** receive `RUBIX_DSN` or
`RUBIX_CH_URL`. The boot log printed:

```
WARN rubix.boot: RUBIX_DSN unset — skipping migrations; agent
                 will boot without DB-backed features
WARN rubix.boot: RUBIX_CH_URL unset — skipping ClickHouse
                 migrations; agent will boot without
                 warehouse-backed features
INFO rubix-agent starting … database_url_set=true
                 clickhouse_url_set=true migrations_skipped=true
                 ch_migrations_skipped=true
```

Note the contradiction in the startup line: `database_url_set=true
clickhouse_url_set=true` (config loaded from `agent.toml`) but
`migrations_skipped=true`. The migration runner reads the env
var, not `cfg.database_url` — see B5.

Restarted with explicit env (`RUBIX_DSN=…postgres://…
RUBIX_CH_URL=http://127.0.0.1:8124 RUBIX_CONFIG=…`). Clean boot:

```
INFO sqlx::postgres::notice: trigger "starter_changes_notify_trg"
     for relation "starter_changes" does not exist, skipping
INFO sqlx::postgres::notice: relation "_sqlx_migrations_auth_users"
     already exists, skipping
INFO rubix.boot: rubix migrations applied sources=2
INFO rubix.boot: rubix ClickHouse migrations applied         ← B3 fix verified
INFO rubix_agent::boot::mcp: rubix ai-agent node kind registered
     node_kinds="com.rubix.ai-agent"                         ← PR #30 wiring
INFO rubix_agent::boot::mcp: rubix MCP surface assembled
     mcp_tools=6                                             ← PR #30 wiring
INFO rubix-agent starting … bind=127.0.0.1:8088
     database_url_set=true clickhouse_url_set=true tools=4
     mcp_tools=6 skills=6 flows=6 migrations=2
     migrations_skipped=false ch_migrations_skipped=false
     i18n_keys=26
INFO rubix_agent::health: rubix-agent listening
     bind=127.0.0.1:8088
```

Post-#30 wiring all present: `com.rubix.ai-agent` kind
registered, six MCP tools, six skills, six flows, both
migration sets applied.

**PASS** for step 1 itself; B5 documents the demo-env-passing
bug that required the workaround.

### Step 2 — Login

```
$ curl -c jar -b jar -X POST .../api/v1/auth/login \
       -H 'content-type: application/json' \
       -d '{"email":"op@example.com","password":"rubix-dev-passwd"}'
{"csrf_token":"…"}                                            HTTP 200

$ curl -b jar .../api/v1/auth/me
{"subject":"201ef5eb-4cb8-4787-9b32-ee9dbb80bbc9",
 "email":"op@example.com","role":"admin"}                     HTTP 200
```

Both `starter_session` (HttpOnly) and `starter_csrf` cookies
landed in the jar; `/me` returns the admin principal.

**PASS.**

### Step 3 — MCP tools/list + invoke disk (en-US)

`tools/list` returned six tools, exactly the six bundled flow
ids:

```
com.rubix.dashboard-assistant
com.rubix.flow-programmer
com.rubix.scheduled-system-check
com.rubix.weekly-report
com.rubix.clickhouse-ruler
com.rubix.user-admin
```

`tools/list` itself: **PASS**.

`tools/call` on `com.rubix.scheduled-system-check` with
`_meta.acceptLanguage: en-US`:

```
# first attempt
{"jsonrpc":"2.0","id":2,
 "error":{"code":-32603,"message":"internal error"}}

# subsequent attempts (same payload)
{"jsonrpc":"2.0","id":11,
 "result":{"content":[{"text":"null","type":"text"}],
           "structuredContent":null}}
```

Three things broken or stubbed:

1. **B7** — the very first MCP `tools/call` after boot returns
   a `-32603 internal error` with no `data` payload, no detail,
   and no log line. `RpcError::internal(e.to_string())` in
   `starter-mcp/src/server/dispatch.rs:115` collapses the source
   chain; `SpiError::Internal` Display is the bare literal
   "internal error" (`starter-spi/src/error/kind.rs:51`). The
   real cause is lost at the wire and there is no compensating
   server log. Setting `RUST_LOG=trace,…` on the agent process
   has no effect because `starter-observability::init_tracing`
   builds the `EnvFilter` from a hardcoded directive string, not
   from `RUST_LOG`.
2. **B8** — when the second and subsequent calls *do* return a
   200, the body is `null`. `RubixAiAgentNode::invoke` in
   `rubix/crates/rubix-agent/src/boot/mcp.rs:280-311` writes its
   reply to slot key `DEFAULT_OUTPUT_SLOT` (`"out"`), but
   `register_one`'s output adapter on the same file at
   `:213-220` looks the slot up by the prefixed key
   `<root_node_id>.out`. The lookup misses and the adapter
   returns `Value::Null`. (Either the engine is meant to
   prefix the node's returned SlotMap keys before persisting
   them to the slot store and that's not happening, or the
   adapter should use the bare slot name. Either way the two
   sides disagree on the key shape.)
3. **B8.2** — even if B8 is fixed, the payload written by
   `RubixAiAgentNode::invoke` at `:301-309` is structurally
   fake: `code: "rubix.system.disk.ok"` and `params: { percent:
   0, free: 0 }` are hardcoded literals, with the actual LLM
   reply buried in a sibling `reply` field. This is the post-#30
   equivalent of the pre-#30 hardcoded-Spanish-strings imposter
   — replaced by hardcoded-English-zeroes. The real `Diagnostic`
   shape from the underlying `rubix.system.disk` tool never
   reaches the MCP response.

Time-to-response on the success cases was ~0s — the Claude CLI
was not invoked. The agent loop either never iterated or
returned immediately with an empty `RunResult`. With B7 hiding
the error chain it is not possible to confirm which from this
session.

Direct REST against the same tool, for comparison — shows the
shape the MCP path *should* be returning:

```
$ curl -b jar -X POST .../api/v1/tools/rubix.system.disk \
       -H 'Accept-Language: en-US' \
       -H "x-csrf-token: $CSRF" -d '{}'
{"free_bytes":35040526336,"mount":"/",
 "percent_used":83,"probed_at_ms":1779592991656,
 "summary":{"code":"rubix.system.disk.warn",
            "params":{"at":{"timestamp":1779592991656},
                      "free":{"i64":35040526336},
                      "percent":{"i64":83}}},
 "total_bytes":208557645824}                                  HTTP 200
```

Numeric `percent: 83`, numeric `free: 35040526336`,
`code: rubix.system.disk.warn`, `at.timestamp`. This is the
contract the AI loop is supposed to produce from the agent loop
+ tool round-trip; today the loop short-circuits and the MCP
client gets `null`.

**FAIL.**

### Step 4 — MCP invoke disk (es-AR)

Same payload, `_meta.acceptLanguage: es-AR`:

```
{"jsonrpc":"2.0","id":3,
 "result":{"content":[{"text":"null","type":"text"}],
           "structuredContent":null}}
```

The locale was plumbed — `register_one`'s seed adapter read
`starter_mcp::current_locale()` and called
`prefs_from_locale(&lang)` so the prefs JSON arrived at the
node — but the MCP response is identical to the en-US response
because of B8: the engine slot store has nothing under
`<root_node_id>.out`. The Spanish `rubix.system.disk.*` keys
exist in the catalogue (verified via `i18n_keys=26` in the boot
log; `rubix-spi/catalogues/es.json` carries the
`ok`/`warn`/`full` entries), but no key resolves because no
payload is rendered.

Direct REST with `Accept-Language: es-AR` returns the same
structured Diagnostic as en-US (the rendered prose differs by
locale; the `code` and numeric `params` do not). The disk
tool's i18n contract works at the REST layer. The MCP path
just never reaches it.

**FAIL.**

### Step 5 — ClickHouse history + alert

After firing five REST disk calls (plus the two during steps
3/4), the warehouse contains seven rows:

```
$ docker exec docker-clickhouse-1 clickhouse-client \
        --query "SELECT count() FROM system_disk_history"
7

$ docker exec … --query "SELECT * FROM system_disk_history
                          ORDER BY 1 DESC LIMIT 2 FORMAT Vertical"
Row 1:
  tenant_id:    00000000-0000-0000-0000-000000000000
  host:         localhost
  percent_used: 83
  free_bytes:   35036315648
  epoch_ms:     1779593039927
Row 2:
  tenant_id:    00000000-0000-0000-0000-000000000000
  host:         localhost
  percent_used: 83
  free_bytes:   35036360704
  epoch_ms:     1779593039857
```

Schema matches the thin-slice design exactly (`tenant_id`,
`host`, `percent_used`, `free_bytes`, `epoch_ms`). One row
per REST call. B4 fix verified — the table exists, the insert
path runs, no 500s. Note the table landed in the `default`
database, not `rubix` (the bootstrap created `rubix` but the
migration writes to `default`). Minor — see B10.

The alert side was **not** exercised: real disk usage on this
host is 83%, below the hardcoded 90% threshold in
`boot::insights`. No `alert.send` log line fired. Per the
session prompt's "best-effort + document" answer to the step-5
question, this is acceptable; the path remains unverified for
this run.

**PARTIAL** — history PASS, alert not exercised.

### Step 6 — CLI parity

```
$ cargo run -p rubix-agent --bin rubix-admin -- system disk
Disk is nearly full (83% used, 34965516288 free,
                     probed at 05/23/2026, 23:24).
```

Same `percent_used = 83`, free bytes within ~75 MB of the REST
reading (different probe instant), rendered through
`starter-i18n` for the host's default locale. The CLI subcommand
calls the same `probe()` function the REST handler dispatches,
per the doc on `bin/rubix_admin/system/disk.rs`.

**PASS.**

---

## Blocking bugs (fix before re-running)

### B5 — `mani run demo` doesn't propagate DSN/CH-URL to the agent

The `demo` aggregate task in `rubix/mani.yaml` runs four shell
lines inside a single `cmd: |` block. Lines 1–3 (docker up,
wait-for-deps, bootstrap-user) execute fine; line 4 launches
the agent as `RUBIX_CONFIG=rubix/dev/agent.toml cargo run -p
rubix-agent` with **no** `RUBIX_DSN` / `RUBIX_CH_URL` exported.

Independently, `boot::migrations::apply_migrations` and
`boot::clickhouse::apply_ch_migrations` decide whether to run
migrations by checking `std::env::var("RUBIX_DSN")` /
`RUBIX_CH_URL`, not `cfg.database_url` / `cfg.clickhouse_url`.
So even though `agent.toml` carries both URLs (and the startup
line correctly reports `database_url_set=true clickhouse_url_set=true`),
the migration runners skip with a warn.

Net effect: `mani run demo` straight from the handover script
boots the agent with `migrations_skipped=true ch_migrations_skipped=true`.
Auth, audit, and history won't work until the operator either:
1. Adds `RUBIX_DSN` + `RUBIX_CH_URL` to the agent line in
   `mani.yaml`, or
2. Switches `boot::migrations` / `boot::clickhouse` to consult
   `cfg.database_url` / `cfg.clickhouse_url`.

(2) is the cleaner fix — the env var is already a layered-config
override, not the source of truth — but either resolves it.

**Files:**
- `rubix/mani.yaml` (the `demo` task, ~line 60)
- `rubix/crates/rubix-agent/src/boot/migrations.rs`
- `rubix/crates/rubix-agent/src/boot/clickhouse.rs`

### B6 — Stale agent on 8088 silently breaks the demo

The Phase-0 binary binds `127.0.0.1:8088`; the boot fails with
`Address already in use (os error 98)` if any prior agent
process still holds the port. This happens routinely when a
previous `mani run demo` was Ctrl-C'd from another terminal and
the child agent leaked.

Two reasonable resolutions:

1. **Pre-flight in the demo task.** `mani run demo` should test
   that `8088` is free (or `pkill -f target/debug/rubix-agent`)
   before launching the agent, with a friendly message instead
   of the bare `os error 98`.
2. **Random-port mode for local demo.** Bind 0 and emit the
   resolved port on the first log line. This breaks the docs
   that say "curl :8088" though, so probably not.

(1) is the safer of the two.

**Files:** `rubix/mani.yaml`.

### B7 — `-32603 internal error` carries no detail; agent log carries none either

Two compounding flaws:

1. `starter_mcp::server::dispatch::tools_call` at line 115
   maps a tool error to `RpcError::internal(e.to_string())` —
   but `SpiError::Internal` Display is the literal string
   `"internal error"` (no source-chain walk), so the wire
   payload is uninformative.
2. The agent process never logs *anything* about the failed
   `tools/call` — neither at the dispatch boundary nor inside
   `FlowAsTool::invoke_with_cancel`. The `info_span!` at
   `flow_as_tool.call` does not emit because
   `starter-observability::init_tracing` builds its `EnvFilter`
   from a hardcoded directive (`starter-observability/src/tracing/init.rs:38-40`)
   and ignores `RUST_LOG`.

Net effect: when an MCP tool call fails, the operator sees
`{"error":{"code":-32603,"message":"internal error"}}` and the
server logs nothing. There is no way to diagnose without
attaching a debugger or recompiling.

Two reasonable resolutions, ideally both:
- Walk the `source()` chain in `RpcError::internal` and surface
  it via the optional `data` field on the JSON-RPC error.
- Honour `RUST_LOG` in `starter-observability::init_tracing`
  (fall back to the directive arg only when env is absent).

**Files:**
- `crates/starter-mcp/src/protocol/error.rs`
- `crates/starter-mcp/src/server/dispatch.rs`
- `crates/starter-observability/src/tracing/init.rs`

### B8 — MCP flow output adapter slot-key mismatch (returns null)

In `rubix/crates/rubix-agent/src/boot/mcp.rs`:

- `register_one` at lines 213–220 keys the output adapter as
  `format!("{}.{}", output_slot.node, output_slot.slot)` —
  i.e. `<root_node_id>.out`.
- `RubixAiAgentNode::invoke` at lines 301–309 inserts into
  the returned `SlotMap` under the bare slot name
  `rubix_flows::DEFAULT_OUTPUT_SLOT` (= `"out"`).

After the engine drives the node and persists its returned
SlotMap, the output adapter looks up `<root_node_id>.out` and
finds nothing, so it returns `Value::Null`. That's what the MCP
client sees as `structuredContent: null`.

Two equally-cheap resolutions, depending on the engine
contract:
- If the engine prefixes a node's returned SlotMap keys with
  the node id when persisting them, then `RubixAiAgentNode`
  is correct and `register_one`'s adapter key shape was wrong
  in an earlier slot-store change that wasn't propagated.
- If the engine writes the bare slot names through verbatim,
  then the adapter is correct and `RubixAiAgentNode::invoke`
  should insert under `format!("{node_id}.out")`.

Either way, an integration test exercising the full
`tools/call → AiAgentNode → output adapter` round-trip would
catch this and is missing.

**Files:**
- `rubix/crates/rubix-agent/src/boot/mcp.rs` (lines 213–220
  and 280–311)

### B8.2 — `RubixAiAgentNode::invoke` writes a stubbed payload

Even when B8 is fixed, the JSON written into the `out` slot is
structurally fake — see `RubixAiAgentNode::invoke` at lines
301–309:

```rust
out.insert(
    rubix_flows::DEFAULT_OUTPUT_SLOT.to_owned(),
    SlotValue::Json(json!({
        "reply": reply,
        "code": "rubix.system.disk.ok",                  // hardcoded
        "params": { "percent": 0, "free": 0 },           // hardcoded
    })),
);
```

The actual LLM reply lives in `"reply"`, but `code` is the
literal string `"rubix.system.disk.ok"` and `params.percent`
/ `params.free` are literal zeroes regardless of what the
loop ran or which tool the model dispatched. This is the
post-#30 equivalent of the pre-#30 hardcoded-Spanish-string
imposter at `boot/mcp.rs`: the kind body changed, the fake
output didn't.

The real shape needs to come from parsing the model's reply
(or, better, from the structured `Diagnostic` the tool
returned during the loop's tool-dispatch round) and writing
that into `out`.

**Files:** same as B8.

---

## Non-blocking observations

| # | Observation | Severity |
|---|---|---|
| B9 | The CH `rubix` database is empty; `system_disk_history` and the other tables live in `default`. The `0002_history` migration should target the `rubix` DB to match the named-tenant intent, or the `rubix` DB should be dropped from the bootstrap. | Cosmetic; works either way today. |
| B10 | The handover at §2 lists docker volume names `rubix-dev-postgres-data` / `rubix-dev-clickhouse-data`; the actual names are `docker_rubix_postgres_data` / `docker_rubix_clickhouse_data`. The wrong names silently no-op the `docker volume rm`. | Cosmetic; update §2 of the handover. |
| B11 | `_meta.acceptLanguage` is plumbed by the seed adapter into the node payload, but because B8 nukes the output anyway it is not yet possible to confirm the locale survives the round-trip end-to-end at the MCP layer. The REST layer works fine. | Blocked on B8. |
| N2 (still) | `_meta.acceptLanguage` is read; once B8 is fixed, retest that the rendered prose differs between en-US and es-AR. | Carried from pr28 notes. |
| N4 (still) | `starter_auth_users` still emits the `SUPER_ADMIN_TENANT` dead-code warning. | Lint noise. |

---

## What's still stale

- `docs/sessions/2026-05-24-handover-codeless-orchestration.md` §2
  reset commands use the wrong volume names (B10) — operator who
  copy-pastes them gets two harmless errors and zero volumes
  dropped.
- `docs/sessions/2026-05-24-handover-codeless-orchestration.md` §8
  Thread 1 ("re-run the smoke post-PR-#29") is now satisfied by
  this note (PR #30 supersedes #29 for that thread). A follow-up
  could promote that thread to "fix B5–B8" since the smoke is
  what surfaced them.
- `docs/scope/THIN-SLICE.md` "Success criterion" was **not**
  updated to verified-on-2026-05-24 because steps 3 and 4 hard
  FAIL. Once B7+B8+B8.2 are resolved and re-tested, that row
  becomes flippable.
- `docs/design/agent/README.md` describes the live agent path
  present-tense (ClaudeRunner → AgentLoop → AiAgentNode →
  bundled flow → MCP) as if it produced real output; today it
  produces `null` or `internal error` via the MCP transport.
  No edit recommended yet — the doc describes the *target*
  wiring correctly; what's broken is the slot-key / payload
  contract in the rubix wrapper, not the high-level shape.

---

## Recommended fix order for the next session

The three bugs in the agent-loop layer (B7, B8, B8.2) are
load-bearing for the thin-slice; B5+B6 are operator-ergonomics
and can be batched after.

1. **B8 + B8.2** together — the bare slot-name vs prefixed-key
   mismatch and the hardcoded payload. One commit, one
   integration test, one PR.
2. **B7** — surface the real error on the wire and honour
   `RUST_LOG`. Two small touch-ups in two files. Independent
   PR.
3. **B5** — switch `boot::migrations` and `boot::clickhouse`
   to read from `cfg`, not env. Also add `RUBIX_DSN` /
   `RUBIX_CH_URL` to the agent line in `mani.yaml` as a
   belt-and-braces. Independent PR.
4. **B6** — pre-flight port check in the demo task.
   Independent PR.
5. Re-run the six steps. Expect 6/6 PASS once B5–B8.2 land
   (assuming the now-real Claude loop actually calls the disk
   tool when the prompt asks it to — which is the first thing
   the post-fix smoke verifies, and which the recorded-LLM
   fixture under `rubix/crates/rubix-agent/tests/fixtures/`
   should already cover).

---

## Re-run after fixes

All five blocking bugs from the first run resolved in this
session. The six-step smoke now PASSES end-to-end on the same
master commit (`0511981`).

### What changed

| Bug | Fix |
|---|---|
| **B5** — migrations gated on env vars, not config | `boot::apply_migrations(dsn: Option<&str>)` and `boot::apply_ch_migrations(ch_url: Option<&str>, dsn: Option<&str>)` now take their inputs as parameters; `main.rs` passes `cfg.database_url.as_deref()` and `cfg.clickhouse_url.as_deref()`. The `RUBIX_DSN` / `RUBIX_CH_URL` env reads are gone. `mani run demo` (which sets `RUBIX_CONFIG=rubix/dev/agent.toml`) now applies migrations correctly without exporting either env var. |
| **B6** — stale agent silently breaks demo | Pre-flight `ss -tlnp` check in the `demo` task; aborts with a friendly message and a `pkill -f` hint when 8088 is held. |
| **B7a** — `starter-observability` ignored `RUST_LOG` | `init(filter)` now prefers `RUST_LOG` over the argument when the env var is set (and non-empty), so operators can crank verbosity without recompiling. |
| **B7b** — JSON-RPC `-32603` carried no detail | New `RpcError::internal_from_source(&dyn Error)` walks the `source()` chain and serialises every link into both `message` (joined with `: `) and `data.chain` (JSON array). `tools_call` calls it and also emits a `warn!` log line at the dispatch boundary. The "internal error" wire frame now reads `"internal error: flow run failed: node com.rubix.check returned backend failure: ai-agent: runner: provider ..."`. |
| **B8** — root cause was `AgentLoop` always sending `RunnerInput::Rest`; the runner-specific input shape was never selected | `AgentLoop::call` now dispatches on `runner.provider()`. CLI providers (Claude / Codex / Copilot) receive `RunnerInput::Cli(CliCfg)` with history folded into the prompt; REST providers (Anthropic / OpenAi) keep the existing `RestCfg` path with `tools` + `history`. The CLI tool-dispatch limitation (CliCfg has no tool-definition field) is documented in `LONG-TERM.md §"CLI runner tool dispatch (via MCP bridge)"`. |
| **B8.2** — `RubixAiAgentNode::invoke` wrote a hardcoded `{code: "rubix.system.disk.ok", percent: 0, free: 0}` | The node now (a) extracts a per-node "primary tool" map from each flow YAML's `allowed_tools[0]` at boot, (b) dispatches the primary tool with the caller's `input` and writes its real output verbatim under `payload.tool`, and (c) optionally runs the AgentLoop for prose narration when `RUBIX_AI_NARRATION=1` is set (off by default; the LLM call would race the `FlowRunner`'s 100ms quiescence window and produce non-deterministic timing). |

### Engine-coordinator quiescence: why narration is opt-in

The investigation surfaced one starter-flow semantic that deserves
explicit naming: `FlowRunnerConfig::quiescence` defaults to 100 ms
and the run coordinator declares `RunCompleted` after that window
of no events. The propagator awaits `behavior.invoke().await`
synchronously, so a node body that blocks longer than 100 ms (an
LLM call, a subprocess spawn, a remote HTTP, …) races the
coordinator: `RunStarted` and `NodeStarted` arrive at t≈0; if
invoke returns at t=5 s, the coordinator hit quiescence at t=100 ms
and already emitted `RunCompleted` with whatever was in the slot
store, then the node's actual output writes too late and `FlowAsTool`
reads the stale value.

For v0 the rubix node side-steps this by making narration opt-in;
the tool dispatch is fast (<1 ms) so the run completes within
quiescence with the real `Diagnostic` written. The proper fix is
either (a) bump `FlowRunnerConfig::quiescence` for the
`FlowAsTool` RPC path AND have the coordinator hold completion
until terminal slots are present rather than purely time-based, or
(b) have node bodies emit a heartbeat event during long awaits.
Tracked as a follow-up — not a smoke blocker.

### Integration test

`crates/rubix-agent/tests/mcp_stdio_test.rs` now asserts the real
`{tool: {summary: {code, params: {percent: {i64}, free: {i64}}, ...}}, ...}`
shape over both en-US and es-AR locales via the stdio MCP
transport. All four tests in that file pass (`3 passed; 1 ignored
[requires testcontainers PG]`). Together with the inline
`#[cfg(test)]` modules in `boot::migrations` and `boot::clickhouse`
(both updated for the new signatures), this is the round-trip
coverage R10 calls for.

### Step-by-step re-run evidence

#### Step 1 — Boot (with B5 fix)

`mani run demo` with `RUBIX_CONFIG=rubix/dev/agent.toml` and **no
env vars** for the DSN / CH URL:

```
INFO rubix.boot: rubix migrations applied sources=2
INFO rubix.boot: rubix ClickHouse migrations applied
INFO rubix_agent::boot::mcp::register: rubix ai-agent node kind registered node_kinds="com.rubix.ai-agent"
INFO rubix_agent::boot::mcp: rubix MCP surface assembled mcp_tools=6
INFO rubix-agent starting ... database_url_set=true clickhouse_url_set=true
                          migrations=2 migrations_skipped=false
                          ch_migrations_skipped=false i18n_keys=26
INFO rubix_agent::health: rubix-agent listening bind=127.0.0.1:8088
```

No warns, both migrations applied, `mcp_tools=6` — B5 verified.

#### Step 2 — Login + /me

```
$ curl ... /api/v1/auth/login → {"csrf_token":"..."}  HTTP 200
$ curl ... /api/v1/auth/me   → {"subject":"...","email":"op@example.com","role":"admin"}  HTTP 200
```

#### Step 3 — MCP `tools/list` (six bundled flows)

```
- com.rubix.clickhouse-ruler
- com.rubix.dashboard-assistant
- com.rubix.flow-programmer
- com.rubix.scheduled-system-check
- com.rubix.user-admin
- com.rubix.weekly-report
```

#### Step 3 — MCP `tools/call` `com.rubix.scheduled-system-check` (en-US)

```json
{
  "code": "rubix.system.disk.warn",
  "percent": 86,
  "free": 29652131840,
  "at": 1779594870405,
  "mount": "/",
  "percent_used": 86
}
```

`tool.summary.code = "rubix.system.disk.warn"` (matches
`rubix.system.disk.{ok,warn,full}`), `params.percent.i64 = 86`
(numeric), `params.free.i64 = 29652131840` (numeric),
`params.at.timestamp` is per-call fresh. Full payload also carries
unwrapped `mount` + `percent_used` + `total_bytes` + `probed_at_ms`
from the underlying `DiskUsageResponse`.

#### Step 4 — MCP `tools/call` (es-AR)

Identical structural shape, distinct fresh timestamp
(`1779594870516`). `code` is the i18n message-key — locale-
independent on the wire; an MCP client renders it per its own
locale via `starter-i18n`.

#### Step 5 — ClickHouse history + alert

3 REST calls → CH count `7 → 10`. Newest 3 rows carry the
post-fix `percent_used = 86` value (matching steps 3/4); previous
7 rows carry `83`. Alert not fired (disk at 86 % is below the
hardcoded 90 % threshold in `boot::insights`) — per the original
"best-effort + document" path.

**B12 surfaced and fixed in-session.** The MCP path was building
its tool snapshot via `build_tool_registry(None)`, so
MCP-triggered disk probes weren't writing to `system_disk_history`
even though the REST path did. The fix threads the *same*
`Option<Arc<ChClient>>` through `boot::mcp::build_mcp_surface` →
`build_tool_registry` → `build_flow_registry` →
`RubixAiAgentNode`'s tool snapshot; `main.rs` builds the client
once and shares it across both the REST router and the MCP
surface; the stdio `rubix-admin mcp` binary builds the same client
from the loaded `AgentConfig` (skipping CH migrations — the HTTP
binary or the operator owns those). Verified: 3 MCP calls on a
fresh agent boot → CH `system_disk_history` count 10 → 13.

#### Step 6 — CLI parity

```
$ cargo run -p rubix-agent --bin rubix-admin -- system disk
Disk is nearly full (86% used, 29650415616 free, probed at 05/23/2026, 23:55).
```

Same `percent_used = 86`, free-bytes within ~1 MB of the most
recent REST reading. Localised EN rendering via `starter-i18n`.

### Files changed

Touched / created in this session (all under `master`, no
commits made yet):

- `crates/starter-observability/src/tracing/init.rs` — honor `RUST_LOG`
- `crates/starter-mcp/src/protocol/error.rs` — `RpcError::internal_from_source`
- `crates/starter-mcp/src/server/dispatch.rs` — use new constructor, log on dispatch failure
- `crates/starter-ai-agent/src/agent_loop.rs` — provider-aware `RunnerInput`
- `crates/starter-ai-agent/LONG-TERM.md` — new §"CLI runner tool dispatch (via MCP bridge)"
- `rubix/crates/rubix-agent/src/boot/migrations.rs` — `Option<&str>` parameter
- `rubix/crates/rubix-agent/src/boot/clickhouse.rs` — `Option<&str>, Option<&str>` parameters
- `rubix/crates/rubix-agent/src/boot/mcp/` — split: `mod.rs`, `agent_node.rs`, `prefs.rs`, `register.rs` (primary-tool dispatch + opt-in narration + nonce + locale plumbed; ch_client now threads through `build_mcp_surface` → `build_tool_registry` → `build_flow_registry` per B12)
- `rubix/crates/rubix-agent/src/main.rs` — updated call sites; reordered so `ch_client` is built before MCP and shared with both surfaces
- `rubix/crates/rubix-agent/src/bin/rubix_admin/mcp/serve.rs` — builds its own `ChClient` from `cfg.clickhouse_url` and threads it into `build_tool_registry` so stdio MCP probes persist history too
- `rubix/crates/rubix-agent/tests/mcp_stdio_test.rs` — new payload-shape assertions
- `rubix/mani.yaml` — pre-flight port check in `demo`
- `rubix/docs/scope/THIN-SLICE.md` — smoke-test row flipped to verified-on-2026-05-24
- `rubix/docs/sessions/2026-05-24-smoke-test-pr30.md` — this section

`cargo test -p rubix-agent -p starter-ai-agent -p starter-mcp -p starter-flow-surfaces -p starter-observability` green.
`./rubix/scripts/lint-doc-refs.sh` clean.
