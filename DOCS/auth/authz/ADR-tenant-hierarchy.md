# ADR — Hierarchical tenants (reseller / sub-tenant tree)

Status: **Proposed**
Supersedes nothing. Extends `SCOPE-EXT.md` R11/R12 (the flat-tenant
contract) with a recursive-tenant model. **Breaking** to the authz
core; acceptable because there is no production data yet.

## One-line summary

Make `tenant` recursive — every tenant gains an optional parent — so a
parent tenant transitively administers and sees everything in its
descendant tenants, while siblings stay isolated by construction. The
existing flat cross-tenant predicate (R11) becomes the depth-0 case of
a subtree-membership predicate.

## The problem

The product needs a reseller chain:

```
me (the box admin)                       super-admin, tenant_id = "*"
└── Daikin                               root tenant
    ├── Acme Facilities  (Daikin's client)          child tenant
    │   ├── Acme North   (Acme's client)             grandchild tenant
    │   └── Acme South
    └── Byco Energy      (another Daikin client)     child tenant
```

Rules:

- **Full transitive downward access.** A Daikin admin reads/writes
  everything in Acme and Acme North/South. An Acme admin reads/writes
  everything in Acme North/South but **nothing** in Byco or Daikin.
- **Strictly downward.** A child never sees its parent or siblings.
- **Arbitrary depth.** Resellers may resell to resellers; the model
  must not cap the depth.
- **One dedicated server per Daikin** — so isolation *between Daikins*
  is a deployment boundary, not an authz boundary. The recursion we
  must model lives *inside* one Daikin box: client → sub-client → …

### Why the current model can't express this

Today (`SCOPE-EXT.md` R11, `crates/starter-authz/src/engine.rs:226`):

- A session/token binds to **exactly one** `tenant_id`
  (`crates/starter-spi/src/auth/principal.rs:38`).
- The cross-tenant predicate is strict equality:
  `principal.tenant_id == object.tenant`, else `Deny{cross_tenant}`,
  evaluated **before** any rule so no rule can override it.
- The only escape is the `"*"` super-admin sentinel
  (`principal.rs:65`), which passes for *every* tenant — too coarse for
  Daikin, who must see its own subtree but not other Daikins (n/a here
  because of one-box-per-Daikin) and, more importantly, must NOT be the
  same authority as the box operator.
- Teams (R13) are flat slug groups **inside one tenant**; `team_members`
  joins straight to users. Teams are the wrong primitive for the
  client/sub-client boundary — they carry no isolation guarantee.

So a parent administering a child is unrepresentable: the predicate
admits exactly one tenant or all tenants, nothing in between.

## Decision

**Adopt a recursive-tenant model (parent_id + closure table). Each
node in the reseller tree is a tenant. The cross-tenant predicate
becomes "object.tenant is in the principal's administered subtree."**

Rejected alternatives, briefly:

- **Recursive teams (`parent_team_id`).** Smallest diff, but clients of
  one Daikin would share a single tenant and rely on rules not to leak —
  exactly the failure R11 exists to prevent. No isolation between
  sibling clients. Rejected.
- **Hierarchy in the domain layer (rubix), authz stays flat.** Re-
  implements an authz concept (who-administers-whom) outside authz, in a
  second place, badly. Rejected.

### Why a closure table, not just `parent_id`

Authz runs on the request hot path. The predicate must answer "is A an
ancestor of B?" as an **indexed point lookup**, not a recursive CTE per
check. `parent_id` alone forces a recursive walk on every `check()`. A
closure table (`(ancestor, descendant, depth)`, one row per
ancestor/descendant pair including self at depth 0) makes the predicate
a single indexed `EXISTS`. `parent_id` stays as the source of truth for
the edge; the closure table is the derived, queryable transitive
closure, maintained in the same transaction as any tree mutation.

This is the standard trade-off for read-heavy trees of modest depth (a
reseller chain is a handful of levels, not millions). It costs writes
(insert N rows when adding a node at depth N) to make reads O(1).

## Data model

### `starter-auth-users` — tenants grow a parent edge + closure

