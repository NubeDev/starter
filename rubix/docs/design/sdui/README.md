# SDUI — server-driven UI surface + per-user dynamic resource authz

> Cites: SCOPE [Phase 3 entry gate Q3](../../SCOPE.md).

## What rubix gets from `starter-sdui-routes`

`starter-sdui-routes` is the resolver: given an SDUI page id, it
returns the page's widget tree. Rubix uses it for the Goal-1
dashboard surface — dashboards are SDUI pages produced by the
`rubix.dashboard.page_set` tool.

## Where pages physically live (Q3 — to resolve)

Two options, pick one before Phase 3 code lands:

**A. In starter-sdui-routes' own store.** Rubix calls a write API
on the resolver. Pro: no rubix table. Con: depends on whether
starter-sdui-routes ships a write path.

**B. In a rubix-owned Postgres table.** `rubix.dashboard.page_set`
writes here; the resolver reads via a `PageStore` trait rubix
implements. Pro: rubix owns the data layout. Con: rubix
re-implements what starter-sdui-routes might one day grow.

**Default assumption pending verification:** option A if the
resolver supports it; otherwise option B with an upstream issue.

## Per-user dynamic resource authz (Q3 + SCOPE Phase 4 hint)

User-defined SDUI pages are **resources** in starter-authz's
`ResourceRegistry`, not static routes. Each page registration adds
a `ResourceSpec`; reads/writes go through `PolicyEngine::decide`
with the calling principal. A user without grant on a page sees
a 404, not a 403 (deny leakage).

This makes "alice has dashboard X but bob doesn't" enforceable
without a hand-rolled authz layer.

## Resource URI scheme intersection

The MCP resource URI scheme (`rubix://<goal>/<resource>` per R12)
overlaps SDUI page ids. Rubix's convention:

- `rubix://dashboard/pages` — MCP resource listing all dashboards
  the caller can see.
- SDUI page ids are `dashboard.<slug>` (no `rubix://` prefix —
  that's MCP's namespace).

Phase 3 design must keep these two namespaces explicit so
extensions don't conflate them.
