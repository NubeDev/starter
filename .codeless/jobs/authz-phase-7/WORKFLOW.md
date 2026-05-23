# Workflow — authz-phase-7

How to drive the stages in `template.yaml`. Read this before every
stage alongside `SCOPE.md` and the authoritative source SCOPE at
[/home/user/code/rust/starter/DOCS/auth/authz/SCOPE-EXT.md](/home/user/code/rust/starter/DOCS/auth/authz/SCOPE-EXT.md).

## Sequencing

Six stages, one REVIEW gate. Strictly linear:

- **Slice 7a** (stage 1) is the load-bearing migration: it adds
  `tenant_id` and `owner_id` to every tenant-scoped table in the
  workspace, lands the immutability triggers for both SQLite and
  Postgres, gates the rule engine on the tenant predicate before
  role/condition, and rewires token-issue + membership-revoke to
  be transactionally consistent. Every later slice depends on
  this being correct. The REVIEW gate after stage 1 exists
  because cross-tenant data leak is the most expensive bug to
  ship and the migration is one-way.
- **Slice 7b** (stage 2) lands teams + the new `contains`
  operator. Cannot proceed without slice 7a because the team
  rules are tenant-scoped.
- **Slice 7c** (stage 3) lands the decision sink + audit table.
  Order-independent with 7b in principle, but ships after 7b so
  the sink can record team-rule matches.
- **Slice 7d** (stage 4) lands the `permission:` manifest field
  + REST adapter wiring. Cannot proceed without 7c because the
  audit consequences (role-outer / permission-inner) are
  documented in terms of the sink's behaviour.
- **Slice 7d.2** (stage 5) lands MCP + gRPC parity using the
  shared `AuthGate.permission` field. Cannot proceed without 7d
  because the field is shared.
- **Stage 6** is the final sweep: docs, the demo as the
  canonical Phase 7 walkthrough, lint pass, exit summary.

Slices 7b, 7c, 7d.2 can each in principle slip without breaking
earlier slices, but **do not batch them**. Each stage ships as
its own commit; if the budget blows mid-job, the half-done
stages stay reviewable and the cherry-pick story is clean.

## Per-stage discipline

Before writing any code in a stage:

1. Re-read the corresponding section in the source SCOPE. The
   source SCOPE is the contract; this WORKFLOW is the process.
   If the source SCOPE is silent on a judgment call, the answer
   goes in `handover.md` under "source-SCOPE gaps" so the next
   round of design picks it up.
2. Re-read `SCOPE.md` §"In scope", §"Out of scope", and the
   relevant `R11`–`R15` rule in §"Constraints". The biggest risk
   on this job is silent scope creep — the source SCOPE
   explicitly carves out tempting features (`DELETE
   /v1/tenants`, query pushdown, intersect operator,
   multi-instance cache invalidation). Stay within the
   carve-outs.
3. For **stage 1**: enumerate every tenant-scoped table in the
   workspace before editing the migrations. `grep -rn 'CREATE
   TABLE' crates/ | grep -iE '(reports|flows|pages|marts|
   sandboxes|sessions|tokens|memberships)'` is the starting set
   — confirm completeness against the audit before adding
   `tenant_id` + `owner_id` columns. Missing a table at this
   stage means a tenant-scoped table without the predicate,
   which is the leak shape R11 exists to prevent.
4. For **stage 1**: write the immutability trigger for BOTH
   SQLite and Postgres in the same commit. The source SCOPE is
   explicit that a constraint in prose is not a constraint —
   missing one engine's trigger means the leak shape ships on
   that engine.
5. For **stage 2**: the new `contains` operator must error at
   engine **compile** time when the LHS isn't a JSON array, not
   silently false at evaluation. The smoke test
   `engine-compile-error-when-contains-LHS-not-array` is the
   gate; write it before the implementation.
6. For **stage 3**: the sink dispatch is non-blocking. Write
   the `deny-drops-cleanly-on-overflow` test first; if `record`
   awaits a DB write, the test will hang. The "fail fast on
   wrong impl" property is the safety net.
7. For **stage 3**: the deterministic-hash sampling uses a
   per-process random seed. Confirm the seed is set once at sink
   construction, not re-randomized per call (which would make
   the sampling non-deterministic per subject).
8. For **stage 4**: layer order is `with_role` outer →
   `with_scope` → `with_permission` inner → handler. The doc
   comment naming the audit consequence (role denies don't
   appear as permission denies) lives on the wiring point so
   the next person who wants to "fix" the order sees the prior
   art.
9. For **stage 5**: add the `surface: rest|mcp|grpc` field to
   `DecisionEntry` AND migration AND `DbDecisionSink` writes in
   one stage; gRPC has no authz today so this is where the
   change visibly lands.
10. `cargo check --workspace` from the `starter` repo root
    before any edit so the baseline is known-clean.

Before committing a stage:

1. `cargo fmt --all` clean.
2. `cargo clippy --workspace --all-features -- -D warnings`
   green.
3. `cargo test --workspace` green — including testcontainer
   Postgres tests under `--features testing -- --ignored`.