```sql
ALTER TABLE starter_auth_users_tenants
  ADD COLUMN parent_id TEXT
    REFERENCES starter_auth_users_tenants(id) ON DELETE RESTRICT;
-- NULL parent_id  = a root tenant (e.g. Daikin on its own box).
-- ON DELETE RESTRICT: deletion is already deferred (ADR-tenant-deletion);
-- a parent with live children cannot be removed out from under them.

CREATE INDEX idx_tenants_parent
  ON starter_auth_users_tenants (parent_id);

-- Transitive closure. One row per (ancestor, descendant) pair,
-- INCLUDING the self-pair at depth 0 — so "subtree of X" is simply
-- "every descendant_id WHERE ancestor_id = X", and X itself is in it.
CREATE TABLE starter_auth_users_tenant_closure (
  ancestor_id   TEXT NOT NULL REFERENCES starter_auth_users_tenants(id) ON DELETE CASCADE,
  descendant_id TEXT NOT NULL REFERENCES starter_auth_users_tenants(id) ON DELETE CASCADE,
  depth         INTEGER NOT NULL,         -- 0 = self, 1 = direct child, …
  PRIMARY KEY (ancestor_id, descendant_id)
);
CREATE INDEX idx_closure_descendant ON starter_auth_users_tenant_closure (descendant_id);
```

**Closure maintenance (in-transaction, on tenant create):** when
inserting tenant `C` under parent `P`:

```sql
-- self row
INSERT INTO ...tenant_closure (ancestor_id, descendant_id, depth)
VALUES (C, C, 0);
-- inherit P's ancestors, one deeper
INSERT INTO ...tenant_closure (ancestor_id, descendant_id, depth)
SELECT ancestor_id, C, depth + 1
FROM ...tenant_closure WHERE descendant_id = P;
```

Re-parenting is **out of scope** (same posture as immutable slugs and
deferred deletion). If it lands later it's a delete-subtree-closure +
re-insert operation behind an ops workflow, not a REST call.

**Depth-cap guard.** A `CHECK`/trigger refuses inserts whose resulting
`depth` exceeds a configured maximum (default e.g. 16) — a cheap
cycle/runaway backstop. `parent_id` cycles are otherwise impossible
because a tenant must exist before it can be a parent and re-parenting
is disallowed.

### Slug uniqueness

Tenant slugs are URL-facing and currently **globally** unique
(`UNIQUE(slug)` in `0005_tenants.sql`). That global-uniqueness is still
correct for the "one box per Daikin" deployment and keeps routing
unambiguous. **Keep it.** (If a future multi-Daikin-per-box need arises,
this becomes `UNIQUE(parent_id, slug)` + slug-path routing — noted as an
open question, not done here.)

## `Principal` changes

The binding stays **one tenant** (`tenant_id` unchanged — switching the
*acting* tenant is still a re-login per R11). What's new is the
principal's **administered subtree**, resolved once at session-mint:

```rust
pub struct Principal {
    // … unchanged: subject, role, scopes, tenant_id, teams, extra …

    /// Phase 7e — the set of tenant ids this principal administers,
    /// i.e. the subtree rooted at `tenant_id` (inclusive). Resolved
    /// at session-mint from the tenant closure table. For a leaf
    /// tenant this is just `[tenant_id]`. For the `"*"` super-admin
    /// sentinel this is empty and `is_super_admin()` short-circuits
    /// instead (the whole-forest case). Empty for pre-Phase-7e
    /// principals — the engine then falls back to strict equality,
    /// preserving R11 behaviour exactly.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tenant_scope: Vec<String>,
}
```

**Why materialize the subtree onto the principal** instead of querying
the closure table inside `check()`:

- `check()` is sync-ish and hot; it already takes no store handle. Keeping
  it pure (no DB call mid-decision) matches the current design — the
  engine reads `principal.teams`, it doesn't query the team table.
- The subtree is small (reseller depth) and changes only when the tree
  changes, which is admin-rare. Session-mint is the right cache point,
  exactly like `teams`.
- Trade-off (must be documented): a tenant added beneath you *after*
  your session was minted is not visible until your next login — same
  staleness window `teams` already has. Acceptable for admin-managed
  structure; called out so it's not a surprise.

For very wide trees this list could grow large; if that ever bites, the
fallback is a closure-backed `is_ancestor(p.tenant_id, object.tenant)`
predicate that *does* take a store handle. Not needed for reseller-shaped
trees. Flagged in Open Questions.

## Engine changes

Single focused change at the cross-tenant predicate
(`engine.rs:232–262`). Today's match is strict equality; it becomes
subtree membership:

