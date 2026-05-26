# Thin-slice plan — one of everything, end-to-end

> **Tier:** plan, not system-as-it-is. Lives in `docs/scope/` per
> [HOW-TO-CODE.md §0a](../../HOW-TO-CODE.md). Source code must not
> reference this file — once a layer below lands, its design moves
> into `docs/design/<area>/README.md` and code links there.

## What this plan is

A **single demo path** that exercises every architectural layer of
rubix end-to-end with the smallest possible surface. The point is
to prove the arch works before broadening any one goal.

The active SCOPE phasing (Phases 1–5 in [SCOPE.md](../../SCOPE.md))
remains the long-term plan. This thin slice collapses Phases 1–4
into one working demo path; afterwards each SCOPE phase becomes
"broaden an already-working layer" rather than "add a missing
layer."

## The single demo path

```
MCP client (Claude Desktop)
   "What's the disk situation?"
        │
        ▼
rubix-agent binary
        │ session + authz gate
        ▼
starter-flow Engine ── runs ──► com.rubix.scheduled-system-check
        │                                  │
        │                                  ▼  ai-agent node
        │                          AiRunner (Claude CLI)
        │                                  │
        │                                  ▼  dispatches
        ▼                          rubix.system.disk tool
  Postgres                                  │
  (sessions, authz,                         │  canonical SI:
   audit, changelog)                        │  Quantity { 76.0, Length }
                                            │  Timestamp(epoch_ms)
                                            ▼
                            MessageBundle::render_diagnostic
                                            │  prefs.language = "es"
                                            │  prefs.timezone = "Europe/Paris"
                                            ▼
                        "El disco está al 76% libre (15/01/2024, 13:00)"
                                            │
                                            ▼  if > threshold
                                  starter-insights rule
                                            │
                                            ▼
                                   rubix.alert.send tool
                                            │  (log line for v0)
                                            ▼
                            ClickHouse (history row per check)
                                    audit + agent-log row
```

## Layers exercised

Exactly one of each. Nothing else.

| Layer | What lights up |
|---|---|
| Auth | `starter-auth-users` cookie session — one operator account |
| AuthZ | `starter-authz` policy gate on the MCP tool call; one tenant + one team |
| Audit | `starter-changelog` row per agent turn via `starter-agent-log` |
| Flow runtime | One bundled YAML, `trigger: explicit` |
| Agent | One skill (`com.rubix.system-checker`), one selector pick per run |
| MCP | `FlowAsTool` auto-surface; `render_diagnostic` for output |
| Tools | `rubix.system.disk`, `rubix.alert.send` |
| Prefs | `Quantity` (length) + `Timestamp` round-trip through pref-aware render |
| i18n | EN + ES catalogue entries for `rubix.system.disk.*` |
| Storage | Postgres (state + audit), ClickHouse (history row per check) |
| Insights | One rule: `disk_used > 90%` → fire `rubix.alert.send` |
| CLI | `rubix system disk` hits the same surface |

## What is deliberately out

| Cut | Why |
|---|---|
| User-admin tool | Multiplies auth surface; one read-only flow is enough |
| Dashboards (SDUI) | Frontend-adjacent; SCOPE locks "no frontend" |
| Flow programmer tool | Recursive concerns; deferred |
| `clickhouse.rule_write` tool | Use `starter-insights` directly with a hardcoded rule for v0 |
| Analytics reports | Needs `starter-export` + blob storage; defer |
| OAuth | Local user + password only; OAuth post-thin-slice |
| Cron scheduling | `trigger: explicit` is enough; cron is GAPS #16, deferred |
| Multi-tenant ClickHouse isolation | One tenant; isolation is Phase 4 entry gate |
| Undo / clipboard | Demo path is read-only |
| Extensions (`com.rubix.example`) | Depends on `starter-ext-flow` upstream item; **deferred** |

## Five PRs

Each PR is independently shippable. Each one lights up another
slice of the arch.