4. Every smoke test in `SCOPE.md` §"Deliverables" for the
   relevant slice is implemented and green.
5. For stages that add SQL: both SQLite and Postgres migrations
   apply on a fresh DB and on a Phase 1–6 DB.
6. For stage 1: `cargo test --workspace` green
   **without** `STARTER_AUTHZ_PHASE_7_DEFAULT_TENANT` set — the
   strictly-additive claim is that a Phase 1–6 deployment that
   doesn't enable tenants stays green.

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
   Stages that touch storage additionally run `cargo test
   --features testing -- --ignored` against testcontainer
   Postgres.
2. `docs` — update `handover.md` for the next stage. Module
   docstrings in new code cite the relevant `R11`–`R15` rule
   number. Stage 1's handover names every tenant-scoped table
   that grew `tenant_id` + `owner_id`; the REVIEW gate reads
   this enumeration.
3. `git` — stage the changes, commit with `stage N: <title>`,
   push to `codeless/authz-phase-7`. One stage, one commit.

A stage is not "done" until all three are green and the push
succeeds. If `checks` or `git` fails, fix the cause and retry —
do not mark the stage `[x]`, do not advance, and never `--force`
or `--no-verify`.

## REVIEW gate (one, after slice 7a)

At the gate write a handover comment containing:

- `cargo test --workspace --features testing -- --ignored`
  transcript proving every smoke test in `SCOPE.md`
  §"Deliverables" slice 7a is green on both SQLite and
  Postgres.
- The `cross-tenant-deny` test transcript with the
  `role:"*", resource:"*", actions:["*"], effect:"allow"` rule
  in place — the deny must still fire **without consulting the
  rule**.
- A `git grep -E 'tenant_id|owner_id' migrations/` enumeration
  of every tenant-scoped table that grew the two columns; cross-
  reference against the audit's expected set.
- The trigger-rejects-update transcript for both engines —
  Postgres `BEFORE UPDATE … RAISE EXCEPTION` and SQLite
  `RAISE(ABORT, '...')` — proving the immutability is enforced
  at the DB layer, not just in code.
- The token-revoke transaction proof: kill the test mid-revoke
  and assert no half-state (no membership row absent +
  token-row revoked, no membership row present + token-row
  unrevoked).
- The OAuth callback transcript showing both the `?tenant=…`
  path (single-membership auto-select bypasses the
  interstitial) and the multi-membership interstitial path
  (rendered HTML form, `POST /v1/auth/oauth/select-tenant`
  writes the resolved tenant to the session row).
- A `cargo test --workspace` transcript **without**
  `STARTER_AUTHZ_PHASE_7_DEFAULT_TENANT` set, proving the
  Phase 1–6 path stays green.
- The migration-refuses-to-run transcript when
  `STARTER_AUTHZ_PHASE_7_DEFAULT_TENANT` is unset on a DB with
  pre-existing Phase 1–6 data.

Gate question: *is the tenant predicate provably untouchable by
a wildcard rule, and does every smoke test in slice 7a's
acceptance pass on both engines?* If any of those is "no", the
gate fails — fix and re-request, do not advance.

Do not start slice 7b without explicit approval at this gate.

## Anti-patterns specific to this job

- **Do not** stuff `tenant_id` into `Principal.extra` and write
  `extra.tenant == object.tenant` conditions. R11 binds:
  tenants are a first-class predicate, not an attribute. The
  attribute-bus shape silently allows on missing keys; the
  typed-predicate shape defaults-deny.
- **Do not** make `tenant_id` `NOT NULL` on `Principal`.
  `Option<String>` is load-bearing for the strictly-additive
  claim: a Phase 1–6 consumer keeps `None` and rules without a
  `tenant` field still evaluate. Forcing `String` breaks every
  existing deployment.
- **Do not** ship one engine's immutability trigger and skip
  the other. The source SCOPE is explicit: a constraint in
  prose is not a constraint. Both Postgres `BEFORE UPDATE …
  RAISE EXCEPTION` and SQLite `BEFORE UPDATE … RAISE(ABORT,
  '...')` land in the same stage. No "we'll add the SQLite one
  later" — later is when the leak ships.
- **Do not** silently allow `principal.teams contains X` when
  the LHS isn't a JSON array. R13 binds: it's an engine
  compile-time error, parallel to R8's "loud failure" on
  missing attributes. Silent false at evaluation is the
  silent-allow shape this whole project exists to prevent.
- **Do not** widen the rule grammar beyond `contains`. R-13
  binds: one new operator. `intersect` is an open question in
  the source SCOPE; wait for the first real rule that needs it.
- **Do not** make the audit dispatch block the request path.
  R14 is explicit: best-effort, drop-on-overflow, fail-open on
  sink errors. A consumer needing fail-closed wires their own
  sink. The shipped sink is best-effort. The
  `deny-drops-cleanly-on-overflow` test exists to catch this
  regression.
