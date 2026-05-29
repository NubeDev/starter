# Workflow — comprehensive-extension-management

How to drive the stages in `template.yaml`. Read this before every stage
alongside `SCOPE.md` and the design doc at
[/home/user/code/rust/starter/rubix/docs/scope/extensions/comprehensive-extension-management.md](/home/user/code/rust/starter/rubix/docs/scope/extensions/comprehensive-extension-management.md).

## Sequencing

Five code stages and one REVIEW gate. Strictly linear:

- Stages 1–3 are additive and independent in spirit but share types in
  `starter-ext-spi` (`issue.rs`, `process.rs`, `metrics.rs`). Land them
  in order so the metrics aggregate (stage 3) can reference the
  `ProcessStats` from stage 2.
- The **REVIEW gate** sits before stage 4. Stage 4 is the first
  destructive work — it deletes warehouse data and cache entries. Do not
  start stage 4 until the operator approves the `CleanupProvider` shape
  and the namespace-scoping guarantee at the gate.
- Stage 5 (rubix wiring + frontend) cannot start until stage 4's trait +
  built-in providers exist — rubix's providers implement that trait and
  the UI calls the `/cleanup` + `?purge` endpoints stage 4 adds.

No other REVIEW gates: stages 1–3 are read-only additive surfaces (new
endpoints, new crate) with no destructive effect, and stage 5 is wiring
against an already-reviewed trait.

## Per-stage discipline

Before writing any code in a stage:

1. Re-read the corresponding section (§1–§5) of the design doc. The
   design text is the contract; this WORKFLOW is the process. The scope
   is locked — if you find yourself wanting to reopen a "Locked
   decisions" item, halt and surface instead.
2. Re-read `SCOPE.md` §"Out of scope" and §"Constraints". The biggest
   risk is **starter/rubix leakage**: putting logic in rubix that could
   live upstream. Before adding any line to a `rubix-*` crate, ask "does
   this need warehouse or skill knowledge?" — if not, it belongs in
   `starter-ext-*`.
3. For stage 1: enumerate every existing failure source the supervisor
   and host already track (event ring kinds, the capability-violation
   counter, `WorkerState.last_error`, `ExtensionRecord.failure`) and map
   each to an `IssueCode`. The issues view is a *projection* of existing
   state, not a new collector — do not add new tracking.
4. For stage 2: confirm the pid is stored and cleared on the same code
   paths as the existing `EventKind::Spawned` push and the exit handler.
   The `/proc` parse is Linux-only; gate it behind `cfg(target_os =
   "linux")` and return `None` elsewhere.
5. For stage 3: the new crate is a **leaf** — it must not depend on
   `starter-ext-supervisor`, `-mcp`, `-server`, or `-workers`. Those
   depend on it. If you reach for a supervisor type inside
   `starter-ext-metrics`, you have the arrow backwards.
6. For stage 4: every `purge` path must be scoped to the extension's own
   namespace and must be safe to run twice. Write the idempotency test
   first.
7. For stage 5: the rubix providers are the *only* place warehouse/skill
   knowledge enters. The envelope projection maps codes to MessageKeys —
   no English strings cross from starter.

## Verify before committing a stage

Run from the relevant workspace root. Stages 1–4 touch the
`starter-extensions` cargo workspace; stage 5 also touches the `rubix`
cargo workspace and the `rubix/frontend` package.

1. `cargo build` green for the affected workspace.
2. `cargo clippy --all-features -- -D warnings` green.
3. `cargo fmt --check` green.
4. New tests for the stage pass (`cargo test` for the affected crates) —
   issue derivation (s1), pid set/clear + `/proc` parse (s2), counter
   increments (s3), discover/purge + idempotent purge + namespace scope
   (s4).
5. Stage 5 only: `tsc -b` (or `npm run typecheck`), `vite build` (or
   `npm run build`), and the frontend lint all green in `rubix/frontend`.