### PR 1 — disk tool, no auth, no MCP

Locks the canonical tool pattern (DTO + descriptor + dispatch +
test + recorded-LLM fixture) before anything else.

**Files touched:**
- `crates/rubix-spi/src/dto/system/disk.rs` — `DiskUsageRequest`,
  `DiskUsageResponse`, the static `ToolDescriptor` (five fields).
- `crates/rubix-spi/catalogues/{en,es}.json` — three keys:
  `rubix.system.disk.{ok,warn,full}`.
- `crates/rubix-tools/src/system/disk.rs` — dispatch only; uses
  `sysinfo` for the actual probe; returns
  `Diagnostic { summary, params }`.
- `crates/rubix-tools/tests/system_disk_test.rs` — drop the
  `#[ignore]`; round-trip through `ai-agent` via the recorded-LLM
  harness.
- `crates/rubix-skills/skills/system-checker/SKILL.md` —
  Localisation section telling the agent to emit `MessageKey` +
  `Quantity`.
- `crates/rubix-flows/flows/scheduled-system-check.yaml` —
  confirm it works as `trigger: explicit`.
- `crates/rubix-agent/src/main.rs` — wire the loader; register
  the tool; load the bundle.

**Exit signal.** `cargo test -p rubix-tools --test system_disk_test`
passes; `cargo run -p rubix-agent` boots and `tools=1 skills=6
flows=6` in the startup line.

### PR 2 — auth + authz + audit + Postgres

Now the disk tool requires a session and a permission; every call
writes a changelog row.

**Files touched:**
- `crates/rubix-agent/migrations/0001_init/{up,down}.sql` — the
  rubix migration scaffold (only what rubix owns; starter migrations
  run first per [docs/design/migrations/](../design/migrations/README.md)).
- `crates/rubix-agent/src/main.rs` — wire `starter-auth-users`
  (cookie sessions), `starter-authz` (one resource per tool, one
  policy), `starter-changelog-postgres`, `starter-agent-log`.
- New `docs/design/auth/README.md`, `docs/design/audit/README.md`,
  `docs/design/migrations/README.md` (placeholders → present-tense).
- One bootstrap operator created by a `mani run bootstrap-user`
  task (idempotent).

**Exit signal.** Unauthenticated REST call returns 401. The
disk-check tool requires the `system.read` permission. Every call
appears in `starter-changelog`.

### PR 3 — MCP exposure

`FlowAsTool` wires the flow; Claude Desktop sees the tool, calls
it, gets EN or ES output depending on `Accept-Language`.

**Files touched:**
- `crates/rubix-agent/src/main.rs` — add `starter-mcp` router;
  wire the `FlowRegistry` into it. MCP session locale comes from
  `Accept-Language` initial handshake.
- `crates/rubix-agent/tests/mcp_disk_test.rs` — round-trip via
  the MCP testing harness with `Accept-Language: es-AR`; assert
  Spanish output with EU date format and the right timezone.
- Update `docs/design/i18n-prefs/README.md` if the MCP locale
  handshake shape changed.

**Exit signal.** A real Claude Desktop instance connects, lists
tools, picks `com.rubix.scheduled-system-check`, calls it, sees
localised output.

### PR 4 — ClickHouse history + insights rule + alert

Disk-check writes a history row; an insights rule fires the alert
tool when the threshold is crossed.

**Files touched:**
- `crates/rubix-agent/migrations/0002_history/{up,down}.sql` —
  ClickHouse migration for the history table (one row per disk
  check: tenant_id, host, percent_used, free_bytes, epoch_ms).
- `crates/rubix-tools/src/system/disk.rs` — write a row after the
  probe.
- `crates/rubix-tools/src/system/alert_send.rs` — log-line
  implementation. Slack / Telegram / email defer.
- `crates/rubix-agent/src/main.rs` — register a hardcoded insights
  rule (`disk_used > 90%`); on fire, dispatch `rubix.alert.send`.
