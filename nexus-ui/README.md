# Nexus · IoT Control — UI Mockup

A premium, dark-mode IoT dashboard **builder** mockup. Drag/drop widgets, CRUD
dashboard pages from the sidebar, all on fake (but deterministic, live-feeling)
telemetry. No backend required.

## Stack
- **Vite + React 18 + TypeScript**
- **Refine** (`@refinedev/core`) as the app shell + CRUD layer
- **shadcn/ui** primitives (hand-built) + **Tailwind CSS** (OLED dark theme)
- **react-grid-layout** — drag/resize widget canvas
- **Recharts** — line/area charts + sparklines; custom SVG radial gauges
- **sonner** — toasts

## Run
```sh
npm install
npm run dev      # http://localhost:5273
npm run build    # production bundle
```

## What works
- **Dashboard CRUD** — create / edit / star / delete pages from the sidebar
  (wired through Refine's `useCreate` / `useUpdate` / `useDelete`). Persists to
  `localStorage`, seeded with 4 example dashboards on first load.
- **Drag & drop builder** — click **Edit**, drag the handle to move, pull the
  corner to resize, **Add widget**, duplicate or remove. Layout auto-saves.
- **6 widget types** — line, area, gauge (threshold-aware), stat/KPI w/ spark,
  status list, device table.

## Where things live
- `src/providers/` — Refine data provider + localStorage store (`store.ts`)
- `src/data/` — types, seed dashboards, deterministic fake-telemetry generators
- `src/components/widgets/` — the widget renderers
- `src/components/DashboardGrid.tsx` — react-grid-layout canvas
- `src/components/layout/Sidebar.tsx` — dashboard list + CRUD
- `src/pages/DashboardPage.tsx` — view/edit toolbar + canvas

> Mockup only: data is fake and client-side. Swap `src/providers/dataProvider.ts`
> for a REST/GraphQL provider to wire a real backend.
