## Done

- Added `rubix/frontend/e2e/chrome.spec.ts` covering: login → switch to sidebar shell → assert `user-email` testid equals `op@example.com` → open Account menu → assert `logout-menu-item` visible → assert sidebar lists Home / Flows / Extensions / Admin / Settings → click logout → assert Sign-in heading returns.
- Committed as `d71be1d` with message starting `phase D.3 — chrome smoke test — test(rubix-frontend) chrome e2e`.

## Next

- (none) — gate stage will follow.

## What you need to know

- The spec text said "redirect to /login", but the app uses `AuthProvider.unauthenticatedSlot` (SCOPE OQ-4), not a routed `/login`. The test asserts the unauthenticated slot's Sign-in heading reappears and the URL stays at `/`. Matches the pattern in `auth.spec.ts`.
- The five "sections" in the spec map to nav item labels (`Home`, `Flows`, `Extensions`, `Settings`) plus the `Admin` group title — all visible inside the sidebar in `sidebar` shell mode.
- In header mode on desktop the `AppSidebar` is mobile-only; the test switches to sidebar mode via the theme drawer (radiogroup aria-label "where the primary navigation lives", radio "Sidebar") to make all five labels assertable.
- Selectors used: `getByTestId('user-email')`, `getByRole('button', { name: /account menu/i })` (i18n `header.userMenu` = "Account menu"), `getByTestId('logout-menu-item')`, sidebar via `[data-slot="sidebar"], [data-sidebar="sidebar"]`.
- `pnpm --filter @nube/rubix-frontend e2e --grep chrome` was attempted; it fails because no `rubix-agent` is running in this worktree (so does `auth.spec.ts`, by the same prereq). The test was not run green here — same posture as prior Phase B/C e2e commits.

## Open questions

- (none)
