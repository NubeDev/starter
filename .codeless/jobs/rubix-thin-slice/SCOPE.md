# Scope — rubix-thin-slice

The authoritative design lives at
[`/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md`](/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md).
This brief is the trimmed per-job scope. Where this disagrees with
the source SCOPE, **the source SCOPE wins** — fix this file
rather than diverge.

## Goal

Take the rubix thin slice from "disk tool works in isolation" to
"the six-step manual smoke in THIN-SLICE.md §Success criterion
passes end to end." Five blocks delivered as five PRs against
the `codeless/rubix-thin-slice` branch. After this job:

1. Three Postgres stores (`PgSessionStore`, `PgTokenStore`,
   `PgTenantStore`) ship upstream in `starter/`, completing the
   `starter-auth-users` Postgres surface.
2. `rubix-agent` boots with cookie sessions, authz per tool,
   audit + agent-log rows, and idempotent first-run claim.
3. The bundled flow `com.rubix.scheduled-system-check`
   auto-surfaces as an MCP tool; calling it with `LANG=es-AR`
   returns Spanish prose with EU date format.
4. Each disk-check writes a row to a multi-tenant ClickHouse
   history table; an in-process insights check fires
   `rubix.system.alert_send` when `percent_used > 90`.
5. The same disk verb is reachable via REST (with
   `Accept-Language`) and via `rubix-admin system disk` on the
   CLI, all hitting the same `probe()` function.

## What is already answered (do not re-litigate)

| # | Question | Answer locked |
|---|---|---|
| T1 | Recorded-LLM harness shape | Record-and-replay; landed in `starter-server::testing` (see latest session handoff). |
| T2 | Bootstrap operator | First-run claim flow via `rubix-admin bootstrap-user` (idempotent). |
| T3 | ClickHouse migration runner | `starter-store-clickhouse::MigrationRunner` exists; use it. The `clickhouse` Rust crate is the official `github.com/ClickHouse/clickhouse-rs` v0.13 and is **already pulled transitively through `starter-store-clickhouse`** — do NOT add a direct `clickhouse` dep to any rubix crate. |
| T4 | Insights rule wire format | **One hardcoded Rust rule** for the thin slice (`disk_used > 90`). No YAML loader, no rhai feature. A `// TODO(upstream: rule.rhai migration)` comment marks the promotion point. |
| T5 | Postgres stores: bundle vs split | **One bundled PR** for Session + Token + Tenant. Saves three review cycles. |
| Q6 | `rubix-client` justification | **Defer.** Do NOT touch `rubix-client`. CLI hits `probe()` in-process; if a REST integration test needs an HTTP client it uses `reqwest` directly. |
| Locale on MCP | How MCP gets caller locale | Read `LANG` from the rubix-agent process env at startup. Per-session, fixed. Matches CLI behaviour; works with Claude Desktop today. No custom MCP `initialize` extension. |

## In scope (five blocks mirroring the THIN-SLICE PRs)

- **Block 1 (stage 1) — upstream Postgres stores (bundled PR).**
  Port `SqliteSessionStore`, `SqliteTokenStore`, `SqliteTenantStore`
  to Postgres in `starter/crates/starter-auth-users/`. Exemplar:
  the already-landed `PgUserStore`. One PR for all three.

- **Block 2 (stage 3) — rubix bootstrap + auth + authz + Postgres
  migrations.** Rubix-side wiring of `starter-auth-users`,
  `starter-authz`, `starter-changelog-postgres`, `starter-agent-log`.
  First-run claim via `rubix-admin bootstrap-user`. New design
  docs `auth/`, `audit/`, `migrations/` rewritten present-tense.

- **Block 3 (stage 5) — MCP exposure with EN/ES round-trip.**
  Wire `starter-mcp` into `rubix-agent`; `FlowAsTool` exposes the
  bundled flow. Locale fixed per session from `LANG`.
  `tests/mcp_disk_test.rs` asserts EN and ES round-trips.

- **Block 4 (stage 7) — ClickHouse history + insights rule + alert.**
  Migration 0002 for `system_disk_history` with per-row
  `tenant_id` (cheapest of the three multi-tenant options,
  committed in `WAREHOUSE.md`). Hardcoded threshold check.
  `rubix.system.alert_send` log-line tool. New design docs
  `warehouse/`, `insights/` rewritten present-tense.

