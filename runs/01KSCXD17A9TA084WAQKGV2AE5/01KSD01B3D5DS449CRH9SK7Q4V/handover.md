## Done

- extended rubix/frontend/src/components/top-header.tsx with user email + role badge (useAuth), tenant indicator (useTenants — display-only for 0/1, disabled dropdown with "switching not yet wired" tooltip for 2+), logout dropdown menu item calling useAuth().logout(), and a theme toggle button cycling light/dark/system via the rubix-side useTheme().setMode
- added i18n keys (header.role.{admin,writer,reader}, header.tenant.{label,none,switchingNotWired}, header.logout, header.userMenu, header.themeToggle) to en.json + es.json
- both header and sidebar layout modes render the new trailing controls via a shared HeaderTrailing
- `pnpm --filter @nube/rubix-frontend typecheck` green
- committed as f488d30

## Next

- Stage 15 (next phase D step) — pick up from SCOPE.md / job plan

## What you need to know

- The package exports `useTenants` (not `useTenantList`); imported from `@nube/starter-ui-authz`. Functionally equivalent.
- SCOPE OQ-7 resolved: `@nube/starter-ui-kit` does not export a `useTheme`; rubix's existing `useTheme` in `src/stores/theme-store.ts` (built on `@nube/starter-ui-core`'s `useLayoutPreferences`) is the canonical source — no hand-rolled CSS-variable toggle needed.
- Test selectors added: `data-testid="user-email"`, `user-role-badge`, `logout-menu-item`, `tenant-indicator-name` for future e2e in later D stages.
- ActionDock is preserved at the trailing edge (it already carries its own ModeSwitcher/PaletteMenu/locale/config) — the new ThemeToggle in the header is the explicit stage-mandated control; the duplication is intentional per the stage spec.

## Open questions

- (none)
