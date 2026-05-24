# Scope — rubix-goals-2-4-3

## Goal

Light up three more of the six rubix backend goals — Goal 2 (user-admin), Goal 4 (clickhouse-ruler), Goal 3 (flow-programmer) — end-to-end through the now-real agent loop landed in PR #30 / #31. After this job, four of the six bundled MCP tools (the three new ones plus the existing scheduled-system-check) invoke a real Claude loop that dispatches a real domain tool that writes to a real store and round-trips through prefs + i18n + audit. Every write is reversible via `rubix.undo.last`; flow definitions for Goal 3 live in PG (`flows_definitions` dimension table) so deploy / undo / cross-instance NOTIFY all light up; CH DDL writes (Goal 4) snapshot the prior state into a bounded `undo_snapshots` table.

The 25 verb files under `rubix/crates/rubix-tools/src/` are stubs today. This job fills the verbs for Goals 2 + 4 + 3 only (12 files), wires them into the tool registry, populates each goal's bundled flow YAML with `allowed_tools[]`, adds integration tests per verb, and writes the present-tense design docs (`docs/design/user-admin/`, `docs/design/clickhouse-rules/`, `docs/design/flow-programmer/`, `docs/design/undo/`). Goal 6 (weekly-report / cron) is **explicitly deferred** — it forces a scheduling decision (GAPS #16) that doesn't belong in this job.

Goals 1 (dashboards SDUI) and 5 (system-check, already wired) are not touched. The three new goals are ordered smallest → biggest unlock: Goal 2 first (proves the write-tool pattern + lands the undo dispatch registry), Goal 4 second (reuses undo + adds CH DDL surface), Goal 3 last (PG dimension table + hot reload + undo of deploys, which is the largest unlock and depends on the prior two having proven the pattern).

## In scope

### Phase A — shared infrastructure (lands first)

- **Undo dispatch registry** — wire `starter-undo`'s `Reversible` trait into the rubix tool dispatch so any tool implementing `Reversible` gets its inverse op recorded automatically. Add `rubix.undo.last` as a real tool that calls `starter-undo::undo_last(actor, scope)`. Bind to `com.rubix.user-admin` and `com.rubix.clickhouse-ruler` and `com.rubix.flow-programmer` flows via `allowed_tools[]`. Add `docs/design/undo/README.md` present-tense. Integration test in rubix-agent: dispatch a reversible tool, call undo, assert reversal happened.
- **`undo_snapshots` table in PG** — new migration under `rubix/crates/rubix-store-postgres/migrations/`. Columns: `id ULID PK`, `tenant_id`, `actor_id`, `resource_kind` (enum-as-text: `user`, `team`, `tenant`, `clickhouse_rule`, `clickhouse_mart`, `clickhouse_retention`, `flow_def`), `resource_id`, `snapshot_jsonb`, `created_at`, `superseded_at NULL`. Retention sweep job that runs at boot + every 24h: prunes per `(tenant_id, resource_kind, resource_id)` to keep the most recent N rows (default 50) or rows newer than X days (default 90), whichever is smaller. Both N and X configurable in `agent.toml` under `[undo]`. Tested via an integration test that inserts 100 snapshots, runs the sweep, asserts ≤ 50 remain.
- **Bundled-flow `allowed_tools[]` population** — every Phase B/C/D goal's YAML at `rubix/crates/rubix-flows/flows/` grows an `allowed_tools[]` list naming the verbs that goal's agent is permitted to dispatch. Today the loader's `convert.rs` already reads `allowed_tools[0]` for the primary-tool dispatch (per PR #31); extend it to read the full list and pass it through as the AiAgentNode's `allowed_tools` config so the AgentLoop's `ToolSet` filter is genuinely scoped per flow.

### Phase B — Goal 2 (user-admin)

