# Proposal: Flow storage model + undo/redo coverage

Status: Draft (v2, peer-review applied 2026-05-28)
Owner: ap@nube-io.com
Date: 2026-05-28

## Summary

Two related questions surfaced during a schema audit:

1. **Flow storage**: should the flow body remain a YAML/JSON blob in `flows_definitions`, or should nodes/edges become first-class rows? Recommendation: keep YAML authoritative, add a **transactional same-tx `flow_node_index`** for cross-flow queries. Defer full relational model until a concrete trigger (multi-user edit, per-node ACLs) lands. Pre-requisite: pick a canonical flow store (§1).
2. **Undo/redo**: today's `undo_snapshots` covers `user`, `team`, `flow_definition`, and `rubix.dashboard.page`. Gaps in dashboard definitions, node-level (deferred), and user role/prefs. Redo is in-process only. Snapshot vs patch policy is undocumented and already inconsistent (`TeamReversible` uses patches, others snapshot).

### Changelog from v1

Applied peer-review feedback:
- §1: flow-ownership question promoted from "open" to **blocking decision** for FK work; engine-topology (SQLite vs Postgres) made explicit.
- §1: added content-addressed `revision_id` as a sub-recommendation.
- §2: `flow_node_index` consistency model specified — same-tx, not "drop and rebuild any time."
- §2: BI1 alternative (skip Phase A, go straight to `flow_nodes`) scored as a considered option, not adopted.
- §3.0: new section — snapshot vs patch policy (currently inconsistent).
- §3.2: editor undo pinned to "op log" (not "CRDT or op log"); revisit gate documented.
- §3.3: stripped "audit trail" conflation. Undo is best-effort UX; audit is a separate system.
- §3.4: redo cursor concurrency model (epoch + optimistic version bump) specified.
- §3.5: per-kind retention via a small policy table, not per-kind code.
- §3.6: drop `tenant` Reversible from CHECK constraint and roadmap.

## 1. Schema relationships audit (summary)

Detail in `stage1-mart-audit.md` is database-shape; this proposal covers cross-table integrity.

### Current state — everything is string-coupled

- `runs.flow_revision_id` → `flow_revisions.revision_id` — TEXT, no FK
- `run_checkpoints.run_id` → `runs.run_id` — TEXT, no FK
- `node_state(flow_id, node_id)` — TEXT pair, no FK to any flow table
- `flow_heads(flow_id, revision_id)` → `flow_revisions` — no FK
- `flows_definitions` (rubix) and `flow_revisions` (starter) — overlapping concerns, no integration
- `dashboards_definitions` revisions → head — no FK
- Warehouse ↔ PG — only `tenant_id` as a soft key (correct; engines differ)
- `entities` / `entity_refs` — fully isolated graph, no FK to runtime tables
- `skill_approvals.skill_id`, `starter_scheduled_flows.flow_id` — logical names, no FK (correct, but needs reconciliation)

### Blocking decision: flow-store ownership

**Before any FK work or Phase B migration**, this must be resolved:

`flow_revisions` (starter, SQLite-backed) and `flows_definitions` (rubix, Postgres-backed) both persist flows. The duplication is the root of the "everything feels isolated" impression. This is not just two tables — it is **two engines**. Picking an owner has runtime topology consequences:

| Option | What it means | Cost |
|---|---|---|
| **Rubix owns persistence** (recommended) | Starter loses its flow store. Starter either (a) requires Postgres, or (b) gains a `FlowStore` trait that rubix implements | Starter standalone needs Postgres OR a re-impl |
| **Starter owns persistence** | Rubix drops `flows_definitions`, layers tenant scoping on top of `flow_revisions` | Rubix must accept SQLite, or starter grows a Postgres backend |
| **Both stay** | Status quo. No cross-store FKs possible | Current isolation is permanent |

Recommendation stands: **rubix owns persistence**, exposes a `FlowStore` trait that starter can implement against SQLite for standalone use. But this is a **decision needed**, not a footnote.

### FK additions (gated on the decision above)

Within whatever the canonical engine ends up being:

| Child | Parent | Action |
|---|---|---|
| `runs.flow_revision_id` | canonical flow revisions | FK, RESTRICT |
| `run_checkpoints.run_id` | `runs(run_id)` | FK, CASCADE |
| `flow_heads(flow_id, revision_id)` | canonical flow revisions | composite FK, RESTRICT |
| `node_state.(flow_id)` | flows parent | FK, CASCADE |