- **Do not** sample denies. R14 binds: 100% deny retention.
  Sampling allows is the trade-off; sampling denies is the
  bug. The `tracing::warn { dropped_count }` path is the
  overflow valve, NOT a sampling mechanism.
- **Do not** sample the audit-of-audit chain. The `audit_logs`
  kind gets a per-kind override to N=1 so paging
  `/v1/authz/decisions` doesn't generate sampled-away allows
  per page request (~99% lossy otherwise).
- **Do not** flip the layer order to make role denies appear in
  the permission-deny dashboard. R15 names this as the
  acknowledged audit consequence. The fix is at the dashboard
  layer or a separate `with_role` audit hook, NOT layer
  reordering — reordering forces the engine to evaluate rules
  for requests the role gate would have killed, which is
  wasted work and a larger attack surface.
- **Do not** let extensions ship `ResourceSpec`s themselves.
  R15 covers the **consumption** side only — extensions
  declare which `(resource, action)` their endpoint maps to;
  the **host** still controls which kinds exist. Letting
  extensions register kinds is a Phase 8 question; doing it
  here moves the security boundary off the host.
- **Do not** introduce `DELETE /v1/tenants/{id}` in this job.
  The source SCOPE deliberately excludes it; cascading
  deletion across every tenant-scoped table deserves an
  explicit ops workflow (`ADR-tenant-deletion`). The
  soft-disable via `PATCH /v1/tenants/{id}` setting
  `disabled_at` is the meantime path.
- **Do not** invent a new migration source format. Use the
  existing `starter-store-sqlite::migrate` and
  `starter-store-postgres::migrate` patterns. Both engines'
  migrations live in the crate that owns the table (tenants
  table in `starter-auth-users`, decisions table in
  `starter-authz`).
- **Do not** silently auto-create a default tenant on first
  boot. The Phase 1–6 → Phase 7 cutover is operator-driven via
  `STARTER_AUTHZ_PHASE_7_DEFAULT_TENANT`. If the var is unset,
  the migration refuses to run and prints the manual-mapping
  caveat. Silent auto-create is the path where an operator
  with informally-multi-tenant data loses their separation on
  upgrade.
- **Do not** treat the OAuth interstitial as an SDUI page. It's
  hand-rolled HTML in `starter-auth-users`. Pulling in
  SDUI/page-builder here couples authz to the UI stack, which
  the project explicitly keeps separable.
- **Do not** add a `surface` enum that's open-ended. The values
  are `rest | mcp | grpc` — strongly typed, exhaustively
  matched. A future surface adds a variant + a migration; an
  open-ended string column silently drops new surfaces into
  the audit log without dashboards knowing.

## When to halt

- **Stage 1 audit** reveals a tenant-scoped table the enumeration
  missed (e.g. a crate added a `notebooks` table since the
  audit was written). Halt; add the table to the migration set,
  update the handover's enumeration, then resume. Missing a
  table at this stage is the leak shape; do not advance with
  partial coverage.
- **Stage 1** fails the strictly-additive test (cargo test
  --workspace **without**
  `STARTER_AUTHZ_PHASE_7_DEFAULT_TENANT` is red). Halt; this
  means a Phase 1–6 deployment would not boot after the
  upgrade. The fix is in the migration's gating, not the
  tests.
- **REVIEW gate 1** fails because the cross-tenant-deny test
  passes only with the wildcard rule absent. Halt; the engine
  is consulting the rule before the predicate, which is the R11
  violation. Fix the predicate ordering, re-request.
- **Stage 2** finds that an existing condition evaluator
  silently returns false on missing attribute access (parallel
  to where `contains` should error). Surface; do not just fix
  `contains` to error and leave the older path silent. R8 in
  the parent SCOPE says loud failure; the existing path is
  also wrong if it's silent. The fix is bundled with the
  `contains` work or surfaced as a separate concern.
- **Stage 3** sees the `deny-drops-cleanly-on-overflow` test
  hang. Halt; the sink dispatch is blocking. The shipped
  contract is `tokio::spawn` / channel-send, not awaited write.
  Diagnose, fix, re-run.
- **Stage 3** sees the deterministic sampling produce
  non-deterministic results for the same subject across runs.
  Halt; the seed is being re-randomized per call. The seed is
  per-process, set once at sink construction.
- **Stage 4** finds that flipping the layer order would make
  the `role+permission-compose-correctly` test pass more
  cleanly. Halt; the layer order is fixed by R15. The fix is
  in the test's assertions or a separate audit hook, not the
  order.
- **Stage 5** finds that the gRPC dispatcher cannot be wrapped
  with a permission check without a major refactor of the
  existing gRPC adapter. Surface; this is the source SCOPE's
  "gRPC has no authz today" reality, and the workstream may
  need its own job. Do not silently land partial gRPC support.
- **Budget** is blown before stage 6. Halt at the second commit
  of stage 5; the demo and lint pass become a follow-up job
  (`authz-phase-7-polish`). Do not silently land a partial
  stage 5 — the `surface-decisions-share-audit-trail` test is
  the load-bearing acceptance and must pass on the bundle.
