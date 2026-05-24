# Starter changes — the upstream PR ledger (index)

This doc is how rubix's [R2](../../SCOPE.md#r2--upstream-first-rubix-specific-stays-in-rubix-reusable-goes-to-starter)
("upstream first") becomes a deliverable, not a slogan. Every
starter capability rubix needs that doesn't yet exist is listed
here, ordered by which rubix phase blocks on it.

The ledger is split per phase. This README is the canonical entry
point; each phase file is the source of truth for the items gating
that phase.

## How this doc is used

- **Before a phase starts:** read the phase file linked below.
  Each item that isn't merged yet must have a draft PR or a filed
  issue with rationale.
- **During a phase:** when rubix code starts to look like a
  re-implementation of a starter capability, file the upstream
  issue *first*, link it into the phase file, then either wait for
  review or ship a temporary rubix impl with the issue link in a
  `TODO(upstream: <issue>)` comment.
- **At phase exit:** the phase file lists every upstream PR filed
  during the phase (merged, in review, or filed-with-rationale).
  A phase with zero PRs is a smell — the reviewer asks "what
  didn't get upstreamed and why?"

This ledger lives in `rubix/` because it is rubix's planning
artifact. The actual code changes ship in starter. Linking is by
PR / issue URL.

## Format for each item

```
### <short title>
- **Crate(s):** starter-foo, starter-bar
- **Blocks rubix phase:** N
- **Why upstream:** one sentence on who else benefits
- **Status:** planned | issue-filed (#NNN) | pr-open (#NNN) | merged (vX.Y.Z)
- **Notes:** any rationale, alternatives considered, or rubix
  fallback if the PR slips
```

## Phases

| Phase | File | Summary |
|---|---|---|
| Phase 1 | [phase-1.md](./phase-1.md) | i18n render API, `DiagnosticParam::Quantity`, `render_diagnostic`, timezone-aware timestamps, `starter-tool-sysdiag`, recorded-LLM harness, `starter-ai-agent` (runner-agnostic `AgentLoop` primitive, **landed in-tree**), `starter-flow-node-loop` (**landed in-tree**), skills parser, MCP prompts/resources, typed agent events. |
| Phase 2a | [phase-2a.md](./phase-2a.md) | `starter-auth-users` Postgres store impls. **Complete.** |
| Phase 2b | [phase-2b.md](./phase-2b.md) | `starter-mcp` Accept-Language plumbing (U1), real `InMemoryTransport` (U2), `starter-flow-surfaces` `FlowRegistry::resolve` + `FlowAsTool::from_registry` (U3). **All complete.** |
| Phase 2c | [phase-2c.md](./phase-2c.md) | gRPC/CLI rough edges; `starter-i18n` interpolate feature-gate mismatch (latent). |
| Phase 3 | [phase-3.md](./phase-3.md) | `starter-tool-sdui` page-builder primitives, `starter-tool-flow-ops`. |
| Phase 4 | [phase-4.md](./phase-4.md) | `cron-schedule` node kind, `starter-tool-clickhouse`, `clickhouse-query` node kind. |
| Phase 5 | [phase-5.md](./phase-5.md) | `starter-ext-flow` adapter, extension-author ergonomics. |

## Items filed (rolling log)

When a PR or issue is opened against starter on rubix's behalf,
append it here with date + link:

```
- YYYY-MM-DD  [#NNN](https://github.com/.../pull/NNN)  short title
```

### `starter-cron` — 5/6/7-field cron grammar + `next_fire`

- **Crate:** `starter-cron` (new)
- **Blocks rubix phase:** 4 (Goal 6 weekly-report — needed the
  scheduler to accept the bundled `0 8 * * 1` cron without the
  previous 5-field-only parser rejecting it during boot seeding).
- **Why upstream:** every starter consumer that wants scheduled
  flows needs the same grammar; baking cron parsing into
  `starter-flow-surfaces` would have coupled timing to the
  service surface forever.
- **Status:** merged on branch `codeless/rubix-goal-6-weekly-report`
  (Phase A.1 commit `5dac211`-era).
- **Notes:** answers SCOPE OQ-1 — the crate stayed standalone
  rather than folding into `starter-flow-spi`, because a future
  `starter-cron-cli` wants the same parser without pulling in
  flow types.

### `starter-store-postgres` — `scheduled_flows` migration + `pg_notify` trigger

- **Crate:** `starter-store-postgres`
- **Blocks rubix phase:** 4 (Goal 6 — durable schedule table is
  the only authority on "due now"; in-process timers were the
  Phase D.0 prototype and lost their next-fire on restart).
- **Why upstream:** every starter consumer hosting cron-style
  flows needs the same row shape and the same NOTIFY channel for
  cross-process reactivity.
- **Status:** merged on branch `codeless/rubix-goal-6-weekly-report`
  (Phase A.2 commit `5dac211`); migration file
  `migrations/scheduled_flows/0001_init.sql` plus the
  `starter_scheduled_flows` NOTIFY trigger on insert and on
  update of `next_run_at` / `enabled`.
- **Notes:** `UNIQUE (tenant_id, flow_id)` enforces one schedule
  per flow per tenant; reseed-on-boot uses
  `ON CONFLICT … DO UPDATE` so the YAML stays the source of truth
  for the cron expression.

### `starter-flow-surfaces::FlowAsService` — register / tick / Clock

- **Crate:** `starter-flow-surfaces`
- **Blocks rubix phase:** 4 (Goal 6 — closes GAPS row 16 by naming
  `FlowAsService` and giving it a body).
- **Why upstream:** the service surface is the dual of `FlowAsTool`;
  any starter consumer with bundled flows benefits from cron
  triggers without re-implementing the lease + tick loop.
- **Status:** merged on branch `codeless/rubix-goal-6-weekly-report`
  (Phase B.1 + B.2 commits `68b60c6` → `d94f147`); covers `Clock`
  (`SystemClock` + `TestClock`), `register_schedule` /
  `unregister_schedule`, `tick()` (SELECT FOR UPDATE SKIP LOCKED
  LIMIT 32), and `start(self) -> JoinHandle<()>` spawning the
  tick loop.
- **Notes:** rubix-agent wires this at boot via the
  `[scheduler]` config section; design lives at
  [`docs/design/scheduling/`](../scheduling/README.md). The
  phase-4 file's `cron-schedule` node-kind item is now
  superseded — the Service surface owns the trigger directly, no
  new node kind needed.

(Phase 1–3 items pre-date the rolling log.)
