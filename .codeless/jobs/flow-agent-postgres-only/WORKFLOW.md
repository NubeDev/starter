# Workflow — flow-agent-postgres-only

How to drive the stages in `template.yaml`. Read this before every
stage alongside `SCOPE.md` and the authoritative ADR at
[/home/user/code/rust/starter/DOCS/storage/ADR-001-flow-agent-postgres-only.md](/home/user/code/rust/starter/DOCS/storage/ADR-001-flow-agent-postgres-only.md).

## Sequencing

Five stages, two REVIEW gates. Strictly linear:

- **Slices A and B** (stages 1 and 2) are inside
  `starter-store-postgres` only — purely additive on the library.
  No consumer is touched. The REVIEW gate after slice B catches
  semantic drift before it compounds.
- **Slice C** (stage 3) is inside `starter-prefs` only — also
  purely additive on the library. Lands the `postgres` feature
  parallel to the existing `sqlite` feature.
- **Slice D** (stage 4) is the application rewire. It changes
  `flow-agent`'s `Cargo.toml` and source. This is where the
  backend swap visibly lands; everything before it is library
  groundwork. The REVIEW gate after slice D catches anything that
  would make deletion (slice E) irreversible without a revert.
- **Slice E** (stage 5) is the deletion + final sweep. Cheap, but
  load-bearing for the "single backend, half the maintenance" goal
  in ADR-001.

Slice C can in principle interleave with slices A or B (different
crate), but **do not batch them**. Each slice ships as its own
commit; the diff stays reviewable; the handover stays honest.

## Per-stage discipline

Before writing any code in a stage:

1. Re-read the relevant ADR section. The ADR is the contract;
   this WORKFLOW is the process. If the ADR is silent on a
   judgment call, the answer goes in `handover.md` under "ADR
   gaps" so the next round of design picks it up.
2. Re-read `SCOPE.md` §"In scope" and §"Out of scope". The
   biggest risk on this job is silent scope creep — the ADR
   explicitly carves out tempting work (removing
   `starter-store-sqlite`, sqlx-generic abstraction, SPI
   conformance suite). Stay within the carve-outs.
3. For **stage 1** (slice A): `ls
   crates/starter-store-sqlite/src/flow/` to enumerate the files
   being ported; for each file, read the file end-to-end before
   writing the Postgres twin. Do not port query-by-query without
   the whole file in head — JSON column types and timestamp
   types are inferred from struct definitions at the top of the
   file.
4. For **stage 2** (agent_session_store.rs): start by `wc -l`
   confirming the file is the expected size (~716 LOC); split
   the port into logical sections (table CRUD, JSONB-heavy
   queries, datetime-heavy queries) and port one section at a
   time, running `cargo check -p starter-store-postgres
   --features flow` after each.
5. For **stage 3** (slice C): `find crates/starter-prefs/src -name
   '*.rs'` and read the existing SQLite store top-to-bottom before
   writing the Postgres twin. If the file layout differs from
   what the audit at stage 1 expected, document the actual layout
   in `handover.md` before editing.
6. For **stage 4** (slice D): `grep -rn 'sqlite\|Sqlite\|SqlitePool'
   examples/flow-agent/src/` and **enumerate every site** in
   `handover.md` before editing. The rewire is mechanical, but
   the safety net is the type-error trail — change one file at a
   time, compile after every file.
7. `cargo check --workspace` from the `starter` repo root before
   any edit so the baseline is known-clean.

Before committing a stage:

1. `cargo fmt --all` clean.
2. `cargo clippy --workspace --all-features -- -D warnings`
   green.
3. `cargo test --workspace` green (Postgres-backed `#[ignore]`
   tests run via the appropriate `--features … -- --ignored`
   invocation; they must pass against the testcontainers
   helper).
4. For stages 1, 2, 3: every new query goes through the dialect
   translation rules in `SCOPE.md` §"Constraints" ADR-3. If a
   translation is non-mechanical (subquery rewrite, JSON path
   change, datetime arithmetic), note it in `handover.md` under
   "semantics-changed queries" with a one-line justification.
5. For stage 4: `cargo run -p flow-agent` against a local
   `docker run postgres:16-alpine ...` boots cleanly; REST
   `/api/flows` CRUD works manually; `cargo test -p flow-agent
   -- --ignored` green.
6. For stage 5: `cargo tree -p flow-agent -e features | grep -i
   sqlite` returns nothing.

