# Out of scope for v1

> Scope tier. See [README](./README.md).

These are deliberately deferred. Each has a one-line note on
*why deferred* and *what would unblock it*.

## Extension install/use gating

Extensions are gateable in principle (they already have stable
identifiers), but v1 ships **Pages only** to keep the surface
small and prove the UX before generalising. The
`instances_provider` hook from G2 is the seam — adding
extensions later is implementing the trait for an
`rubix.extension` kind and registering a tier map.

Unblock: a clear answer on whether extension permissions are
**install-time** (admin only) or **per-user** (some extensions
gate their own surfaces). Today the codebase treats it as
install-time only, so there's nothing for a per-team grant to
attach to.

## Tools tab in Simple mode

`rubix.tool` is a single gate (`invoke`), and most operators
either want all-or-nothing per team. Exposing it as a list in
Simple mode adds clutter for a control most tenants don't
exercise. Advanced mode's Rules tab continues to gate it.

Unblock: a tenant where someone actually wants per-team tool
gating. Then we register an `instances_provider` for tools and
add a Tools tab next to Pages.

## Roles outside reader/writer/admin

The tenant-membership role today is a fixed three-value enum.
Custom roles per tenant (e.g. "operations-lead") are a real
future need but require a `tenant_roles` table and a UI to
manage them; out of v1.

Unblock: a roles CRUD scope of its own. The grants API doesn't
care — `role:` strings are opaque — so the engine work is the
small part.

## Resource-instance ACL inheritance / folders

Grafana folders cascade permissions to dashboards inside them.
Our pages don't currently live in folders. Adding folder
semantics is its own design problem (storage, move semantics,
permission inheritance order). Out of v1.

Unblock: a folder concept in `starter-dashboards`. Until then,
each page is a flat ACL target.

## Bulk operations

"Grant Edit on every page in tag X to team Y", "revoke all of
team Z's grants" — useful but secondary. Out of v1; once the
single-resource flow is loved, bulk is a UI-only addition over
the same `/grants` API.

## Audit-log search by resource id

Today's Decisions endpoint filters by tenant + subject + action,
not by resource id. The Audit log tab in Advanced mode keeps
the existing filters in v1. Filtering by resource id would let
us answer "who tried to edit Boiler Overview last week" from the
UI; out of v1, captured here so we remember.

Unblock: a small `?resource_id=` filter on `GET /v1/authz/decisions`.

## Migration of legacy condition-based rules

The codebase has seeded rules that use `condition` expressions
like `principal.teams contains 'hvac-ops'`. G2's `EffectiveAcl`
summariser flags them as legacy and the UI shows them
read-only; G3's grants API does not rewrite them.

Unblock: a one-off migration that recognises the canonical
`principal.teams contains '<slug>'` pattern and rewrites it as a
`source='grant'` row with `role='team:<slug>'`. Mechanical but
risky to do silently — should be its own scope with a dry-run
report.