- New `docs/design/warehouse/README.md` and
  `docs/design/insights/README.md` (placeholders → present-tense).
  WAREHOUSE.md commits to per-row tenant column (cheapest of the
  three options).

**Exit signal.** Running the disk check 100 times produces 100
history rows. Manually inflating `percent_used` past 90% triggers
the alert tool, which writes a log line.

### PR 5 — extension contribution (DEFERRED)

Depends on the planned `starter-ext-flow` adapter (see
[docs/design/starter-changes/](../design/starter-changes/README.md)).
Once that lands, `com.rubix.example` contributes one tool that
appears in the shared `ToolRegistry` indistinguishably from
rubix-bundled tools.

**Why deferred:** upstream-first (R2) is more valuable than demo
completeness. PRs 1–4 prove the arch on their own.

## Order of work and dependencies

```
PR 1  ─── disk tool standalone
   │
   ▼
PR 2  ─── add auth + authz (consumes PR 1)
   │
   ▼
PR 3  ─── add MCP exposure (consumes PR 2)
   │
   ▼
PR 4  ─── add ClickHouse + insights + alert (consumes PR 3)
   │
   ▼
[PR 5  ── extension contribution, post starter-ext-flow]
```

No parallelism worth chasing — each PR consumes the previous one's
seams. Aim for one PR per week.

## After the thin slice lands

The five SCOPE phases stay; each one becomes "broaden a layer that
already works":

| SCOPE phase | After thin slice = |
|---|---|
| Phase 1 | Broaden `system` goal: add `system.db`, `system.flow_errors`. |
| Phase 2a | (already landed in PR 2 + 3) — broaden authz: add OAuth, more permission resources. |
| Phase 2b | Add gRPC + CLI on the same surface. CLI hits a tool that already works. |
| Phase 3 | Add user-admin goal, then dashboard goal, then flow-programmer goal. |
| Phase 4 | Broaden warehouse: cron triggering, `clickhouse.rule_write` tool, analytics reports. |
| Phase 5 | Extensions (PR 5 from this thin slice, plus broader contribution kinds). |

The thin slice doesn't replace the phases — it **front-loads one
end-to-end demo** so every later broaden-a-layer change has
something working to test against.

## Goals lit up beyond the thin slice

Branch `codeless/rubix-goals-2-4-3` broadened Phase 3 + Phase 4
incrementally on top of the landed thin slice; branch
`codeless/rubix-goal-6-weekly-report` then lit up Goal 6 plus the
durable cron scheduler primitive upstream; branch
`codeless/rubix-dashboards-goal-1` then lit up Goal 1 end-to-end.
All six SCOPE goals now light up end-to-end through the real
agent loop.

