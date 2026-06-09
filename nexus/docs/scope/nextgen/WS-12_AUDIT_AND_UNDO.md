# WS-12 — Audit Log & Undo/Redo (one changelog substrate, for everything)

> **Status:** Proposal · **Wave:** 1 (changelog-on-PG + audit query) + 2 (per-kind Reversible + undo UI) · **Owner:** _unassigned_
> **Depends on:** `starter-spi/changelog` + `starter-changelog-postgres` + `starter-undo` (exist, production-grade) · **absorbs** the WS-09 "audit log" item
> **Migration:** block `16xx` — `1601_changelog.sql` + `1602_undo_cursors.sql` (port `starter-changelog-postgres/*` + `starter-undo/*`, add `tenant_id`+RLS) · **Read first:** GAP_ANALYSIS §2.12, ROADMAP §0
> **Verified:** `82a6a19a` on 2026-06-09 — re-grep this WS's file:line claims before building (ROADMAP §0).
>
> **The user's ask:** undo/redo **and** an audit log, **for everything** — users, dashboards,
> datasources, … — built the way that's "better long term."
>
> **The better-long-term answer is already designed in this repo:** audit and undo are **one
> append-only change ledger**, not two systems. `starter-spi/changelog`'s own docstring says it:
> *"Five product features collapse onto this primitive: user audit log, AI-agent log, undo/redo,
> duplicate, and copy/paste."* WS-12 is mostly **wiring that substrate into nexus** + writing **one
> `Reversible` impl per resource kind**.

---

## 1. The idea in one paragraph

Every domain mutation in nexus (create/update/delete on a user, dashboard, datasource, panel, flow,
alert rule, grant, …) appends one `Change` row to an **append-only changelog** (`starter_changes`):
`{id, at, actor, resource{kind,id,owner}, op, before, after, patch, group_id, correlation}`. That
single ledger is read two ways: **audit log** = query it (who changed what, when, before→after);
**undo/redo** = replay rows in reverse/forward via a per-kind `Reversible` impl, tracked by a
per-actor redo cursor and grouped by `group_id` (a multi-row transaction undoes as one step). Adding
a new undoable/audited kind = **register one `Reversible` + emit one `ChangeDraft` after the
mutation**. Duplicate and copy/paste fall out of the same primitive for free. This is *the* "for
everything, better long term" architecture — one substrate, five features.

---

## 2. What already exists (production-grade — do NOT rebuild)

Read before writing. This subsystem is mature; nexus just hasn't adopted it.

### `starter-spi/src/changelog/` — the model + extension points
- **`Change`** (`change.rs`) — the append-only envelope (fields above). One row in `starter_changes`.
- **`Actor`** (`actor.rs`) — `User{subject}` | `Agent{run_id,model}` | `System`. `subject` reuses
  `Principal::subject` — no parallel id. **The AI-agent log is the same ledger, just `Actor::Agent`.**
- **`Op`** (`op.rs`), **`ResourceRef`** (kind+id+owner+tenant), **`GroupId`/`ChangeId`/`TraceId`**.
- **`ChangeRecorder` + `ChangeTx`** (`recorder.rs`) — the ONLY write path; `transaction()` assigns one
  `group_id` per tx so grouping is correct by default; **`forget()`** tombstones before/after/patch
  for GDPR right-to-erasure while preserving replay integrity.
- **`Reversible`** (`reversible.rs`) — **the one extension point.** Per resource kind: `apply_inverse`
  (undo), `apply_forward` (redo/paste), `clone_with` (duplicate → N rows sharing a group). Ships
  snapshot-vs-patch guidance (small/lifecycle → snapshot; large/diff-shaped → RFC-6902 patch).

### `starter-changelog-postgres/` + `starter-changelog-sqlite/` — the store (complete)
- `ChangeRecorder` impls over **`starter_changes`** (migration `0001_init.sql`: indexed `at`,
  `resource_kind/id`, `group_id`, `actor`, generated `actor_model` column).
- A **`ChangeLog` query trait + `PgChangeLog`** (`query.rs`) — paged reads over the ledger (the audit
  *read* substrate already exists).
- LISTEN/NOTIFY **tailing** (`tail.rs`/`tail_listen.rs`) for live audit streams, **retention/prune**
  (`prune.rs`), and **per-kind policy** (`policy.rs` — undo-retention / audit-retention per kind).

