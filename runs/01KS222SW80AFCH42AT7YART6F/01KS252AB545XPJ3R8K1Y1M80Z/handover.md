## Done

- Added `Sidebar` primitive at `packages/starter-ui-kit/src/components/ui/sidebar.tsx` (400 lines): `Sidebar`, `SidebarProvider`, `SidebarHeader`, `SidebarContent`, `SidebarGroup`, `SidebarGroupLabel`, `SidebarGroupContent`, `SidebarItem`, `SidebarTree` + `useSidebar` hook. Composes radix `Collapsible` + `ScrollArea`. Cookie (`sidebar_state`) persists open/collapsed; `(max-width: 1024px)` matchMedia forces icon-rail.
- Exported primitive from `packages/starter-ui-kit/src/index.ts`.
- Rewrote `examples/flow-agent/frontend/src/layout/Sidebar.tsx` as a nested tree: top-level `Flows` and `Agents` collapsible groups fed by react-query (`api.flows.list` / `api.agents.list`); `useSse<SidebarEvent>("/api/events", …)` invalidates the matching cache on `flow-*` / `agent-*` events for live updates without polling. `Settings` rendered as a `SidebarItem asChild` wrapping `NavLink`.
- Wrapped `Shell` in `<SidebarProvider>`.
- Expanded `useUiStore` to track unified `expandedGroups: string[]` + `toggleGroup` (replacing single `expandedFlowGroups`), persisted via `partialize` under `fa-ui` localStorage key alongside `sidebarOpen` + `activeSection`. Active section auto-tracks the current route.
- `pnpm --filter @nube/starter-ui-kit typecheck` and `pnpm --filter flow-agent-frontend typecheck` both green.
- Committed as `f1921d7` on branch `codeless/flow-agent-example`.

## Next

- Stage 7 / Phase F6 polish (Apple-modern shadcn aesthetic pass) per the example SCOPE checklist.

## What you need to know

- No `SIDEBAR.md` file actually exists in the repo — the spec referenced it. The cookie name (`sidebar_state`), the single localStorage key (`"fa-ui"`), the 1024 px breakpoint, and the partialize fields were interpreted from the stage spec directly. If the next session creates `SIDEBAR.md`, keep these constants stable.
- The `SidebarItem` primitive supports `asChild` to merge classes onto a child element (used for the Settings `NavLink`). Tree leaves use a `render` callback for the same wrapping pattern — both avoid nesting interactive elements.
- SSE strategy is "invalidate-on-event" rather than patching the query cache, which is simpler and atomic; the cost is one refetch per event but the list endpoints are cheap.
- Sidebar primitive is exactly 400 lines (the stated cap).

## Open questions

- Should the `SidebarProvider` also surface a hamburger toggle in `Topbar` (useSidebar().toggle)? Not in this stage; left for the F6 polish pass.
- Per-flow children (e.g. recent runs as sub-items) — `SidebarTree` already supports nesting but no second-level data source is wired yet.
