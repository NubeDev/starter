# Scope — rubix-thin-slice (v2: PR 3 → PR 4 → PR 5 only)

The authoritative design lives at
[`/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md`](/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md).
Latest state lives in
[`/home/user/code/rust/starter/rubix/docs/sessions/2026-05-23-next-steps-7.md`](/home/user/code/rust/starter/rubix/docs/sessions/2026-05-23-next-steps-7.md).
Where this disagrees with either source, **the source wins** — fix
this file rather than diverge.

## Goal

Take the rubix thin slice from "PR 1, PR 2 (both parts), and Path B
all green; U1+U2+U3 upstream blockers cleared" to "the six-step
manual smoke in THIN-SLICE.md §Success criterion passes end to end."

Three remaining PRs against the `codeless/rubix-thin-slice-v2`
branch:

- **PR 3 — MCP exposure** (the unblocked next active target).
- **PR 4 — ClickHouse history + insights rule + alert.**
- **PR 5 — REST + CLI parity (the final smoke seam).**

After this job the human runs the THIN-SLICE.md success-criterion
smoke and confirms every architectural layer is live.

## What is already landed (do not redo)

These are on master at `ad393ba` or earlier. Re-doing them creates
conflicts.

| # | Already on master | Commit |
|---|---|---|
| **PR 1** | Disk tool standalone | landed in -5 |
| **PR 2 part 1** | Per-verb permission constants + integration test stub | landed in -5 |
| **PR 2 part 2 / Path B** | Bootstrap-user + auth + authz + Postgres migrations | `5083d87` |
| **Phase 2a** | All three Postgres stores (Session, Token, Tenant) | `51e3ed8` |
| **U1** | `starter-mcp` Accept-Language → `current_locale()` task-local | `4c15dcb` |
| **U2** | `starter-mcp` real `InMemoryTransport` (replaces `_private: ()` stub) | `9ab273d` |
| **U3** | `starter-flow-surfaces` `FlowRegistry::resolve` + `FlowAsTool::from_registry` | `7216d78` |

## What is already answered (do not re-litigate)

| # | Question | Answer locked |
|---|---|---|
| T1 | Recorded-LLM harness shape | Record-and-replay; landed in `starter-server::testing`. |
| T2 | Bootstrap operator | First-run claim via `rubix-admin bootstrap-user` (idempotent). Already landed in Path B. |
| T3 | ClickHouse migration runner | `starter-store-clickhouse::MigrationRunner` exists; use it. The `clickhouse` Rust crate is the official `github.com/ClickHouse/clickhouse-rs` v0.13 and is **already pulled transitively through `starter-store-clickhouse`** — do NOT add a direct `clickhouse` dep to any rubix crate. |
| T4 | Insights rule wire format | **One hardcoded Rust rule** for the thin slice (`disk_used > 90`). No YAML loader, no rhai feature. A `// TODO(upstream: rule.rhai migration)` comment marks the promotion point. |
| T5 | Postgres stores: bundle vs split | ✅ Resolved — three separate upstream PRs was the right call; all landed. Not applicable to this job. |
| Q6 | `rubix-client` justification | **Defer.** Do NOT touch `rubix-client`. CLI hits `probe()` in-process; REST integration tests use `reqwest` directly if needed. |
| MCP locale | How MCP gets caller locale | **`starter-mcp` binds a task-local at dispatch time**; `current_locale()` reads it; `_meta.acceptLanguage` is the on-the-wire convention. This is the U1 contract. Do not invent a different mechanism. |
| MCP `FlowAsTool` wiring | How to expose a flow | `let tool = FlowAsTool::from_registry(&registry, &flow_id, &rev, engine).await?;` — that's the contract from U3. Do not hand-roll a `FlowAsTool` builder. |

Anything else that needs a real decision: `BLOCKED:` escape hatch
in §"When codeless gets stuck" below.

## In scope (three blocks mirroring the remaining THIN-SLICE PRs)