### `starter-undo/` — per-actor undo/redo over the changelog (complete)
- **`UndoService`** (`service.rs`) — groups by `group_id`, dispatches through a
  **`ReversibleRegistry`**, keeps a **per-actor redo cursor** (`starter_undo_cursors`, CAS-on-epoch
  for concurrent-safe pop). `undo_last`/`redo_last` convenience wrappers.
- **`record_if_reversible`** (`dispatch.rs`) — the call you drop in after a successful mutation: if
  the kind is registered, it appends the `ChangeDraft`; otherwise no-op. Returns the `group_id`.
- **Routes** (`routes.rs`) — `POST /v1/undo`, `POST /v1/redo`, targeting the authenticated principal,
  returning the applied `group_id` so the UI refreshes affected resources.

### What is NOT there yet (the WS-12 work)
- **No audit HTTP endpoint** — `PgChangeLog` can query, but no `GET /api/v1/audit` route exposes it.
- **nexus doesn't record anything** — no mutation handler calls `record_if_reversible`; no nexus
  `Reversible` impls are registered. (The crates ship example impls — `UserReversible` snapshot,
  `TeamReversible` patch — as references.)
- **Not mounted on nexus' Postgres+RLS** — migrations + recorder wiring need to land in nexus-api.
- **No undo/redo UI** in nexus-ui.

---

## 3. Design for nexus

### 3.1 Mount the substrate (Wave 1)
- Port `starter-changelog-postgres/migrations/*` (the `starter_changes` table, NOTIFY, policy) and
  `starter-undo/migrations/postgres/*` (the cursor table) into nexus migrations **under the runtime
  RLS role**, tenant-scoped (add `tenant_id` + RLS like the other nexus tables; the recorder writes
  inside the tenant tx so `current_setting('app.tenant_id')` is bound — reuse `tenant_tx.rs`).
- Construct a `PgChangeRecorder` + `UndoService` (with a `PgUndoCursor`) in nexus-api state; mount
  `undo_router`.