- **Block 5 (stage 9) — REST + CLI parity.** Per-tool REST
  handler (≤20 lines, extract → `probe()` → DTO → return).
  `rubix-admin system disk [--json]` calls `probe()` in-process
  and renders server-side via `MessageBundle::render_diagnostic`.

Each work block is followed by a REVIEW stage that gates the
next block.

## Out of scope (explicit carve-outs)

The source SCOPE deliberately excludes these. They are NOT
deferred-but-coming; they are NOT in this job under any
circumstance.

- **Extensions** (THIN-SLICE PR 5). Depends on `starter-ext-flow`
  upstream — separate job.
- **Cron triggering** on the bundled flow. GAPS #16.
- **OAuth.** Local password only.
- **Dashboards / flow programmer / analytics reports / user-admin
  tools / `clickhouse.rule_write` tool.** Per-goal broadening is
  post-thin-slice.
- **gRPC.** Not in the thin slice; REST + MCP + CLI is enough.
- **`rubix-client` crate.** Q6 deferred; stays as a stub.
- **New `starter-tool-*` crates.** Tools stay in `rubix-tools` for
  this job. Upstream promotion is a separate job once the thin
  slice is green.
- **`rule.rhai` / `rule.sql` / YAML insights loader.** T4 locked
  to one hardcoded Rust rule.
- **Cancel-mid-MCP / supervisor restart paths.** No streaming
  nodes in the thin slice.
- **Touching `rubix-old/`.** Read for archaeological context only;
  never copy code without flagging.

## Acceptance — when each block is "done"

### Block 1 — Postgres stores
- `cargo build` green in `starter/`.
- `cargo test -p starter-auth-users` green: every sqlite test
  name passes against the Postgres variants with identical
  behaviour.
- `cargo clippy --workspace -- -D warnings` clean.
- Each line diverging from the sqlite source carries a one-line
  comment explaining why.

### Block 2 — bootstrap + auth
- Migrations 0001 apply cleanly on a fresh Postgres.
- Unauthenticated REST call to a tool returns 401.
- `rubix-admin bootstrap-user --email op@example.com` is
  idempotent (second run is a no-op + 0 exit).
- Every tool call appears in `starter-changelog`; every
  `agent.turn.start` produces a row in `starter-agent-log`
  carrying the active skill id.
- `docs/design/{auth,audit,migrations}/README.md` rewritten from
  placeholder to present-tense, describing the system as it now
  exists.

### Block 3 — MCP
- `cargo test -p rubix-agent --test mcp_disk_test` passes for
  both `LANG=en-US` and `LANG=es-AR`.
- The bundled flow appears in the MCP tool catalogue with **zero
  per-flow MCP wiring code**. `FlowAsTool` does the work.