**Do NOT add FKs:**

- PG ↔ warehouse — different engines, append-only telemetry is by design
- `entities` ↔ runs — dimension data, lifecycle-independent
- `skill_approvals` / `scheduled_flows` — soft refs to logical names; add a reconciliation job instead

### Sub-recommendation: content-addressed revision_id

Once the canonical store is picked, change `revision_id` from opaque TEXT to a hash of the canonicalized YAML body (BLAKE3 or SHA-256). Free dedup, free integrity check, and "redo cursor points at a deleted revision" becomes detectable rather than silently broken. Cheap addition during the consolidation migration; expensive to retrofit later.

## 2. Flow storage: blob vs. relational

### The actual question

Not "1M nodes in one flow" (no human authors that), but: at what scale does the blob model break?

### Thresholds

| Scenario | Blob OK? | Why |
|---|---|---|
| 100k flows × 10 nodes each | ✅ | Postgres handles millions of TEXT rows; one flow loads at a time |
| One flow > 1–5 MB serialized | ❌ | Write amplification on every edit; revision bloat |
| Cross-flow queries needed ("find all `ai-agent` nodes tenant-wide") | ❌ | Can't query into blobs without parsing all of them |
| Concurrent multi-user editing | ❌ | No per-node locking |
| Per-node ACLs or audit | ❌ | Granularity wrong |

We are well inside the "blob is fine" regime today. Cross-flow queries will hit first.

### Recommended path

**Phase A (now): keep YAML, add a transactional index table**

```sql
CREATE TABLE flow_node_index (
  tenant_id     UUID NOT NULL,
  flow_id       TEXT NOT NULL,
  revision_id   TEXT NOT NULL,
  node_id       TEXT NOT NULL,
  kind          TEXT NOT NULL,
  kind_version  TEXT,
  PRIMARY KEY (revision_id, node_id),
  FOREIGN KEY (revision_id) REFERENCES <canonical_flow_revisions>(revision_id) ON DELETE CASCADE
);
CREATE INDEX flow_node_index_kind ON flow_node_index(tenant_id, kind);
```

**Consistency model (specified, not hand-waved):**

- Index population happens **in the same transaction** as the `flow_revisions` insert. Single engine, single tx, atomic. There is no "rebuild from YAML later" runtime path.
- `ON DELETE CASCADE` on `revision_id` means revision deletion cleans the index automatically.
- "Rebuild from YAML" exists only as a **disaster-recovery property** (a one-off `rebuild_flow_node_index` admin tool), not a normal operation. The blob is the source of truth; the index is its same-tx projection.
- Concurrent flow saves are isolated by transaction; the index inherits whatever isolation level the revision insert uses (RC by default; RR if we need stricter).

This kills the drift failure mode that the original "drop and rebuild any time" framing invited.

**Phase B (when needed): promote to authoritative**

Triggered by any of: flows > 1–5 MB, multi-user concurrent edit, per-node ACLs, generated flows.

```sql
flows(flow_id, tenant_id, head_revision_id, ...)
flow_revisions(revision_id, flow_id, created_at, author, parent_revision_id, ...)
flow_nodes(revision_id, node_id, kind, config JSONB, position JSONB, ...)
flow_edges(revision_id, from_node, from_port, to_node, to_port)
```

YAML becomes a serialization format (import/export, git review), not the storage format. Node *config* stays JSONB per-kind.

### Considered alternative: BI1 — skip Phase A, jump to flow_nodes only

> Argument: if we have to write index rebuild logic either way, the delta to a real `flow_nodes` table is small. YAML stays as import/export. Skip the "derived index that drifts" failure mode.

**Scored, not adopted.** Trade-offs:

- ✅ Avoids derivation entirely
- ✅ One fewer migration if Phase B is inevitable
- ❌ Edges-stay-in-YAML splits the source of truth: nodes in rows, edges in blob. Queries like "what does this node feed into" still require parsing the blob — the worst of both worlds
- ❌ Every viewer/editor pays serialization-on-read forever, even before any concrete pressure justifies it
- ❌ YAML is the artifact people diff in git, paste into issues, copy between flows — making it derived rather than primary has ergonomic cost not captured in the storage-shape argument

**Verdict:** Phase A with the transactional consistency model in this proposal addresses the drift concern BI1 raises, without paying the serialization-cost up front. Re-evaluate at the Phase B trigger — and at that point go to **full Phase B with edges**, not a half-step.

### What we don't do

