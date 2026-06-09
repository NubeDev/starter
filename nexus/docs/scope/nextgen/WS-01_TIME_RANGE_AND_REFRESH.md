# WS-01 — Global Time Range & Auto-Refresh

> **Status:** Not started · **Wave:** 2 (after WS-03 macro engine) · **Owner:** _unassigned_
> **Depends on:** C2 macro engine (WS-03), C1 JSON model, C3 URL-state scheme (all Wave 0)
> **Migration:** none (state is dashboard-model + URL) · **Read first:** GAP_ANALYSIS §2.1, ROADMAP §0 + §6
> **Verified:** `82a6a19a` on 2026-06-09 — re-grep this WS's file:line claims before building (ROADMAP §0).

## Goal
A Grafana-class **global time-range picker + refresh control** at the dashboard level, whose
`{from, to, interval}` flows into **every panel query** via the macro engine, with shareable URL
state, auto-refresh, and zoom-by-drag on time-series panels. This is the **#1 user-visible gap** —
it converts a static query grid into a live, interactive dashboard.

## Current state (evidence)
- **Nothing exists.** `grep -rniE "timerange|refresh|now-|\$__time"` over `ui/src` returns empty.
- UI store holds only `editMode` + `selectedWidgetId` — `ui/src/store/ui.ts`.
- Panel query is `{datasourceId, sql}` with no time bounds — `ui/src/data/types.ts:34-39`,
  `ui/src/features/widgets/useWidgetQuery.ts`.
- `POST /query` accepts raw `sql` only — `nexus-spi/src/dto/query/run.rs`.

## Scope (this workstream)
1. **Time-range model + store** (`ui/src/store/time.ts`, new): `{from, to}` where each is an
   absolute ISO string **or** a relative token (`now`, `now-6h`, `now/d`). A resolver turns the
   relative range into concrete `{fromTs, toTs}` at query time.
2. **TimeRangePicker UI** (`ui/src/features/time/**`, new): quick ranges (Last 5m/15m/1h/6h/24h/7d/30d,
   Today, Yesterday), absolute from/to with a calendar, relative input, and the **refresh-interval**
   dropdown (off/5s/10s/30s/1m/5m/15m + manual refresh button). Mount in `DashboardToolbar.tsx`.
3. **Auto-refresh loop**: when interval set, invalidate the dashboard's panel queries on a timer
   (TanStack `refetchInterval` or a tick that bumps a `now()` floor in the query key). Pause when
   the tab is hidden.
4. **Wire `{from,to,interval}` into queries**: extend `PanelQuery`/`QueryRequest` so the server
   receives the resolved time range, and call the **WS-03 macro engine** so `$__timeFilter(col)` /
   `$__timeGroup(col, '$__interval')` in panel SQL get substituted. Panels without a time macro are
   unaffected (back-compat).
5. **URL state (C3)**: reflect `?from=&to=&refresh=` in the URL; restore on load; shareable.
6. **Zoom-by-drag**: dragging a range on a line/area panel sets the global time range (and a "zoom
   out / back" affordance). ECharts `dataZoom`/brush → store update.
7. **Per-panel time override** (stretch): a panel may opt out of / shift the global range
   (`timeShift`, relative override) — Grafana parity, low priority.

## Design notes
- **`$__interval` auto-calculation**: derive a sensible bucket from `(to-from)/targetDataPoints`
  (panel width in px is fine) so `$__timeGroup` produces ~N points, like Grafana. Compute
  server-side in the macro ctx or client-side and pass in — decide with WS-03.
- **Resolution is server-authoritative for `now`**: pass relative tokens *and* the client's
  resolved instant, but let the server resolve `now` to avoid clock skew between fan-out panels.
  Freeze one `now` per dashboard refresh so all panels share an instant.
- **Cache key (C3)**: include resolved `{fromTs,toTs}` (snapped to the refresh tick, not raw ms,
  or every tick busts cache) + interval in the TanStack key and in the WS-09 result-cache key.
- **DTO**: add `time_range: Option<{from,to}>` and `max_data_points: Option<u32>` to
  `QueryRequest`; the macro ctx is built from these. Keep raw-SQL-without-macros working.

## Acceptance criteria
- [ ] Picker in the toolbar; selecting "Last 6h" re-runs all panels whose SQL uses a time macro.
- [ ] Auto-refresh at the chosen interval; pauses on hidden tab; manual refresh works.
- [ ] `from/to/refresh` survive a page reload and are shareable via URL.
- [ ] A panel with `$__timeFilter(ts)` returns only in-range rows; one without is unchanged.
- [ ] Drag-zoom on a line panel updates the global range; "zoom out" restores.
- [ ] All panels in one refresh share a single frozen `now`.
- [ ] Tests: range resolver (relative→absolute, `now/d` rounding), cache-key snapping, macro
  substitution integration, picker component logic.

## Out of scope (hand off)
- The macro engine itself → **WS-03** (this WS *consumes* it).
- Variable interpolation → **WS-02**.
- Result caching → **WS-09** (just make the key correct here).
- Annotations / event overlays on the time axis → **WS-05**.