- **Verbs:** fill `user/create.rs`, `user/disable.rs`, `user/list.rs`, `team/create.rs`, `team/assign.rs`, `tenant/list.rs`. Each `impl Tool` follows the `rubix.system.disk` pattern (DTO + descriptor + dispatch + `Diagnostic` output, never strings). The five write verbs (`user.create`, `user.disable`, `team.create`, `team.assign`) plus future `team.unassign` if natural — implement `Reversible` so undo works. `tenant.list` is read-only.
- **MessageKeys + catalogues:** every output `Diagnostic` ships with a `code` like `rubix.user.created`, `rubix.user.disabled`, `rubix.user.already_disabled`, `rubix.team.created`, `rubix.team.assigned`, `rubix.team.member_already`, `rubix.tenant.listed` — entries in both `rubix-spi/catalogues/en.json` and `es.json` in the same commit.
- **Skill:** `rubix-skills/skills/user-admin/SKILL.md` already exists; update its `Tools` and `Localisation` sections present-tense to reflect the six bound verbs and the MessageKey contract.
- **Flow YAML:** `flows/user-admin.yaml` grows `allowed_tools: [user.create, user.disable, user.list, team.create, team.assign, tenant.list, rubix.undo.last]`.
- **Integration test:** `tests/goal_2_user_admin_test.rs` — fires `tools/call com.rubix.user-admin` with prompt "create a user named ada with role admin"; asserts the user lands in PG, the Diagnostic carries `code=rubix.user.created`, the undo registry has one entry. Second test fires undo, asserts user is disabled.
- **Design doc:** `docs/design/user-admin/README.md` present-tense — how the six verbs map to `starter-auth-users` calls, the Reversible contract, the i18n keys.

### Phase C — Goal 4 (clickhouse-ruler)

- **Verbs:** fill `clickhouse/rule_write.rs`, `clickhouse/mart_create.rs`, `clickhouse/retention_set.rs`. Each writes via `ChClient`. Each implements `Reversible`: before the write, snapshot the current state (`SHOW CREATE TABLE` for rule.write and mart.create, current TTL for retention.set) into `undo_snapshots`; the inverse op rewrites the snapshot. mart.create's snapshot when the mart didn't previously exist is an empty body, and the inverse op is `DROP TABLE IF EXISTS`. Document this honestly in the design doc — undo of a `DROP TABLE` that follows a `mart.create` will recover the schema but not the data ingested between the create and the undo.
- **MessageKeys:** `rubix.clickhouse.rule.written`, `rubix.clickhouse.rule.invalid`, `rubix.clickhouse.mart.created`, `rubix.clickhouse.mart.already_exists`, `rubix.clickhouse.retention.set`, `rubix.clickhouse.retention.unchanged`. EN + ES.
- **Skill:** `rubix-skills/skills/clickhouse-ruler/SKILL.md` present-tense update.
- **Flow YAML:** `flows/clickhouse-ruler.yaml` grows `allowed_tools: [clickhouse.rule.write, clickhouse.mart.create, clickhouse.retention.set, rubix.undo.last]`.
- **Integration test:** `tests/goal_4_clickhouse_ruler_test.rs` — fires `tools/call com.rubix.clickhouse-ruler` with prompt "set retention on system_disk_history to 30 days"; asserts the ALTER ran, the Diagnostic carries `code=rubix.clickhouse.retention.set`, snapshot row exists. Second test fires undo, asserts retention is back to the prior value.
- **Design doc:** `docs/design/clickhouse-rules/README.md` present-tense — the three verbs, the snapshot-before-write contract, the data-loss caveat for mart.create undo.

### Phase D — Goal 3 (flow-programmer)