### 3.2 Record on every mutation (the "for everything" part)
Drop **`record_if_reversible(registry, recorder, actor, draft)`** into each nexus write handler,
right after the successful domain mutation, inside the same tenant transaction. The handler already
has the `before` (it read the row for authz/version checks) and the `after` (what it wrote):
```
// in routes/dashboards/update.rs (illustrative)
let before = store.get(id).await?;            // already fetched
let after  = store.update(id, patch).await?;  // the mutation
let group  = record_if_reversible(&reg, &recorder,
                Actor::User { subject: principal.subject.clone() },
                ChangeDraft::update(resource_ref, json(before), json(after))).await?;
```
Kinds to cover (the user's "users, dashboards, datasources and so on"): **dashboards, panels,
datasources, flows, alert rules/channels/silences, grants/shares, folders (WS-05), variables
(WS-02), kinds (WS-10), users/teams** (via the auth crates' handlers — coordinate). Read-only verbs
record nothing (unregistered kinds are skipped by design).

### 3.3 One `Reversible` per kind (the extension point)
For each undoable kind, implement `Reversible` (`apply_inverse`/`apply_forward`/`clone_with`) against
the nexus store. Pick snapshot vs patch per the matrix in `reversible.rs`:
- **Snapshot** (full before/after): datasources, alert rules, panels, grants — small, lifecycle
  (create/delete flip existence).
- **Patch** (RFC-6902): large/diff-shaped edits — a flow's node config, a team's membership flip.
- ⚠️ **Dashboards are PINNED to snapshot (ROADMAP §6a D2), not patch** — even though the JSON model
  can get big. Reason: WS-05 "restore to version N" needs an **absolute state at N**, and a patch
  chain can't give that without walking back to a snapshot. So the dashboard `Reversible` records full
  `before`/`after`. If snapshot size becomes a real problem, the mitigation is *periodic snapshot +
  patch-between* (a compaction detail decided then) — **not** plain patch, which would break
  versioning. This is a committed decision, not "flip later."
Register them all in a `ReversibleRegistry` at server build (one line per kind).

### 3.4 Audit query surface (Wave 1 — the net-new HTTP piece)
- **`GET /api/v1/audit`** over `PgChangeLog`: filter by `resource_kind`, `resource_id`, `actor`,
  `op`, time range; paged; newest-first; tenant-scoped via RLS. Returns `Change` rows (before/after
  for diff rendering).
- **`GET /api/v1/audit/resources/{kind}/{id}`** — the history timeline for one resource (powers a
  "History" tab on a dashboard/datasource).
- Optional **live tail** via the existing LISTEN/NOTIFY (`tail_listen.rs`) → an SSE audit stream
  (reuse the nexus SSE infra).
- **AuthZ:** audit read is privileged — gate behind an admin/`audit:read` permission (a tenant admin
  sees their tenant's log; cross-tenant only for a platform super-admin). Never leak another tenant's
  rows (RLS + grant check, like everything else).

### 3.5 Duplicate / copy-paste (free bonus, couples with WS-05)
`Reversible::clone_with` is exactly **"duplicate this dashboard"** / paste-as-new — N new rows under
one group, undoable as one step. WS-05's "duplicate dashboard" should be implemented *via* this, not
separately.

### 3.5b Coverage verification harness — the safeguard against silently-partial audit (review #3)
Because each kind's `record_if_reversible` call lives in *another workstream's* handler (C6), audit
coverage can drift to silently-partial — **the worst failure mode for an audit log** (it looks
complete but isn't). WS-12 ships a **coverage guard**, not just a convention:
- **A `ReversibleRegistry` ↔ mutation-route audit test**: enumerate the registered mutable kinds and
  assert each has at least one create/update/delete route that produces a `Change` row. A registered
  kind with a mutation path but no recorded change **fails CI**. (Implement as an integration test
  that exercises each kind's write endpoint and asserts a row landed, or a static check that every
  `routes/<kind>/{create,update,delete}.rs` contains a `record_if_reversible` call.)
- **A "known mutable kinds" manifest** WS-12 owns: the checklist of kinds that MUST record. Adding a
  new mutable kind without recording → the guard flags it. This makes "did WS-08 wire its datasource
  recording?" a *test failure*, not a thing someone has to remember to check.
- **`before` capture correctness**: the test also asserts the `before` snapshot is non-null on an
  update (catches the easy mistake of forgetting the pre-read or running it outside the tenant tx so
  RLS returns nothing → a silently empty audit row).

### 3.6 Undo/redo UI (Wave 2)
- Global **Undo/Redo** (Cmd/Ctrl+Z / Shift+Cmd/Ctrl+Z) calling `POST /v1/undo|redo`; on success use
  the returned `group_id` to invalidate the affected TanStack queries so the canvas refreshes.
- A toast "Renamed dashboard · Undo" after mutations (optimistic, with the group id).
- An **audit/history view**: a per-resource "History" tab + an admin "Audit log" screen with
  filters and a before→after diff viewer.

## 4. Scope (this workstream)
1. **Mount changelog + undo on nexus Postgres+RLS** (migrations + recorder/service wiring + tenant tx).
2. **`record_if_reversible` in every nexus mutation handler** (dashboards, panels, datasources, flows,
   alerts, grants, folders, variables, kinds; users/teams via auth crates — coordinate).
3. **`Reversible` impl + registry entry per kind** (snapshot/patch per the matrix).
4. **Audit query API** (`GET /audit`, `GET /audit/resources/{kind}/{id}`, optional SSE tail) + authz gate.
5. **Undo/redo UI** (shortcuts + toasts + query invalidation by `group_id`).
6. **Audit/history UI** (per-resource History tab + admin Audit screen + before→after diff).
7. **Retention policy** wiring (`policy.rs`): per-kind undo-retention + audit-retention; a prune sweep
   (`prune.rs`) on a schedule (reuse the alert-scheduler tick pattern).
8. **GDPR `forget()`** path for user-erasure requests (tombstone a subject's rows).
9. **Coverage guard (§3.5b)**: the "known mutable kinds" manifest + the CI test that fails if a
   registered mutable kind has no recording path / records an empty `before`. WS-12 cannot reach
   "done" with this red. *(This is how WS-12 stays whole even though the per-handler calls land in
   other workstreams' PRs.)*

## 5. Acceptance criteria
- [ ] Editing a dashboard appends a `Change` row (actor=user, before/after captured), inside the
  tenant tx; `POST /v1/undo` reverts it and refreshes the canvas; `POST /v1/redo` re-applies.
- [ ] Undo of a multi-row action (e.g. add-panel-and-layout) reverts as **one group**.
- [ ] The same flow works for a datasource and an alert rule (proving "for everything" via the
  registry, not per-feature code).
- [ ] `GET /api/v1/audit` returns who/what/when with before→after; filterable; **tenant-isolated**
  (cross-tenant read impossible); gated behind audit permission.
- [ ] A resource "History" tab lists its changes with a diff.
- [ ] Duplicate-dashboard is implemented via `clone_with` and is itself undoable.
- [ ] Retention prunes per policy; `forget()` tombstones a subject while preserving row counts/order.
- [ ] **Coverage guard is green:** every registered mutable kind has a recording path and records a
  non-empty `before` on update; a deliberately-unwired kind makes the guard test **fail** (proving it
  works). No silently-partial audit.
- [ ] Tests: record-on-mutate per kind, undo/redo round-trip + grouping, audit query + tenant
  isolation, clone_with, forget tombstoning, **the coverage guard**. (The crates already test cursor
  CAS + dispatch — add nexus-edge tests.)

## 6. Open questions to settle in Wave 0
1. **Tenant scoping of `starter_changes`** — the crate's table is not tenant-aware out of the box;
   nexus needs `tenant_id` + RLS. Add it in the nexus migration (or upstream a tenant column). Confirm
   the recorder writes inside the tenant tx so RLS binds. **Recommended:** nexus migration adds
   `tenant_id` + `FORCE ROW LEVEL SECURITY`, recorder runs in `tenant_tx`.
2. **Snapshot vs patch for dashboards** — start snapshot (simple, lifecycle-friendly), flip to patch
   if the JSON model (WS-05) makes snapshots heavy. Decide with WS-05.
3. **Audit retention vs undo retention** — different horizons (audit = long/compliance; undo =
   short/session-ish). Set per-kind policy defaults.
4. **Users/teams recording** — those mutations live in `starter-auth-users`/`starter-authz`. Do we
   wire `record_if_reversible` there (upstream) or only audit nexus-owned kinds for v1? **Recommended:**
   v1 records nexus-owned kinds + *audits* auth events via the auth crates' existing hooks if any;
   full undo of user/team changes is a fast-follow.
5. **Relationship to WS-05 dashboard versioning** — **RESOLVED** (ROADMAP §6a D1/D2): one ledger,
   versions = tagged changelog snapshots, dashboards pinned to snapshot. No longer an open question;
   see §7.

## 7. Relationship to other workstreams
- **WS-09 (production hardening):** the "audit log" item in WS-09 **moves here** — WS-12 *is* the audit
  log. WS-09 keeps rate-limit/cache/quotas/OTel; audit is WS-12. (WS-09 §P1 audit row now points here.)
- **WS-05 (dashboard structure):** **DECIDED (ROADMAP §6a D1/D2), not open.** There is ONE history
  system — this changelog. A WS-05 "dashboard version" is a **named checkpoint tagging a changelog
  snapshot** (label + message → a `change_id`), **not** a separate JSON-snapshot store and **not** a
  second diff/restore stack (WS-05 reuses this WS's diff + restore). The enabling constraint: the
  **dashboard `Reversible` is pinned to snapshot** (D2) so restore-to-version has an absolute state.
  **Duplicate-dashboard uses `clone_with` (this WS), not a bespoke copy.**
- **WS-02 / WS-10 (variables / kinds):** new persisted kinds register a `Reversible` like any other —
  free undo + audit by following the pattern.
- **AI ("Ask Nexus"):** agent-made changes record with `Actor::Agent{run_id,model}` → the **AI-agent
  log is the same ledger**, and AI edits are **undoable** by the user. Strong safety story for
  AI-generated dashboards/panels.
- **WS-11 (prefs):** audit timestamps render in the viewer's tz/format (WS-11); before→after numeric
  diffs can show converted units.

## 8. Out of scope (hand off / defer)
- Changing the `changelog`/`undo` crate internals — they're complete; nexus *consumes* them. A genuine
  gap (e.g. a missing query filter) is an upstream change, noted here.
- Cross-actor "global undo" — explicitly a crate non-goal (undo targets the authenticated principal).
- Full undo of identity (users/teams) — audit first; undo as fast-follow (§6.4).