- Don't move flow *logic* into YAML/JSON (expressions, conditionals as strings). Logic stays in Rust; declarations stay in YAML.
- Don't FK across the rubix/starter boundary until we pick one owner (§1).

## 3. Undo / redo

### Today

- Table: `undo_snapshots(id, tenant_id, actor_id, resource_kind, resource_id, snapshot_jsonb, created_at, superseded_at)`
- **Mixed model**: some Reversibles snapshot full before/after; `TeamReversible` already uses patches (`TeamPatch`). No documented policy for which to use when.
- Per-actor in-process redo cursor (`InMemoryUndoCursor`), no SQL persistence
- Retention: smaller of {50 rows per resource, 90 days}, swept every 24h
- Actor attribution: yes (`actor_id` + polymorphic `Actor::{User,Agent,System}`)

### Coverage today

| Resource | Status |
|---|---|
| `user` (create/disable) | ✅ `UserReversible` (snapshot) |
| `team` | ✅ `TeamReversible` (patch) |
| `flow_definition` (deploy/duplicate, whole flow) | ✅ `FlowDefReversible` (snapshot) |
| `rubix.dashboard.page` (create/update) | ✅ `DashboardReversible` (snapshot) |
| `tenant` | ⚠️ kind defined — **propose to drop, see §3.6** |
| `clickhouse_rule` / `clickhouse_mart` / `clickhouse_retention` | ⚠️ defined, no impl |

### 3.0 Snapshot vs patch policy (NEW — must be settled before extending coverage)

The codebase already disagrees with itself. Without a rule, every new Reversible re-litigates the choice. Proposed policy:

| Use **snapshot** (full before/after JSON) when… | Use **patch** (RFC 6902 or equivalent) when… |
|---|---|
| Resource is small (< ~10 KB serialized) | Resource is large (flow YAML, dashboard layout) and most edits touch a tiny slice |
| Resource has no useful intermediate state ("you have it or you don't" — users, teams) | Edits are naturally diff-shaped (rename, field flip, single-cell update) |
| Lifecycle includes creation/deletion (the "before" state may be `{}`) | Edits never create or destroy the resource |
| Round-trip cost is dominated by network, not storage | Storage cost of full snapshots × revision count would dominate |

By this rule, `TeamReversible` (patch) and `UserReversible` (snapshot) are both correct as-is. `FlowDefReversible` should **stay snapshot** for now (deploys are coarse-grained; a flow YAML is the unit of change) but is the candidate to flip to patch once Phase B node-level granularity lands.

Action: write this policy as a short rustdoc comment on the `Reversible` trait, and reference it in every existing impl. Audit-blocker for any new Reversible.

### 3.1 Dashboard pages — verify scope

`rubix.dashboard.page` covers create/update of page bodies. **Open question**: does it cover deletion, reorder, layout-only changes? Audit pass needed against `DashboardReversible::change_for` — if the tool only fires on body mutations, layout/reorder changes silently escape undo.

Also: dashboard *definitions* (the parent `dashboards_definitions` row — title, owner, tags) have no Reversible impl. Either fold metadata into the page snapshot, or add a `rubix.dashboard.definition` kind.

### 3.2 Nodes inside a flow

Today, the snapshot granularity is **the whole flow YAML on deploy**. Editing one node and deploying re-snapshots everything. This is:

- Correct for *deploy* (a deploy is atomic, undo restores the prior revision)
- Wrong for *interactive editing* (dragging a node, tweaking config) — there's no undo in the editor itself, only on save

Two layers of undo are needed:

| Layer | Scope | Storage |
|---|---|---|
| Editor undo (Ctrl-Z) | Per-edit, per-session | Client-side **op log** (linear history) |
| Deploy undo | Per revision | Existing `flow_definition` Reversible |

