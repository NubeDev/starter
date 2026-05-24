# Workflow — rubix-goals-2-4-3

## Sequencing

15 stages across five phases. The phases are ordered by dependency: A (shared infra: undo dispatch + snapshot table + allowed_tools) lands first because every other phase depends on it; B (Goal 2, smallest, proves the write-tool + undo pattern); C (Goal 4, reuses undo for DDL); D (Goal 3, biggest, reuses undo and adds PG dimension storage); E (closing docs + PR).

Five REVIEW gates — one at the end of each phase. A failed gate stalls only the next phase, not the whole job.

## Per-stage discipline

### Phase A — shared infrastructure

The three Phase A stages are infrastructure. They have **no user-visible demo** of their own; their job is to make Phases B/C/D's demos cheap. Treat each stage as: build → unit-test → integration-test → commit. Do not skip the integration test because "it's just plumbing" — Phase A's bugs surface in Phase B as confusing "undo doesn't fire" and "allowed_tools doesn't filter" failures.

Stage A.1 has an upstream-first branch (SCOPE OQ-1): if `starter-undo` doesn't already expose a dispatch-wrapper pattern, the change lands in `crates/starter-undo/` **first** with its own commit + test, then the rubix consumer lands in a separate commit. Both must build standalone.

### Phases B / C / D — per-goal implementation

Each phase is three stages: verbs (writes first, then reads) → skill + flow YAML + integration test + design doc. The phase ends with a REVIEW gate.

**Discipline per verb file:**