- **PG dimension table:** new migration `flows_definitions` under `rubix/crates/rubix-store-postgres/migrations/`. Columns: `id ULID PK`, `tenant_id`, `flow_id TEXT` (e.g. `com.rubix.weekly-report`), `revision_id ULID`, `body_yaml TEXT`, `created_at`, `created_by`, `superseded_at NULL`. UNIQUE `(tenant_id, flow_id, revision_id)`. Latest revision per `(tenant_id, flow_id)` is the row with `superseded_at IS NULL`.
- **FlowRegistry refactor:** `rubix-agent::boot::flows::build_flow_registry` (or whatever the equivalent in HEAD is — confirm at stage start) gains a second source: the include_dir!-bundled YAMLs **seed** the dimension table on first boot if a `(tenant_id, flow_id)` row doesn't exist, then every boot loads from PG. `pg_notify('rubix_flows', ...)` on insert/supersede + a listener in `rubix-agent` that calls `FlowRegistry::reload(flow_id, body)` on receipt. Bundled YAMLs remain the seed source of truth; deployed revisions are PG-only.
- **Verbs:** fill `flow_ops/deploy.rs` (validates YAML against `rubix_flows::yaml::RubixFlowYaml`, writes a new revision row, NOTIFY-s); `flow_ops/lint.rs` (parses YAML, returns a structured `Diagnostic` with line-numbered errors — does not write); `flow_ops/list.rs` (`SELECT flow_id, MAX(created_at) FROM flows_definitions WHERE superseded_at IS NULL` — read-only); `flow_ops/duplicate.rs` (read latest revision of source flow, write a new revision under target flow_id, NOTIFY). `deploy` and `duplicate` implement `Reversible` — undo marks the new revision `superseded_at = NOW()` and clears the previous revision's `superseded_at` (no row deletion, full audit trail preserved). `lint` and `list` are read-only.
- **`flow_ops/validate.rs`** is **out of scope** for this job — lint covers the parse + schema check; a deeper semantic validate (cycle detection, node-kind resolution, slot-shape compatibility) needs starter-flow exposure that doesn't exist yet and would derail the job. Track as a follow-up in `docs/scope/GAPS.md`.
- **MessageKeys:** `rubix.flow.deployed`, `rubix.flow.validation_failed`, `rubix.flow.lint_clean`, `rubix.flow.lint_errors`, `rubix.flow.listed`, `rubix.flow.duplicated`. EN + ES.
- **Skill:** `rubix-skills/skills/flow-programmer/SKILL.md` present-tense update.
- **Flow YAML:** `flows/flow-programmer.yaml` grows `allowed_tools: [flow.deploy, flow.lint, flow.list, flow.duplicate, rubix.undo.last]`.
- **Integration test:** `tests/goal_3_flow_programmer_test.rs` — fires `tools/call com.rubix.flow-programmer` with prompt "duplicate the scheduled-system-check flow as com.example.my-check"; asserts a new revision row exists, FlowRegistry resolves both flow ids, the Diagnostic carries `code=rubix.flow.duplicated`. Second test fires undo, asserts the new revision is superseded and `tools/list` no longer surfaces it.
- **Design doc:** `docs/design/flow-programmer/README.md` present-tense — the deploy contract, the PG dimension table shape, the NOTIFY mechanism, the undo contract.

### Phase E — closing docs + smoke

- Update `rubix/docs/scope/THIN-SLICE.md` — the table at the top calls out one-of-everything; add a new section "Goals lit up beyond the thin slice" listing the four now-real goals (2, 3, 4, 5) and the two still stubbed (1, 6) with their unblock criteria.
- Add `rubix/docs/sessions/<today>-goals-2-4-3-landed.md` — a session note documenting the verification evidence per goal (one MCP tools/call per goal, one undo per goal, one integration test count per goal), the new MessageKey count (`i18n_keys` boot log line), the snapshot retention sweep evidence (insert 100, sweep, count).
- The four bundled MCP tools that now invoke real loops: `com.rubix.scheduled-system-check`, `com.rubix.user-admin`, `com.rubix.clickhouse-ruler`, `com.rubix.flow-programmer`. The two remaining stubs (`com.rubix.dashboard-assistant`, `com.rubix.weekly-report`) return a "not wired yet" Diagnostic with a `code=rubix.goal.not_wired` that links the design doc explaining what's needed to wire them.

## Out of scope

