## Done

- Wired rubix/frontend/src/main.tsx with QueryClientProvider → RubixClientProvider → DirectionProvider → ThemeProvider → I18nProvider → AuthProvider(unauthenticatedSlot=<LoginRoute/>) → RouterProvider
- QueryClient configured with starter defaults (staleTime 30s, gcTime 5min, retry skips 401/403)
- ReactQueryDevtools mounted behind `import.meta.env.DEV` (SCOPE OQ-3)
- Layout-level auth guard via AuthProvider unauthenticatedSlot, no per-route checks (SCOPE OQ-4)
- Added src/routes/login.tsx — file-route at /login + named LoginRoute component using starter-ui-kit Card/Input/Label/Button, calls useAuth().login, redirects to same-origin ?returnTo or /
- Added deps: @nube/starter-client-react, @nube/rubix-client-react, @tanstack/react-query-devtools
- pnpm --filter @nube/rubix-frontend typecheck green
- Commit: eea34ca

## Next

- Stage 13 (per workflow): wire /extensions SSE-driven panel and /admin/users write+undo

## What you need to know

- rubix/frontend has no `test` script (only playwright e2e); typecheck is the only gate run
- `pnpm vite build` regenerates routeTree.gen.ts via the router plugin — needed any time a file under src/routes/ is added before `tsc` will accept it
- pnpm-lock.yaml updated; a peer-dep warning surfaced: `@tanstack/react-query-devtools 5.100.14` wants `@tanstack/react-query ^5.100.14` but installed is 5.100.11 — non-blocking, works at runtime, can bump query in a later stage if desired
- AuthProvider's unauthenticatedSlot path: when /me 401s, LoginRoute renders in place of <RouterProvider>; the address bar keeps its current path, login reads ?returnTo for redirect
- /login also exists as a routable URL so external links land on the same form

## Open questions

- (none)
