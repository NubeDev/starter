# G1 — Simple/Advanced mode + IA restructure

> Scope tier. See [README](./README.md). Frontend-only stage.

## Goal

Default the admin Access UI to three tabs an operator can reason
about — `Teams / Members / Pages` — and hide the engine
primitives behind an Advanced toggle.

## What ships

### Tab structure

**Simple mode (default):**

```
Teams  |  Members  |  Pages
```

**Advanced mode:**

```
Teams  |  Members  |  Pages  |  Rules  |  Assignments  |  Audit log
```

`Audit log` is the existing Decisions tab renamed. Same data,
same component, header text changes.

The Resources catalogue (read-only kind list) moves out of the
tab bar entirely into a small **"Registered resource kinds"**
link inside Advanced mode's toolbar overflow, since it's a
dev-tools view.

### Mode toggle

- Top-right of the panel header, next to `+ Invite User` / `+ Add Team`.
- Pill toggle: `Simple` / `Advanced`.
- Persisted per-user via `localStorage` key `rubix.authz.admin.mode`.
- Default for fresh users: `Simple`.

### Tenant home (Simple)

Replaces the current Overview stats grid. The stats cards
(Total Members, Active Teams, Active Rules) are removed from
Simple mode — operators don't act on them. Tenant status
(Live / Policy Coverage / Audit Log) moves to a slim sidebar
strip on the tenant home and stays visible in both modes.

The tenant home in Simple mode is just the **Teams** tab content
preceded by a one-line tenant header. Members and Pages are
accessed by tab click.

### Routes

URL structure stays compatible:

```
/admin/access/t/:tenantSlug                     → Teams tab
/admin/access/t/:tenantSlug/members              → Members tab
/admin/access/t/:tenantSlug/pages                → Pages tab (new, lands in G2)
/admin/access/t/:tenantSlug/rules                → Rules (Advanced only; redirects to /pages if Simple)
/admin/access/t/:tenantSlug/assignments          → Assignments (Advanced only)
/admin/access/t/:tenantSlug/audit                → Audit log (Advanced only; was /decisions)
```

If a user has Simple mode on and hits an Advanced-only URL
directly (e.g. bookmarked `/rules`), redirect to the Pages tab
and surface a one-time toast: *"Rules are in Advanced mode.
Toggle it in the top-right to access."*

## Files touched

Frontend only.

- [`packages/starter-ui-authz/src/panels/authz-admin.tsx`](../../../../packages/starter-ui-authz/src/panels/authz-admin.tsx)
  — tab list becomes mode-aware; reads/writes `rubix.authz.admin.mode`.
- [`packages/starter-ui-authz/src/panels/`](../../../../packages/starter-ui-authz/src/panels/)
  — new `mode-toggle.tsx` component (pill + persistence hook).
- Existing tab components (`overview.tsx`, `teams.tsx`,
  `members.tsx`, `rules.tsx`, `assignments.tsx`, `decisions.tsx`)
  are untouched in this stage. The Overview component stops
  being mounted in Simple mode; it stays for Advanced (or is
  deleted in a follow-up).
- Decisions tab header string changes from "Decisions" to
  "Audit log".

## Tests

- `mode-toggle.test.tsx` — toggle flips state, persists to
  localStorage, restores on mount.
- `authz-admin.test.tsx` extension — in Simple mode, query for
  Rules/Assignments/Audit tabs returns nothing; toggling to
  Advanced surfaces them.
- Route redirect test — Simple-mode user hitting `/rules` lands
  on `/pages` (`/teams` for this stage since Pages lands in G2)
  with toast visible.

## Verification

```bash
pnpm -F starter-ui-authz test
pnpm -F rubix-frontend dev   # then open /admin/access
```

Playwright smoke (verification sub-agent):
1. Log in as `op@example.com`, navigate to `/admin/access/t/system`.
2. Assert tab bar has exactly `Teams`, `Members` (Pages will be
   there after G2; in G1 just those two).
3. Click `Advanced` toggle. Assert `Rules`, `Assignments`,
   `Audit log` appear.
4. Reload page. Assert tab bar still shows Advanced state.
5. Toggle back to Simple. Reload. Assert Simple state persists.

## Out of this stage

- The Pages tab content (G2).
- Any backend change.
- Deleting the Overview component entirely — leave for follow-up
  once Simple mode is the established default.