```rust
if spec.tenant_scoped && !principal.is_super_admin() {
    match (&principal.tenant_id, &object.tenant) {
        (None, _)        => return deny("no_tenant_binding"),
        (Some(_), None)  => return deny("cross_tenant"), // tenant-scoped row w/o tenant = bug
        (Some(pt), Some(ot)) => {
            let in_scope = pt == ot                       // own tenant (depth 0), or
                || principal.tenant_scope.iter().any(|t| t == ot); // a descendant
            if !in_scope {
                return deny("cross_tenant");
            }
            // fall through to role / condition / ownership
        }
    }
}
```

Properties preserved from R11:

- Runs **before** role/condition — a misconfigured `role:"*"
  resource:"*"` allow still cannot reach outside the subtree.
- `Deny{cross_tenant}` / `Deny{no_tenant_binding}` reason codes unchanged.
- `"*"` super-admin sentinel still short-circuits (whole forest).
- A principal with empty `tenant_scope` falls back to pure equality →
  **byte-for-byte the current behaviour** for any consumer that hasn't
  adopted the hierarchy. Strictly additive in that sense.

### Ownership rules and the subtree

`CompiledCondition::Owner` matches `object.owner == principal.subject`
(`engine.rs:292`). Under hierarchy, a Daikin admin acting on an Acme-
owned row is **not** the owner, so owner-conditioned rules would deny.
That's correct: "owner can edit their own row" should not silently mean
"every ancestor admin is the owner." Ancestor admin access is granted by
**role/tenant-scoped rules**, not by ownership. Documented so nobody
"fixes" ownership to walk the tree.

### Tenant-scoped rules and the subtree

Rule-tenant matching (`engine.rs:274`) currently requires
`principal.tenant_id == rule.tenant_id`. A rule written *for a parent
tenant* should also apply when that parent's admin acts on a child's
resource. Decision: a rule whose `tenant_id` is an **ancestor of
`object.tenant`** matches. Concretely, match when
`rule.tenant_id == object.tenant` OR `rule.tenant_id` is in the chain
from `object.tenant` up to a root that the principal is bound to. The
cleanest implementation reuses the closure: the rule's tenant must be an
ancestor-or-self of the object's tenant *and* within the principal's
scope. (Implementation note: this needs the object's ancestor set, which
the predicate already established the object is in-scope; carry the
object's tenant chain or keep rule-tenant matching to `principal.tenant_id`
+ ancestors. Resolve during implementation — see Open Questions.)

## Store / trait changes (`TenantStore`)

`crates/starter-auth-users/src/store/tenant_store/mod.rs`:

- `create_tenant` gains a `parent_id: Option<&str>` and writes the
  closure rows in the same transaction.
- New: `subtree_ids(tenant_id) -> Vec<String>` — `SELECT descendant_id
  FROM ...tenant_closure WHERE ancestor_id = $1`. Used at session-mint to
  fill `Principal.tenant_scope`.
- New: `is_ancestor(ancestor, descendant) -> bool` — for the
  authorization of *provisioning* actions (can this admin create a tenant
  under target X?).
- `TenantRecord` grows `parent_id: Option<String>`.
- Both sqlite + postgres impls updated; closure-maintenance SQL mirrored.

## Authenticator / session-mint changes

`session/verify.rs` — extend `verify_session_with_teams_and_extras` (or
add a `_with_scope` sibling) to also call `subtree_ids(tenant_id)` and
set `principal.tenant_scope`. Same shape as the existing team lookup at
`verify.rs:138`. The `"*"` sentinel skips it (super-admin short-circuit).
Token-verify path gets the same treatment so PATs/API tokens for a
parent admin carry the subtree.

## Provisioning authority (who can create what)

Creating a tenant under parent `P` requires the caller to be an admin
**within `P`'s subtree at or above the insertion point** — i.e.
`is_ancestor_or_self(caller.tenant_id, P)` AND caller role is `admin`
(or the `"*"` box operator). This is what makes the chain self-service:

- box operator (`"*"`) creates Daikin (root, `parent_id = NULL`).
- Daikin admin creates Acme (`parent_id = Daikin`).
- Acme admin creates Acme North (`parent_id = Acme`).
- Acme admin **cannot** create anything under Byco or Daikin (not an
  ancestor-or-self).

`POST /v1/tenants` gains an optional `parent_id`; the handler authorizes
it against the caller's principal before insert. The reserved-slug list
and immutability posture are unchanged.