Commit + push via **mani** from the codeless-workspace root:

```
./bin/mani --config mani.yaml run commit --projects starter \
  MSG='stage N: <one-line title from template.yaml>'
./bin/mani --config mani.yaml run push --projects starter
```

No `--force`, no `--no-verify`. If a hook fails, fix the cause.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in
order. The user watches these tick over in the `Stages` overview;
they are how the user confirms a long-running stage actually
landed instead of just looking like it did. Do **not** rename or
reorder them.

1. `checks` — `cargo fmt --check` + `cargo clippy --workspace
   --all-features -- -D warnings` + `cargo test --workspace`.
   Stages 1, 2, 4 additionally run the relevant `--ignored`
   Postgres tests via `cargo test … --features … -- --ignored`.
2. `docs` — update `handover.md` for the next stage. Stages 1
   and 2 write into the "semantics-changed queries" section.
   Stage 4 writes the smoke-transcript snippet for the REVIEW
   gate. Module docstrings in any new code cite ADR-001 by
   number.
3. `git` — stage the changes, commit with `stage N: <title>`,
   push to `codeless/flow-agent-postgres-only`. One stage, one
   commit.

A stage is not "done" until all three are green and the push
succeeds. If `checks` or `git` fails, fix the cause and retry —
do not mark the stage `[x]`, do not advance, and never `--force`
or `--no-verify`.

## REVIEW gates (two)

### Gate 1 — after slice B (stage 2), before slice C

At the gate write a handover comment containing:

- `cargo test -p starter-store-postgres --features 'flow
  agent-session testing' -- --ignored` transcript, green.
- The "semantics-changed queries" section listing every query
  whose semantics drifted from SQLite to Postgres (not just
  syntax). Each entry: file:line, the SQLite version, the
  Postgres version, the one-line justification for the change.
- `git diff master -- crates/starter-store-sqlite` transcript
  showing **empty** — the SQLite crate must be unchanged.
- `cargo tree -p starter-store-postgres --features flow` showing
  the new dep tree (chrono, json, uuid features enabled as
  needed).

Gate question: *do the Postgres store impls satisfy the SPI
traits with no semantic regressions?* If any query's semantics
drifted unintentionally, the gate fails — fix and re-request,
do not advance.

Do not start slice C without explicit approval at this gate.

### Gate 2 — after slice D (stage 4), before slice E

At the gate write a handover comment containing:

- `cargo test -p flow-agent -- --ignored` transcript, green.
- A manual smoke transcript: `docker run` Postgres, `cargo run
  -p flow-agent`, `curl POST /api/flows` to create one, `curl
  POST /api/flows/<id>/fire` to fire it, browser screenshot
  showing SSE landing in the UI. Capture the full output, not a
  summary.
- An enumeration of every SQLite artefact slice E will delete:
  files under `examples/flow-agent/migrations/flow_agent/` that
  remain in their SQLite form, any orphan constants in source.
- `grep -rn 'sqlite\|Sqlite\|SqlitePool' examples/flow-agent/`
  showing zero matches that aren't comments or doc references.

Gate question: *does flow-agent build, boot, and pass its
ignored test suite end-to-end against a real Postgres?* If any
of those is "no", the gate fails — fix and re-request.

Do not start slice E (deletion) without explicit approval at
this gate. Deletion without the gate is recoverable but the
revert is messy; the gate is the cheap defence.

## Anti-patterns specific to this job

- **Do not** delete `crates/starter-store-sqlite/src/flow/`. It
  stays untouched. The SQLite version retains feature parity for
  other consumers per ADR-6. If something on the SQLite side
  looks wrong while reading it for the port, note it in
  `handover.md` under "follow-ups" and move on. This job's
  scope is the port, not a SQLite refactor.
- **Do not** introduce a sqlx-generic abstraction layer. ADR-001
  is explicit that the starter pattern is *two parallel impls
  behind one trait*; generalising over `Database` defeats the
  compile-time query benefits of each backend and is not in
  scope. If a port is making you wish for `impl<D: Database>
  FlowStore for Pool<D>`, write the second impl by hand and
  move on.
- **Do not** widen any `starter-flow-spi` trait to accommodate
  Postgres-specific behaviour. The traits are stable per ADR-1.
  If a method's semantics are hard to replicate on Postgres,
  surface — that's a design conversation, not a sneak edit.
- **Do not** use `sqlx::query!` compile-time macros in the new
  Postgres impls. The SQLite source uses runtime
  `sqlx::query(...)`; the Postgres impls match. The workspace
  must not need a live `DATABASE_URL` at compile time.