**Op log, not CRDT.** Single-user editor sessions need a linear op log + undo stack. CRDT is only justified if collaborative editing is on the roadmap (it isn't, currently). The op log is trivially upgradable to a server-side session document if/when collaboration is funded — revisit at that point.

If we later move to the Phase B relational `flow_nodes`/`flow_edges` model (§2), per-node Reversible becomes natural — each `INSERT/UPDATE/DELETE flow_nodes` can produce a patch-style change. Defer until Phase B; per §3.0, that's the moment `FlowDefReversible` flips from snapshot to patch.

### 3.3 Users

Currently covered: create, disable. **Missing**:

- Role / permission changes (likely the most-asked-for undo: "I just demoted the wrong person")
- Profile / prefs (email, units, locale per `i18n_and_unit_prefs`)
- Tenant assignment

Add to `UserReversible` (or split into `UserRoleReversible` if role changes flow through a different tool).

**Important: undo is not audit.** Role changes need an **append-only audit log** (signed, retention-unbounded, immune to undo-pruning). The audit log is the security-relevant artifact. Undo is best-effort UX — "I demoted the wrong person; let me reverse it in the next 5 minutes" — and is allowed to expire under retention. Two separate systems. Do not conflate.

If the audit log doesn't exist yet, it's a separate proposal — but **do not extend undo retention to substitute for audit**. The temptation is real and wrong.

### 3.4 Redo across processes

In-memory cursor breaks the moment we run >1 agent process. Promote to:

```sql
CREATE TABLE rubix_undo_cursors (
  tenant_id    UUID NOT NULL,
  actor_kind   TEXT NOT NULL,        -- 'user' | 'agent' | 'system'
  actor_id     TEXT NOT NULL,
  redo_stack   JSONB NOT NULL,       -- Vec<GroupId>
  epoch        BIGINT NOT NULL,      -- monotonic version, bumped on every write
  updated_at   TIMESTAMPTZ NOT NULL,
  PRIMARY KEY (tenant_id, actor_kind, actor_id)
);
```

**Concurrency model:**

- Every read returns `(redo_stack, epoch)`. Every write is `UPDATE … SET redo_stack = $new, epoch = epoch + 1 WHERE epoch = $observed_epoch`. Mismatched epoch → conflict → reload and retry (or surface as "stack changed; try again").
- Any new mutation by an actor (recorded via `record_if_reversible`) clears that actor's redo stack and bumps `epoch`. Two processes racing redo for the same actor: one wins, the other sees an epoch conflict and reloads.
- **TTL:** rows are dropped when the youngest referenced GroupId falls outside undo retention. Aligned with `undo_snapshots` sweep (§3.5).

Also expose `rubix.undo.redo` verb (today only `rubix.undo.last` is surfaced).

### 3.5 Per-kind retention policy

Today retention is global (50 rows per resource OR 90 days). Per-kind tuning is desirable (security-relevant changes deserve longer; ephemeral edits shorter), but adding per-kind code branches is the wrong shape. Instead:

```sql
CREATE TABLE undo_kind_policy (
  resource_kind         TEXT PRIMARY KEY,
  max_rows_per_resource INT NOT NULL DEFAULT 50,
  max_age_days          INT NOT NULL DEFAULT 90
);
```

The sweep job reads policy per kind and applies it. New kinds get default policy unless an operator overrides. No code change to add a new kind's retention curve.

### 3.6 Drop `tenant` Reversible

Listed today as a defined-but-unimplemented kind. Drop it from the CHECK constraint and from the roadmap.

Reason: tenant create/delete is rare, operator-driven, and high-blast-radius. "Undo delete tenant after 89 days, when other tenants may have reused names/IDs and downstream warehouse data has been retention-pruned" is more dangerous than useful. If a tenant is deleted in error, the correct recovery is restore-from-backup + audit-log replay, not `rubix.undo.last`. Belongs in operator runbook, not UX undo.

### Concrete next steps

1. Add §3.0 policy doc-comment to the `Reversible` trait; reference it in every existing impl.
2. Audit `DashboardReversible::change_for` for delete/reorder coverage — patch if missing.
3. Add `rubix.dashboard.definition` Reversible (or fold into page).
4. Extend `UserReversible` to cover role, prefs, tenant assignment. **Separately**: scope an audit-log proposal for role changes.
5. Land `rubix_undo_cursors` table with epoch concurrency; expose `rubix.undo.redo`.
6. Land `undo_kind_policy` table; migrate global retention to per-kind reads.
7. Drop `tenant` from the kind CHECK constraint.
8. Defer node-level granularity until Phase B flow storage.

## Open questions (now narrower)

- **Flow-ownership decision** — promoted from "open" to **blocking**. Needs an answer before any §1 FK work or §2 Phase B move. Choices and costs documented in §1.
- Is editor-side undo (Ctrl-Z during authoring) in scope for the next quarter, or post-Phase-B?
- Do role changes flow through a dedicated tool, or through the generic user update path? (Affects whether `UserReversible` extension or a new `UserRoleReversible` is the right shape.)
- Does an append-only audit log already exist somewhere, or is that a separate proposal? (Required dependency for §3.3 role-change handling.)