1. Read the existing stub at `rubix/crates/rubix-tools/src/<area>/<verb>.rs` to confirm the module shape codeless seeded.
2. Read the equivalent already-working verb at `rubix/crates/rubix-tools/src/system/disk.rs` (the only fully-implemented verb today, per PR #28+). Match its shape: DTO struct, descriptor, dispatch fn, Diagnostic output.
3. Implement the verb body. For writes, implement `Reversible` — snapshot before the mutation, store via the snapshot writer added in A.2, inverse op restores from the snapshot.
4. Add the MessageKey entries to `rubix-spi/catalogues/en.json` AND `es.json` **in the same commit**. R5.
5. Add an integration test under `rubix/crates/rubix-agent/tests/` exercising the verb via MCP `tools/call` end-to-end. R6.
6. Run `cargo test -p rubix-tools -p rubix-agent` green. `./rubix/scripts/lint-doc-refs.sh` clean.

**Design doc per goal:** `docs/design/<area>/README.md` is present-tense — describe what the verbs DO now, not what they WILL do. Link from code comments only to `docs/design/<area>/README.md`, never to SCOPE, GAPS, sessions, or NEW-SESSION (R3).

### Phase D special — PG dimension table + NOTIFY

Two things bigger than the per-goal pattern:

1. **Migration adds a new table** under `rubix/crates/rubix-store-postgres/migrations/`. Number it after the existing migrations. Match the existing migration style (column types, comments).
2. **`pg_notify` trigger.** The migration adds `CREATE OR REPLACE FUNCTION rubix_flows_definitions_notify() RETURNS trigger ...` and a `CREATE TRIGGER ... FOR EACH ROW EXECUTE FUNCTION ...`. The listener side (`rubix-agent/src/boot/flow_notify.rs`) uses `sqlx::postgres::PgListener::listen('rubix_flows_definitions')` and dispatches `FlowRegistry::reload`. Integration test for the listener: insert a row, assert the FlowRegistry reflects it within 1 s.

The bundled YAMLs become the **seed** — on first boot, if a `(tenant_id, flow_id)` row doesn't exist, write one from the include_dir!-bundled YAML. Subsequent boots read from PG only. **Important:** if an operator edits a bundled YAML in the source tree after first boot, the change does **not** propagate — the PG row is the source of truth post-seed. Document this in `docs/design/flow-programmer/README.md`.

### Phase E — closing

Phase E is one stage. It does three things in one commit + PR:

1. Add a "stub" Diagnostic for Goals 1 and 6 (`com.rubix.dashboard-assistant`, `com.rubix.weekly-report`) so the MCP `tools/call` doesn't return null or error — returns a real Diagnostic with `code=rubix.goal.not_wired` and a `link` field pointing at the future design doc.
2. Update `THIN-SLICE.md` with a new "Goals lit up beyond the thin slice" section.
3. Add the session note `docs/sessions/<today>-goals-2-4-3-landed.md` documenting verification.

Then open the PR.

## Anti-patterns specific to this job

- **Don't skip Phase A's integration tests.** Phase A is infrastructure; bugs there surface as "undo isn't working" in Phase B and cost more to debug than the test cost.
- **Don't reuse the system/disk.rs file pattern by copy-paste — re-read it.** It's the only fully-implemented verb. Copying without reading risks copying outdated comment links or stale i18n key names.
- **Don't write inline Diagnostic strings.** Every `code` must exist in both catalogue files in the same commit.
- **Don't add tools to the registry without adding them to the flow YAML's allowed_tools.** The AgentLoop's ToolSet filter (per A.3) means a tool not in `allowed_tools` is invisible to the agent even if registered. Both edits in the same commit.
- **Don't write a Reversible whose `revert` is a no-op log line.** If revert isn't real (because the underlying store can't be reverted), the verb shouldn't implement `Reversible` at all — document the limitation in the design doc.
- **Don't add starter-flow exposure for the deeper `flow.validate`.** That's out of scope per SCOPE. lint covers parse+schema; deeper semantic validate is a tracked follow-up.
- **Don't list paths with brace expansion in handovers.** `routes/{mod.rs,tools.rs}` trips the diff-verify pre-check. List paths individually.
- **Don't list a path under "Done" that the stage didn't modify.** Same trap. "Done" means files modified, not files touched or scripts run.
- **Don't `--no-verify`, don't `--force`** (only `--force-with-lease` after explicit rebase need, with operator confirmation). Hooks fail → fix the cause.

## REVIEW gate behaviour

Each REVIEW gate commits and pushes the stage(s) that led to it; the gate itself commits nothing. Write the gate's question into `handover.md` for the next stage, halt, wait for operator confirmation.

At each REVIEW gate, the handover must include:

- One-line title + closed bug ids per commit made in the phase.
- `cargo test` counts per crate touched.
- One operator-runnable manual flow (the curl sequence demonstrating the goal end-to-end).
- Any deviations from SCOPE (e.g. an Open Question resolved differently than the default).
- Whether the upcoming phase is unblocked by the just-completed one or whether a BLOCKED needs to be raised.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in order. Do **not** rename or reorder them.

1. `checks` — run the stage's verify list. Every step must pass. On failure: stop, fix, re-run; do not advance.
2. `docs` — update `handover.md` for the next stage and the active session doc, in the same worktree.
3. `git` — stage the changes, commit with the message `stage N: <one-line title from template.yaml>`, push to `codeless/rubix-goals-2-4-3`.

A stage is not "done" until all three are green and the push succeeds. Never `--force`, never `--no-verify`. If a REVIEW gate stage produced no diff of its own (gate-only), mark `git` as `skipped — gate-only`.

## Hard rules (repeated)

- One verb per file. ≤ 400 lines hard, ~100 typical. No utils.rs / helpers.rs / common.rs / misc.rs.
- Code comments link `docs/design/<area>/README.md` only. Never SCOPE / HOW-TO-CODE / NEW-SESSION / FILE-LAYOUT / docs/scope / docs/sessions. `./rubix/scripts/lint-doc-refs.sh` enforces it.
- No phasing markers in code. No `// Phase A`, `// STAGE-1 done`, `// FIXED:`.
- Upstream-first (R2). starter-undo / starter-flow changes land before rubix consumes them.
- Tool outputs are `Diagnostic` + structured data, never pre-formatted strings.
- Catalogue files are the source of truth for MessageKeys. No new key without entries in both en.json and es.json in the same commit.
- Tests live with the code in the same commit (R6).
- Comments explain *why*, not *what*. No emojis.

## References

- `rubix/SCOPE.md`
- `rubix/HOW-TO-CODE.md`
- `rubix/FILE-LAYOUT.md`
- `rubix/NEW-SESSION.md`
- `rubix/docs/scope/THIN-SLICE.md`
- `rubix/docs/scope/GAPS.md`
- `rubix/docs/sessions/2026-05-24-smoke-test-pr30.md`
- `rubix/docs/design/starter-changes/README.md`
- `crates/starter-undo/`
- `crates/starter-flow/`
- `rubix/crates/rubix-tools/src/system/disk.rs` (the canonical implemented-verb exemplar)
- `rubix/crates/rubix-flows/flows/scheduled-system-check.yaml` (the canonical implemented-flow exemplar)