Record the test transcript for the stage in `handover.md`.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in order.
The user watches these tick over in the `Stages` overview; they are how
the user confirms a long-running stage actually landed. Do **not** rename
or reorder them.

1. `checks` — run the stage's `verify:` list. Every step must pass. On
   failure: stop, fix, re-run; do not advance to `docs`.
2. `docs` — update `handover.md` for the next stage and tick the relevant
   `[x]` in `SCOPE.md` §"Deliverables", in the same worktree, so the
   fresh agent opening the next stage has the context it needs.
3. `git` — `git add -A` from the worktree root (or specific paths if the
   stage was surgical), commit with `stage N: <one-line title from
   template.yaml>` so history mirrors the template stages one-for-one,
   and push to `codeless/comprehensive-extension-management` so the work
   survives a wiped worktree.

A stage is not "done" until all three are green and the push succeeds. If
`checks` or `git` fails, fix the cause and retry — do not mark the stage
`[x]`, do not advance, and never `--force` or `--no-verify`. The REVIEW
gate still commits + pushes the stage that led to it; it only pauses the
next stage.

## REVIEW gate behaviour (before stage 4)

When you reach the gate, write into `handover.md`:

- The final `CleanupProvider` trait signature as landed in stage-4 prep
  (or as designed if not yet written).
- The exact list of `CleanupKind`s and, for each, the namespace bound
  that scopes it (e.g. `WarehouseTable` → only `com_<id>__*`).
- A worked dry-run example: for one installed extension, the
  `Vec<CleanupItem>` `discover` would return.
- Confirmation that `purge` is idempotent and the test that proves it.

Then stop and wait for operator approval before starting stage 4.

## Anti-patterns specific to this job

- **Do not** put cleanup logic that needs no warehouse/skill knowledge in
  rubix. The enablement-row, UI-cache, and i18n-cache providers are
  built-in to `starter-ext-server`. Only the warehouse-drop and
  skill-unregister providers live in rubix.
- **Do not** make `starter-ext-metrics` depend on the supervisor or the
  adapters. It is a leaf; the dependency arrows point into it.
- **Do not** add a second timer/collector thread for metrics or process
  sampling. Ride the existing health tick (locked decision #2).
- **Do not** report a host pid for builtin/wasm extensions. They return
  `process: null` and `flavour` distinguishes them (locked decision #3).
- **Do not** make `purge` return `404` on an already-clean id. It is a
  no-op `200` (locked decision #4).
- **Do not** flip the enablement row to `disabled` on purge and leave it
  — `purge` **deletes** the row. (Plain uninstall without `purge` keeps
  today's `disabled` behaviour.)
- **Do not** bake English strings into starter responses. Stable `code`
  strings only; rubix owns the MessageKey mapping.
- **Do not** attempt hot-mount of a freshly installed extension. Install
  stays next-boot; you only add the `restart_required` flag.
- **Do not** widen scope into fleet-wide dashboards, resource
  enforcement, or the registry-URL install — all explicitly out of
  scope.

## When to halt

- A failure source exists that does not map cleanly to an `IssueCode`
  (stage 1). Surface it rather than inventing an `Other` catch-all that
  hides real categories.
- The `/proc` sampler needs a crate dependency to be reliable (stage 2).
  Halt — the locked decision is a dependency-free Linux `/proc` read; a
  new dep is a scope change worth surfacing.
- A cleanup provider cannot guarantee namespace scoping for some resource
  (stage 4) — e.g. a warehouse object not prefixed `com_<id>__`. Halt;
  removing data outside the extension's namespace is never acceptable.
- Stage 5 reveals that the rubix `SkillRegistry` has no `remove` (only
  add/approve). Halt — adding a removal path to the registry is a
  starter/rubix design question, not a silent inline patch.
- Any urge to reopen a "Locked decisions" item. The scope is locked;
  surface the conflict instead of re-deciding.