## Routes

Additive to the existing `/v1/tenants/*` surface
(`routes/tenants.rs`):

```
POST /v1/tenants            body gains optional  "parent_id"
                            (omitted = root; allowed only for "*" box operator)
GET  /v1/tenants            scoped to caller's subtree (box operator sees all)
GET  /v1/tenants/{id}       allowed iff id is in caller's subtree
GET  /v1/tenants/{id}/children      list direct children   (NEW, convenience)
GET  /v1/tenants/{id}/subtree       list full subtree       (NEW, admin UI tree view)
```

All existing member/team routes stay; they now implicitly work for any
tenant in the caller's subtree because the gate is the subtree predicate.

## Migration strategy

1. Add `parent_id` (nullable) + closure table.
2. Backfill: every existing tenant is a root — insert its depth-0 self
   row into the closure. No parent edges exist yet, so no deeper rows.
   This is a pure addition; existing flat tenants keep working.
3. Ship `Principal.tenant_scope` defaulting to empty → engine falls back
   to strict equality → **zero behaviour change** until an authenticator
   starts populating it.
4. Flip the authenticator to populate `tenant_scope`; the predicate
   starts honouring the subtree. This is the one observable cutover and
   it's gated by the wiring choice, not the migration.

## Smoke tests (before merge)

- **subtree-downward-allow** — Daikin admin reads an Acme-North row →
  allow. (object.tenant is a depth-2 descendant.)
- **sibling-isolation-deny** — Acme admin reads a Byco row →
  `Deny{cross_tenant}`. Sibling not in subtree.
- **upward-isolation-deny** — Acme admin reads a Daikin-level row →
  `Deny{cross_tenant}`. Parent not in a child's scope.
- **own-tenant-still-works** — leaf tenant admin on own row → allow
  (depth-0 path, equals old R11 behaviour).
- **flat-fallback-unchanged** — principal with empty `tenant_scope`
  behaves byte-for-byte as pre-ADR (strict equality). Guards "strictly
  additive."
- **provisioning-authority** — Acme admin can create under Acme,
  **cannot** create under Byco/Daikin; box operator can create a root.
- **closure-maintenance** — create a depth-3 chain; assert the closure
  has the right (ancestor, descendant, depth) rows including all self
  rows.
- **misconfigured-global-allow-cannot-escape-subtree** — a `role:"*"
  resource:"*"` allow still denies cross-subtree (predicate runs first).
- **session-staleness documented, not asserted** — note that a child
  added after mint needs re-login; covered by re-running the mint.

## Open questions

- **Rule-tenant ancestor matching** (engine.rs:274) — exact shape of
  "a parent's rule applies to a child's resource." Two candidates:
  (a) match rule.tenant_id against the object's ancestor chain;
  (b) keep rules matched to `principal.tenant_id` + ancestors. Pick
  during implementation with a test that pins the intended semantics.
- **Wide-subtree principals** — if `tenant_scope` ever gets large,
  switch the predicate to a closure-backed `is_ancestor` lookup taking a
  store handle. Not needed for reseller-shaped trees.
- **Re-parenting** — deliberately unsupported now; would be an ops
  workflow (delete + recompute closure), never a REST call.
- **Slug scoping** — kept globally unique (good for one-box-per-Daikin +
  routing). Revisit to `UNIQUE(parent_id, slug)` + path routing only if a
  multi-root-per-box need appears.
- **Audit** — `DecisionEntry.tenant` (R14) records the *object's*
  tenant; a subtree-allow by an ancestor is attributable via
  `subject` + `tenant`. Confirm dashboards can express "show me
  everything Daikin admins did across their subtree" (filter by subject
  set, or add an `acting_tenant` column). Flag for the audit phase.

## Bottom line

Make tenants recursive with a closure table; resolve each principal's
administered subtree at session-mint into `Principal.tenant_scope`; turn
the R11 cross-tenant equality check into a subtree-membership check that
still runs before every rule. Parents get full transitive downward
access, siblings and parents stay invisible to children, depth is
unbounded, and a consumer that never populates `tenant_scope` keeps the
exact flat-tenant behaviour. The change is concentrated:
`Principal` (+1 field), the engine predicate (~10 lines), `TenantStore`
(+closure SQL, +2 methods), the session-mint path (+1 lookup), and the
`/v1/tenants` provisioning gate.
