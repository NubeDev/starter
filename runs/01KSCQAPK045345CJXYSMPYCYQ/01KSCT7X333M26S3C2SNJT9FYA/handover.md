## Done

- Added rubix/frontend/e2e/{auth,extensions,users}.spec.ts covering the three flows mandated by stage 14, each documenting the `mani run demo` (op@example.com / rubix-dev-passwd, agent on 127.0.0.1:8088) prerequisite at the top.
- `pnpm --filter @nube/rubix-frontend typecheck` green; `playwright test --list` enumerates the new specs alongside the existing six.
- Committed as 6d930ea on branch codeless/rubix-frontend-wire with the stage-title prefix.

## Next

- Stage 15 (next session) — per the stage 14 brief, e2e `green (against a running backend)` was deferred since this worktree has no rubix-agent running; verifying that on a host with `mani run demo` up is the natural follow-up.

## What you need to know

- auth.spec.ts performs logout via `POST /api/v1/auth/logout` against 127.0.0.1:8088 rather than through the UI, because `src/components/layout/nav-user.tsx`'s "Sign out" item is not yet wired to `useAuth().logout()`. Wiring it would let the spec switch to a pure UI path.
- extensions.spec.ts assumes at least one installed extension fixture so the table is non-empty; if the bootstrap seed leaves the table empty the "first row present" assertion will fail — that's a backend fixture issue, not a frontend regression. The spec falls back to Restart if the first row's Start button is already disabled.
- users.spec.ts uses a timestamped email per run to avoid colliding with prior creates against the same DB, and accepts either hard-delete or status="Disabled" after undo to stay robust against undo semantics.
- Selectors lean on the visible CardTitle "Sign in to Rubix" and route headings ("Installed extensions", "Users") plus the `#login-email` / `#login-password` / `#user-email` ids already in the routes.

## Open questions

- Should the NavUser "Sign out" item be wired to `useAuth().logout()` in a follow-up so auth.spec.ts can exercise the full UI path instead of POSTing to the endpoint directly?
- Does the bootstrap seed include any extensions? If not, an `extensions.spec.ts`-friendly fixture should be added to the demo task.