- **Do not** keep a hidden fallback `DATABASE_URL` default that
  points at a SQLite file. ADR-4 is explicit: boot fails fast
  on missing `DATABASE_URL` with an error message naming the
  env var verbatim. No silent fallback.
- **Do not** leave a side-SQLite database for `starter-prefs`.
  Slice C is the whole reason this job is one job instead of
  two — finishing without slice C means the example still ships
  `sqlx-sqlite` for prefs and the goal isn't achieved.
- **Do not** batch the rewire (stage 4) into a single mega-edit
  across `Cargo.toml`, `store.rs`, `main.rs`, `server.rs`,
  `migrations.rs`, and the test files. Change one file at a
  time and compile after each. The type-error trail is the
  safety net.
- **Do not** mark Postgres tests as not-ignored (the default
  `#[test]`). The project convention is `#[ignore]` for
  Postgres-backed tests so plain `cargo test` skips them on
  developers' machines without Docker. Stay consistent.
- **Do not** weaken the "semantics-changed queries" handover
  section in stage 2. If every query is "syntax only", the
  section is empty — that's fine and explicit. If even one
  query needs reasoning, every entry needs reasoning. The gate
  reads this section; treating it as a checkbox is a slow-burn
  correctness bug.
- **Do not** introduce a SQLite-to-Postgres data migration
  script. ADR's out-of-scope is explicit; this is a fresh-DB
  example.
- **Do not** add a SPI conformance test suite that runs the
  same trait-level tests against both backends. That is
  library-level work and warrants its own job per ADR's
  "What this ADR does NOT do". Note the gap in the final
  handover as a follow-up; do not implement it here.
- **Do not** introduce CI infra changes beyond extending the
  existing `--ignored` testcontainers job to cover
  `flow-agent`. If the existing pattern can't cover slice D's
  tests, surface in chat — CI infra changes need approval
  above this job's authority.
- **Do not** touch PR #22 surface
  (`starter-flow-spi::DynamicNodeKindRegistry`,
  `Contributes.nodes`, supervisor wiring, flow-nodes
  manifest). The port is orthogonal to PR #22 and must not
  modify its files. If a rewire step seems to require
  touching them, it's almost certainly a misshape — surface.

## When to halt

- **Stage 1 audit** reveals that a query in
  `starter-store-sqlite::flow` uses an API or SQLite extension
  with no Postgres equivalent (e.g. SQLite's `json_each` table
  function being used in a way that doesn't map cleanly to
  `jsonb_array_elements`). Halt; surface the specific query
  and propose a port shape. The resolution may be a small SPI
  trait tweak or a query rewrite that preserves trait
  semantics; either way, the design conversation happens
  before the port lands.
- **Stage 2 (agent_session_store.rs)** turns out to have a
  semantic drift on a query the test suite doesn't cover.
  Halt; write a Postgres test that reproduces the SQLite
  behaviour first, then port. Do not advance with an untested
  semantic change.
- **REVIEW gate 1** fails because the "semantics-changed
  queries" section reveals a regression. Halt; fix and
  re-request. Do not advance to slice C with known semantic
  drift in slice A/B.
- **Stage 4 (rewire)** turns out to have a `SqlitePool` site in
  `flow-agent` source that the audit at the start of stage 4
  missed. Stop the per-file rewrite, re-grep, update the
  handover's enumeration, then resume. Do not finish the
  rewire on partial coverage.
- **REVIEW gate 2** fails because the manual smoke doesn't
  land (e.g. SSE doesn't fire end-to-end against Postgres).
  Halt; this is a real bug. Diagnose, fix, re-request. Do not
  advance to deletion.
- **Stage 5** finds that `cargo tree -p flow-agent | grep
  sqlite` still shows a reachable `sqlx-sqlite`. Halt;
  something in the rewire is still pulling SQLite. Diagnose
  (most likely a `starter-prefs` feature mismatch or a forgotten
  `sqlx` feature on `flow-agent`'s `Cargo.toml`), fix, re-run
  the trio.
- **Budget** is blown before slice E. Halt at REVIEW gate 2,
  split slice E off as `flow-agent-postgres-only-deletion`.
  Do not silently land a partial deletion; the half-deleted
  state on master is worse than a clean intermediate where
  the rewire is done but the SQLite migrations still exist
  (the latter at least boots and runs).