| Goal | Flow | State | Evidence / unblock |
|---|---|---|---|
| 1 — Dashboards | `com.rubix.dashboard-assistant` | **real** | Phase E — `dashboards_definitions` PG store + seven `rubix.dashboard.*` verbs (`get`/`list`/`create`/`update`/`duplicate`/`delete`/`page_set`) with undo + `<SduiPage>` renderer in new `@nube/starter-ui-sdui-react` + bundled `disk-overview` demo. End-to-end test `rubix-agent/tests/dashboard_crud_test.rs` + Playwright `rubix-frontend/e2e/dashboards.spec.ts`. See [`docs/design/sdui/`](../design/sdui/) + [`docs/sessions/2026-05-25-dashboards-goal-1-landed.md`](../sessions/2026-05-25-dashboards-goal-1-landed.md). |
| 2 — User admin | `com.rubix.user-admin` | **real** | Phase B — six verbs + undo + integration test `rubix-agent/tests/goal_2_user_admin_test.rs`. See `docs/design/user-admin/`. |
| 3 — Flow programmer | `com.rubix.flow-programmer` | **real** | Phase D — flow definitions live in PG (`flows_definitions` dimension table), cross-instance NOTIFY, `flow.{deploy,lint,list,duplicate}` + undo. See `docs/design/flow-programmer/`. |
| 4 — ClickHouse ruler | `com.rubix.clickhouse-ruler` | **real** | Phase C — `clickhouse.{rule.write,mart.create,retention.set}` + undo with snapshot rows in bounded `undo_snapshots`. See `docs/design/clickhouse-rules/`. |
| 5 — Scheduled system check | `com.rubix.scheduled-system-check` | **real** | Original thin-slice path. See `docs/scope/THIN-SLICE.md` smoke checklist. |
| 6 — Weekly report | `com.rubix.weekly-report` | **real** | Phase D — `analytics.query` (six named CH templates) + `analytics.report` (rendered via `starter-export`, persisted via `starter-blob-fs`) + durable cron scheduler upstream (`starter-cron` crate + `scheduled_flows` PG table + `FlowAsService`). End-to-end test `rubix-agent/tests/goal_6_weekly_report_test.rs` advances a `TestClock` by 7 days, asserts one fire, HTML blob lands, `rubix.undo.last` deletes it. See [`docs/design/reports/`](../design/reports/) + [`docs/design/scheduling/`](../design/scheduling/) + [`docs/sessions/2026-05-24-goal-6-landed.md`](../sessions/2026-05-24-goal-6-landed.md). |
| Frontend surfaces | `rubix/frontend` console (all left-nav sections live) | **real** | Branch `codeless/rubix-frontend-surfaces` — consumes already-built starter UI packages (`starter-ui-authz`, `starter-ui-flow`, `starter-ui-kit`) + adds the rubix-side `<WarehouseAdmin>` panels (rules · marts · retention · insights). Routes: `/admin/access` (8-tab `<AuthzAdmin>`), `/admin/users`, `/admin/warehouse`, `/flows` + `/flows/$flowId` (FlowCanvas with rubix-flavoured NodeKindRegistry override for `ai-agent`), `/extensions`, polished top-header (user + tenant + logout + theme toggle), empty states + loading skeletons, toast error listener pattern documented (mount pending starter-ui-kit `Toast` primitive — OQ-6). Smoke specs: `authz-admin.spec.ts`, `flows.spec.ts`, `warehouse.spec.ts`, `chrome.spec.ts`. See [`docs/design/frontend/`](../design/frontend/) + [`docs/sessions/2026-05-24-frontend-surfaces.md`](../sessions/2026-05-24-frontend-surfaces.md). |
| Live flow runtime | `com.rubix.tick-counter` (reference live demo) | **real** | Branch `codeless/rubix-flow-live-tick-demo` — upstream `NodeStateStore` trait + `InMemoryNodeStateStore` + `SqliteNodeStateStore` + `starter.flow.counter` node + present-tense rewrites of `DOCS/flow/scope/{hot-reload,settings,node-state}.md`; rubix-side always-on mounter `rubix-agent/src/boot/flow_runtime.rs` (every `starter.flow.trigger.schedule` node registers with `FlowAsService` from boot), SSE route `GET /api/v1/flows/{flow_id}/events` (15 s heartbeat, `FlowSubscriptionRegistry` fan-out), bundled `tick-counter.yaml` (cron `*/5 * * * * *` → counter → log), `flow_ops.list` carries `body_yaml`, new `flow_ops.kinds` exposes config schemas, `useFlowEvents` + settings sidebar on `/flows/$flowId` so the operator hot-edits step + cron without restarting the agent. End-to-end test `rubix-agent/tests/flow_live_tick_test.rs` boots the runtime under testcontainers PG + tempdir sqlite, asserts ≥3 ticks in 3 s, asserts hot-edit jumps +10, asserts the count survives a `FlowRunner` rebuild. See [`docs/design/flows/`](../design/flows/) §"Always-on flow runtime + live view" + [`docs/sessions/2026-05-25-flow-live-tick-demo-landed.md`](../sessions/2026-05-25-flow-live-tick-demo-landed.md). |
| Extensions wired | `com.rubix.example` (reference bundle) | **real** | Branch `codeless/rubix-extensions-wire` — `rubix-agent` boot composes `starter-ext-host` + `starter-ext-supervisor` + `starter-ext-server` + new upstream `starter-ext-store-pg` (PG-backed `EnablementStore`) under `/api/v1/extensions/*` with admin authz; `rubix/extensions/` is its own cargo workspace; `com.rubix.example` builds, loads, supervises, contributes `com.rubix.example.echo` to `tools/list`, and renders a federated UI panel into `packages/test-ui-5`'s `/extensions` route. Integration test `rubix-agent/tests/extensions_lifecycle_test.rs` drives the eight-step lifecycle (list/start/stop/restart/disable/enable/PG-row/SSE) under testcontainers PG. See [`docs/design/extensions/`](../design/extensions/) + [`docs/sessions/2026-05-24-extensions-wired.md`](../sessions/2026-05-24-extensions-wired.md). |

