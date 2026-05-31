# G4 — Team detail Permissions tab

> Scope tier. See [README](./README.md). Frontend-only stage.

## Goal

Make a team's detail page show **what this team can access**, not
just who's in it. This is the inverse view of G2's Pages tab: G2
answers "for this page, which teams?" — G4 answers "for this
team, which pages (and other resources)?"

## What ships

Team detail route gains a tab bar:

```
/admin/access/t/:tenantSlug/teams/:teamSlug

   Members 5  |  Permissions 8
```

`Members` is the existing component, unchanged.

`Permissions` table:

| Resource | Kind | Tier | Granted by | Actions |
|---|---|---|---|---|
| 📄 Boiler Overview | Page | Edit | direct grant | ✕ Revoke |
| 📄 Site Dashboard | Page | View | tenant default | (read-only) |
| 🔧 export.csv | Tool | Invoke | direct grant | ✕ Revoke |

Data source is `GET /v1/authz/grants?subject=team:<slug>`
shipped in G3.

The `Granted by` column distinguishes:
- **direct grant** — `source = 'grant'`, row is editable.
- **tenant default** — wildcard-subject rule that happens to
  include this team via membership; row is read-only and links
  to the wildcard rule in Advanced mode.
- **legacy rule** — `source = 'manual'` with a condition that
  references this team slug; read-only with an Advanced-mode
  edit link.

Clicking a row deep-links to the resource detail (for pages:
opens the same drawer as G2). Clicking ✕ Revoke fires
`DELETE /v1/authz/grants/:id`.

## Files touched

Frontend only.

- [`packages/starter-ui-authz/src/panels/team-detail.tsx`](../../../../packages/starter-ui-authz/src/panels/team-detail.tsx)
  — add tab bar; existing content becomes the `Members` tab.
- `panels/team-permissions-tab.tsx` *(new)*.
- `client/grants.ts` — already shipped in G3; reuse `listGrants`.
- Routing: `team-detail.tsx` already mounts under
  `/teams/:teamSlug`; add nested route `/permissions`.

## Tests

- `team-permissions-tab.test.tsx`:
  - Renders rows from a fixture covering all three
    `Granted by` classes.
  - Direct-grant row's Revoke button fires `DELETE`.
  - Tenant-default and legacy rows render without action
    controls and link to Advanced.
- Routing test: `/teams/:slug` defaults to Members tab;
  `/teams/:slug/permissions` lands on Permissions tab.

## Verification

```bash
pnpm -F starter-ui-authz test
```

Playwright smoke:
1. From G3 state, navigate to
   `/admin/access/t/system/teams/hvac-ops/permissions`.
2. Assert the page granted in G3 appears with tier `Edit`.
3. Click ✕ Revoke, assert row disappears, refresh, assert it
   stays gone.
4. Navigate back to the page in Pages tab, assert the team is
   no longer in its grants list (consistency).

## Out of this stage

- Bulk operations ("revoke all of this team's edit access").
- Cross-resource visualisation (matrix view).