- `docs/design/i18n-prefs/README.md` updated: the locale source
  list lists `LANG` env var as the MCP path (not "MCP-session
  Accept-Language initial handshake" — that was speculative).

### Block 4 — ClickHouse + insights + alert
- Migration 0002 (ClickHouse) applies via
  `starter-store-clickhouse::MigrationRunner` on a fresh CH.
- A `probe()` call writes one row into `system_disk_history` with
  the right `tenant_id`.
- A test that inflates `percent_used > 90` triggers
  `rubix.system.alert_send` exactly once; below threshold, zero
  times.
- `docs/design/{warehouse,insights}/README.md` rewritten present-
  tense.

### Block 5 — REST + CLI
- `cargo test -p rubix-agent --test rest_disk_test` passes for
  both `Accept-Language: en-US` and `Accept-Language: es-AR`.
- `rubix-admin system disk` renders server-side (`LANG`-driven);
  `rubix-admin system disk --json` dumps the raw `Diagnostic` +
  data.
- The REST handler is ≤20 lines. If it grew, it's leaked domain
  logic — push back to `probe()`.

## Hard rules (subset that bites in this job)

All from rubix `HOW-TO-CODE.md`, `FILE-LAYOUT.md`, `SCOPE.md`.

- **One verb per file**, ≤400 lines hard, ~100 typical. No
  `utils.rs` / `helpers.rs` / `common.rs` / `misc.rs`. `mod.rs`
  is a barrel only.
- **Doc-tier rule.** Code comments reference
  `docs/design/<area>/README.md` only. Never `SCOPE.md`,
  `HOW-TO-CODE.md`, `NEW-SESSION.md`, `FILE-LAYOUT.md`,
  `docs/scope/`, or `docs/sessions/`. `mani run lint-doc-refs`
  enforces this; running it is not required of the codeless
  agent, but a fresh grep against the forbidden patterns must
  return clean before any block closes.
- **No phasing markers** in code: no `// Phase 0`, `// STAGE-1
  done`, `// FIXED:`, `// Previously this used X`.
- **No emojis, no ASCII banners.** `// TODO(name): ...` or
  `// TODO(upstream: <issue>): ...`. Never bare TODOs.
- **Tool outputs are `Diagnostic` + structured data**, never
  pre-formatted strings. `Quantity`-typed params carry
  `(canonical: f64, quantity: Quantity)`. `Timestamp` is epoch
  ms UTC. Renderer formats at the edge.
- **Catalogue files are the source of truth** for `MessageKey`
  entries. Adding a key in Rust without matching entries in
  `crates/rubix-spi/catalogues/en.json` AND `es.json` fails
  review.
- **Skill bodies + tool descriptors stay EN canonical.** Never
  translate.
- **Layer separation.** Transport handlers ≤20 lines; the gRPC-
  swap smoke test ("if I swap REST for gRPC tomorrow, does this
  file change beyond route wiring + DTO shaping?") must pass for
  every REST handler.
- **Tests live with the code in the same PR.** Unit tests inline
  as `#[cfg(test)] mod tests { ... }`. Integration tests under
  `tests/` mirroring source paths. No live LLM in CI — use the
  recorded-LLM harness from `starter-server::testing`.

## When codeless gets stuck

Codeless cannot ask the human. So the escape hatch is:

1. **Stop work on the current block immediately.**
2. **Open the PR anyway** with whatever code does compile.
3. **Add `BLOCKED: <one-line question>`** to the PR description,
   followed by a paragraph explaining what was tried and why it
   didn't match the spec.
4. **Move to the next block only if it does not depend on the
   blocked one.** Otherwise stop and wait.

The human reviews the blocked PR and answers. Codeless does not
guess to unblock itself.

## References

- Source SCOPE (authoritative):
  [`/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md`](/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md)
- Rubix architecture:
  [`/home/user/code/rust/starter/rubix/SCOPE.md`](/home/user/code/rust/starter/rubix/SCOPE.md)
- Contributor entry point:
  [`/home/user/code/rust/starter/rubix/HOW-TO-CODE.md`](/home/user/code/rust/starter/rubix/HOW-TO-CODE.md)
- File-layout rules:
  [`/home/user/code/rust/starter/rubix/FILE-LAYOUT.md`](/home/user/code/rust/starter/rubix/FILE-LAYOUT.md)
- Session boot:
  [`/home/user/code/rust/starter/rubix/NEW-SESSION.md`](/home/user/code/rust/starter/rubix/NEW-SESSION.md)
- Latest session handoff:
  `/home/user/code/rust/starter/rubix/docs/sessions/2026-05-23-next-steps-6.md`
- Forward gaps (read for context, not for scope drift):
  [`/home/user/code/rust/starter/rubix/docs/scope/GAPS.md`](/home/user/code/rust/starter/rubix/docs/scope/GAPS.md)
- Upstream PR ledger:
  [`/home/user/code/rust/starter/rubix/docs/design/starter-changes/README.md`](/home/user/code/rust/starter/rubix/docs/design/starter-changes/README.md)
- Exemplars to copy religiously:
  - `PgUserStore`: `/home/user/code/rust/starter/crates/starter-auth-users/src/store/postgres.rs` (or equivalent)
  - Tool dispatch: `/home/user/code/rust/starter/rubix/crates/rubix-tools/src/system/disk.rs`
  - DTO + descriptor: `/home/user/code/rust/starter/rubix/crates/rubix-spi/src/dto/system/disk.rs`
  - Bundled flow: `/home/user/code/rust/starter/rubix/crates/rubix-flows/flows/scheduled-system-check.yaml`
  - SKILL.md: `/home/user/code/rust/starter/rubix/crates/rubix-skills/skills/system-checker/SKILL.md`