- **Goal 1 (dashboards SDUI).** Frontend-adjacent; depends on resolving the SDUI persistence model and the `starter-ui-kit` ↔ backend wire format. Tracked as a separate future job.
- **Goal 6 (weekly-report / analytics).** Forces a cron-scheduling decision (GAPS #16) and an export sink (currently a non-goal). Deferred.
- **Deep semantic `flow.validate`** (cycle detection, slot-shape checks). Needs starter-flow API exposure that doesn't exist; tracked in `docs/scope/GAPS.md`.
- **`team.unassign`, `tenant.create`, `tenant.disable`.** Phase B covers what the bundled flow YAML asks for; the additional verbs are not in the goal scope.
- **Multi-tenant ClickHouse isolation.** Goal 4 writes use the existing single-tenant CH connection. Per-tenant CH DBs are a Phase 4 entry-gate concern in SCOPE.
- **OAuth + dashboards + flow-programmer-builds-UI + clipboard primitives.** Each has its own phase.
- **Live LLM in CI.** Recorded fixtures under `rubix/crates/rubix-agent/tests/fixtures/` remain the seam.
- **No `--no-verify`, no `--force` push** (only `--force-with-lease` after explicit rebase need, with operator confirmation). **No phasing markers in code.**

## Constraints

- **R1 — One verb per file.** ≤ 400 lines hard, ~100 typical. The existing `rubix-tools/src/<area>/<verb>.rs` shape is the contract; new files added in this job (the undo dispatch wiring, the snapshot sweep, the NOTIFY listener) follow the same shape.
- **R2 — Upstream-first.** If wiring `starter-undo` into rubix exposes a missing API on the starter side, file the upstream change first in `crates/starter-undo/` and land it before the rubix consumer. Same for any `starter-flow` exposure needed by Goal 3's FlowRegistry refactor.
- **R3 — Doc-tier rule.** Code comments link `docs/design/<area>/README.md` only — never `SCOPE.md`, `HOW-TO-CODE.md`, `NEW-SESSION.md`, `FILE-LAYOUT.md`, `docs/scope/`, or `docs/sessions/`. `./rubix/scripts/lint-doc-refs.sh` runs in every stage's `checks`.
- **R4 — Tool outputs are `Diagnostic` + structured data**, never pre-formatted strings.
- **R5 — Catalogue files are the source of truth for MessageKeys.** Every new `code` needs entries in both `en.json` and `es.json` in the same commit that introduces the verb.
- **R6 — Tests live with the code in the same commit.** Each verb gets an integration test under `rubix/crates/rubix-agent/tests/` in the same commit that fills the verb.
- **Commit messages.** `feat(rubix-tools):` for new verbs, `feat(rubix-agent):` for the registry / NOTIFY / sweep wiring, `feat(starter-undo):` for any upstream change, `feat(rubix-store-postgres):` for new migrations, `docs:` for design docs, `chore(catalogues):` for new MessageKey entries when not bundled with the verb commit.
- **Per-phase REVIEW gate.** Phase A ends with REVIEW (infrastructure landed, undo dispatch works); B ends with REVIEW (Goal 2 demo + undo); C ends with REVIEW (Goal 4 demo + DDL undo); D ends with REVIEW (Goal 3 demo + PG dimension + NOTIFY); E ends with REVIEW (closing docs + smoke).

## Open questions

1. **Where exactly does the undo dispatch registry live in rubix?** `starter-undo` ships the `Reversible` trait + cursor; rubix needs a thin wrapper that intercepts every `Tool::invoke` call and records the inverse if the impl is `Reversible`. Stage A.1 must grep `crates/starter-undo/src/` for the existing dispatch shape; if no thin-wrapper pattern exists in starter (only the trait), this becomes the upstream-first item under R2 — add `starter-undo::dispatch::record_if_reversible` and consume from rubix.
2. **NOTIFY channel naming for `flows_definitions`.** Default to `rubix_flows_definitions` (snake_case, prefixed). Confirm at stage D.1 whether the existing starter PG NOTIFY convention uses a different separator.
3. **Snapshot JSON shape per `resource_kind`.** Default each verb's `Reversible::snapshot()` returns the minimal JSON needed for the inverse op (e.g. the prior user row's `{email, role, disabled_at}` for `user.disable`; the prior `SHOW CREATE TABLE` output for `clickhouse.rule.write`). Document each shape in the per-goal design doc.
4. **`tenant_id` for snapshots when the operator is super-admin and the resource is global.** Default to the actor's `tenant_id` (the super-admin's home tenant); if the resource genuinely has no tenant (e.g. `clickhouse.retention.set` on a system table), use the all-zero tenant sentinel that's already used elsewhere (per `rubix/docs/sessions/2026-05-24-smoke-test-pr30.md` Step 5 evidence).
5. **Should `flow.deploy` accept arbitrary YAML or only the rubix surface?** Default: only YAMLs that parse via `rubix_flows::yaml::RubixFlowYaml`. Operators deploying flows with new node kinds need to land the node kind in `starter-flow` first (R2 again).

## References

- `rubix/SCOPE.md` — the six goals, R1–R13, the phases.
- `rubix/HOW-TO-CODE.md` — contributor entry point.
- `rubix/FILE-LAYOUT.md` — Rule Zero.
- `rubix/NEW-SESSION.md` — non-negotiables.
- `rubix/docs/scope/THIN-SLICE.md` — the demo path; one-of-everything was the previous bar, this job moves beyond.
- `rubix/docs/scope/GAPS.md` — undo (item 2), clipboard (item 3), and the deeper validate / cron / scheduling items.
- `rubix/docs/sessions/2026-05-24-smoke-test-pr30.md` — current verified state of Goal 5 + Phase A reference for the agent loop shape.
- `rubix/docs/design/starter-changes/README.md` — upstream PR ledger; any R2 upstream change goes here.
- `crates/starter-undo/` — the Reversible trait and cursor; the building block for Phase A.
- `crates/starter-flow/src/run.rs` — the engine; do not touch it in this job (PR #31's in-flight tracker is load-bearing).