- **Block 1 (stage 1) — MCP exposure with EN/ES round-trip (PR 3).**
  Mount `starter-mcp`'s `mcp_router` against a `FlowRegistry`
  holding `com.rubix.scheduled-system-check`. Use
  `FlowAsTool::from_registry` (one-line wiring). Round-trip test
  via the new `InMemoryTransport` (U2) with two locales.

- **Block 2 (stage 3) — ClickHouse history + insights rule + alert
  (PR 4).** Migration 0002 for `system_disk_history` with per-row
  `tenant_id`. Hardcoded `disk_used > 90` check dispatches the
  `alert_send` tool (log-line implementation only). Rewrite
  `docs/design/warehouse/README.md` and `insights/README.md` from
  placeholder to present-tense.

- **Block 3 (stage 5) — REST + CLI parity (PR 5).** Per-tool REST
  handler (≤20 lines: extract → `probe()` → DTO → return).
  `rubix-admin system disk [--json]` calls `probe()` in-process
  and renders server-side via `MessageBundle::render_diagnostic`.

Each work block is followed by a REVIEW stage that gates the next
block.

## Out of scope (explicit carve-outs)

- **Re-doing Blocks 1–2 of the original spec.** PR 1, PR 2 (both
  parts), Path B, and Phase 2a are already on master at `ad393ba`.
- **Extensions (THIN-SLICE PR 5 from the original spec — the
  extension version).** Depends on `starter-ext-flow` upstream;
  separate job.
- **Cron triggering** on the bundled flow. GAPS #16.
- **OAuth.** Local password only.
- **Dashboards / flow programmer / analytics reports / user-admin
  tools / `clickhouse.rule_write` tool.** Per-goal broadening is
  post-thin-slice.
- **gRPC.** Not in the thin slice.
- **`rubix-client` crate.** Q6 deferred; stays as a stub.
- **New `starter-tool-*` crates.** Tools stay in `rubix-tools` for
  this job. Upstream promotion is a separate job once the thin
  slice is green.
- **`rule.rhai` / `rule.sql` / YAML insights loader.** T4 locked
  to one hardcoded Rust rule.
- **Removing the dev-dep pin in `starter-mcp/Cargo.toml:37-42`.**
  That's the workaround for the `starter-i18n` interpolate latent
  bug (commit `f7b69fd`). Remove when the upstream fix lands; not
  in this job.
- **Touching `rubix-old/`.** Read for archaeological context only;
  never copy code without flagging.

## Acceptance — when each block is "done"

### Block 1 — MCP exposure
- `cargo test -p rubix-agent --test mcp_disk_test` passes for both
  `_meta.acceptLanguage: en-US` and `es-AR`.
- The bundled flow `com.rubix.scheduled-system-check` appears in
  the MCP tool catalogue with **zero per-flow MCP wiring code** —
  `FlowAsTool::from_registry` does the work.
- `docs/design/i18n-prefs/README.md` updated only if the MCP
  locale handshake shape needs to be reconciled with U1's
  `_meta.acceptLanguage` convention (it does — the doc currently
  speculates about "Accept-Language initial handshake header";
  replace with the actual U1 mechanism).
- The dev-dep pin in `starter-mcp/Cargo.toml:37-42` is left
  alone (out-of-scope per §"Out of scope").

### Block 2 — ClickHouse + insights + alert
- Migration 0002 (ClickHouse) applies via
  `starter-store-clickhouse::MigrationRunner` on a fresh CH.
- A `probe()` call writes one row into `system_disk_history` with
  the right `tenant_id` (per-row column, the cheapest of the
  three options enumerated in WAREHOUSE.md).
- A test that inflates `percent_used` past 90 fires
  `rubix.system.alert_send` exactly once; below threshold, zero
  times.
- The exact comment `// TODO(upstream: rule.rhai migration) —
  promote to starter-insights::RuleRegistry once a second rule
  appears.` precedes the hardcoded check.
- `docs/design/warehouse/README.md` and `insights/README.md`
  rewritten present-tense.
- **No direct `clickhouse` dep** on any rubix crate (`cargo tree
  -p rubix-agent --invert clickhouse` shows only the transitive
  path through `starter-store-clickhouse`).