Every "real" goal above writes are reversible through
`rubix.undo.last`, audits a row to `changelog`, and round-trips
the `Diagnostic` through the EN/ES catalogues. Per-goal
integration tests live in `rubix-agent/tests/goal_{2,3,4}_*.rs`;
the closing smoke session note is
[`docs/sessions/2026-05-24-goals-2-4-3-landed.md`](../sessions/2026-05-24-goals-2-4-3-landed.md).

## Open questions specific to the thin slice

| # | Question | Resolves before |
|---|---|---|
| T1 | Recorded-LLM harness shape — record-and-replay vs deterministic stub | PR 1 |
| T2 | Bootstrap operator — first-run claim vs seeded credentials | PR 2 |
| T3 | ClickHouse migration tooling — `starter-store-warehouse` provides a runner? | PR 4 |
| T4 | Insights rule wire format — hardcoded Rust vs YAML on disk | PR 4 |

T1 + T2 + T3 + T4 need an answer before their PR starts; surface
to the user as the PR opens.

## Success criterion for the thin slice

When PR 4 ships, **a single curl + claude-desktop invocation** can
demonstrate every layer:

```bash
# 1. Boot
mani run run

# 2. Log in as the bootstrap operator
curl -c cookies.txt -X POST http://127.0.0.1:8088/api/v1/auth/login \
     -d '{"email":"op@example.com","password":"..."}'

# 3. Call the disk-check via REST with Spanish + Paris prefs
curl -b cookies.txt -H "Accept-Language: es-AR" \
     http://127.0.0.1:8088/api/v1/tools/rubix.system.disk
# → JSON with Diagnostic { code, params: { percent, free }, message_en }

# 4. Connect Claude Desktop to the MCP endpoint
# → "What's the disk situation?" returns Spanish prose with EU date format

# 5. Inspect the audit trail
psql -c "SELECT actor, action, kind FROM changelog ORDER BY at DESC LIMIT 5"

# 6. Inspect the history
clickhouse-client -q "SELECT * FROM system_disk_history ORDER BY at DESC LIMIT 5"
```

If all six steps work, the arch is real.

### Status

**Last verified: 2026-05-24** on master commit `0511981` (PR #30
merged). Six of six steps PASS end-to-end after the B5–B8.2
fix-up landed; see
[`docs/sessions/2026-05-24-smoke-test-pr30.md`](../sessions/2026-05-24-smoke-test-pr30.md)
§"Re-run after fixes" for the evidence and the fix list.

Re-run when any of the load-bearing pieces touches the boot
order, the slot/seed contract, the `AiAgentNode` body, the
run-coordinator quiescence/in-flight-tracker model, the
JSON-RPC error mapping, or the cfg/env split for migrations.

The starter-flow run coordinator now holds completion until no
nodes are in flight — the `slow_node_body_does_not_race_quiescence`
test in `crates/starter-flow/src/run.rs` is the canary. Rubix
narration is on by default; opt out with `RUBIX_AI_NARRATION=0`.