### Block 3 — REST + CLI
- `cargo test -p rubix-agent --test rest_disk_test` passes for
  both `Accept-Language: en-US` and `Accept-Language: es-AR`.
- `rubix-admin system disk` renders server-side under
  `LANG=en_US.UTF-8` (English) and `LANG=es_AR.UTF-8` (Spanish).
- `rubix-admin system disk --json` dumps the raw `Diagnostic` +
  data; no rendered string.
- The REST handler is ≤20 lines (`wc -l` it). If it grew, domain
  logic leaked — push it back into `probe()`.
- The CLI binary never opens a TCP connection to itself (verified
  via the test harness; no HTTP client constructed).
- Q6 unchanged — `rubix-client` untouched.

## Hard rules (subset that bites this job)

All from rubix `HOW-TO-CODE.md`, `FILE-LAYOUT.md`, `SCOPE.md`.

- **One verb per file**, ≤400 lines hard, ~100 typical. `mod.rs`
  is a barrel only; no `utils.rs` / `helpers.rs` / `common.rs` /
  `misc.rs`.
- **Doc-tier rule.** Code comments reference
  `docs/design/<area>/README.md` only. Never `SCOPE.md`,
  `HOW-TO-CODE.md`, `NEW-SESSION.md`, `FILE-LAYOUT.md`,
  `docs/scope/`, or `docs/sessions/`.
  `./rubix/scripts/lint-doc-refs.sh` enforces this — run it
  before considering a stage done.
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
- **Layer separation.** REST handlers ≤20 lines; gRPC-swap smoke
  test passes for every handler.
- **Tests live with the code in the same PR.** Unit tests inline.
  Integration tests under `tests/` mirroring source paths. No
  live LLM in CI — use the recorded-LLM harness from
  `starter-server::testing`.

## When codeless gets stuck

Codeless cannot ask the human. So the escape hatch is:

1. Stop work on the current block immediately.
2. Open the PR anyway with whatever code does compile.
3. Add `BLOCKED: <one-line question>` to the PR description plus
   a paragraph explaining what was tried and why it didn't match
   the spec.
4. Move to the next block only if it does not depend on the
   blocked one. Otherwise stop and wait.

The human reviews the blocked PR and answers. Codeless does not
guess to unblock itself.

## References

- Source SCOPE:
  [`/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md`](/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md)
- Current handoff:
  [`/home/user/code/rust/starter/rubix/docs/sessions/2026-05-23-next-steps-7.md`](/home/user/code/rust/starter/rubix/docs/sessions/2026-05-23-next-steps-7.md)
- Rubix architecture:
  [`/home/user/code/rust/starter/rubix/SCOPE.md`](/home/user/code/rust/starter/rubix/SCOPE.md)
- Contributor entry point:
  [`/home/user/code/rust/starter/rubix/HOW-TO-CODE.md`](/home/user/code/rust/starter/rubix/HOW-TO-CODE.md)
- File-layout rules:
  [`/home/user/code/rust/starter/rubix/FILE-LAYOUT.md`](/home/user/code/rust/starter/rubix/FILE-LAYOUT.md)
- Session boot:
  [`/home/user/code/rust/starter/rubix/NEW-SESSION.md`](/home/user/code/rust/starter/rubix/NEW-SESSION.md)
- Upstream PR ledger:
  [`/home/user/code/rust/starter/rubix/docs/design/starter-changes/README.md`](/home/user/code/rust/starter/rubix/docs/design/starter-changes/README.md)
- Exemplars to copy religiously:
  - Tool dispatch: `rubix/crates/rubix-tools/src/system/disk.rs`
  - DTO + descriptor: `rubix/crates/rubix-spi/src/dto/system/disk.rs`
  - Bundled flow: `rubix/crates/rubix-flows/flows/scheduled-system-check.yaml`
  - SKILL.md: `rubix/crates/rubix-skills/skills/system-checker/SKILL.md`
  - U1/U2/U3 API shapes: read commits `4c15dcb`, `9ab273d`,
    `7216d78` directly via `git show`.
