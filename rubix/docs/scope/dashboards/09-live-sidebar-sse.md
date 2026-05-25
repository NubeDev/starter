# Live dashboard sidebar over SSE

> **Tier:** scope (plan). Lifetime: weeks. Per
> [HOW-TO-CODE.md §0a](../../../../HOW-TO-CODE.md), **no source
> code may reference this file.** Promote landed sections into
> `docs/design/sdui/` once shipped.

## Goal

When a user — operator **or AI assistant** — creates, renames, or
deletes a dashboard page, the **rubix sidebar updates in real
time** without a refresh. The key requirement is **SSE**: an
already-open browser tab is the source of truth for "what just
happened", so we *see* the AI's work the moment it lands instead
of after the next route change.

> Out of scope: live updates inside a dashboard's *body* (KPI
> values, chart points) — that path already exists via
> [`SubscriptionPlan`](../../../../crates/starter-ui-bindings/src/subscription.rs).
> This scope is exclusively about the sidebar **page list**.

## User story

> *Operator opens `/`. They type
> "make me a dashboard that shows my five biggest disks" into the
> chat. The AI calls `rubix.dashboard.create`. Within ~1 s the
> left sidebar grows a new entry "Biggest disks" under
> **Dashboards**. They click it and the SDUI route resolves the
> page they just had the AI author.*

No reload. No polling. No "refresh sidebar" button. The chat tab
and the sidebar are the same surface.

## What we already have (no design needed)

| Substrate | Status | Source |
|---|---|---|
| `DashboardStore` trait with `list`, `create_revision`, etc. | ✅ shipping | [`rubix/crates/rubix-spi/src/dashboard.rs`](../../../../rubix/crates/rubix-spi/src/dashboard.rs) |
| `PgDashboardStore` writing `dashboards_definitions` (and emitting `starter_changes` rows for every write) | ✅ shipping | [`rubix/crates/rubix-store-postgres/src/dashboards/mod.rs`](../../../../rubix/crates/rubix-store-postgres/src/dashboards/mod.rs) |
| `ChangeTail` trait — `subscribe() -> mpsc::Receiver<Change>` over the change log | ✅ shipping | [`crates/starter-changelog/src/tail.rs`](../../../../crates/starter-changelog/src/tail.rs) |
| Postgres `LISTEN/NOTIFY` `ChangeTail` impl | ✅ shipping | [`crates/starter-changelog-postgres/`](../../../../crates/starter-changelog-postgres/) |
| Axum SSE helpers — `from_stream`, `keep_alive` | ✅ shipping | [`crates/starter-server/src/sse/`](../../../../crates/starter-server/src/sse/) |
| `principal` task-local + `with_principal` HTTP layer | ✅ shipping | [`crates/starter-server/src/auth/`](../../../../crates/starter-server/src/auth/) |
| Static `NAV_GROUPS` in the rubix frontend | ✅ shipping | [`rubix/frontend/src/lib/nav.ts`](../../../../rubix/frontend/src/lib/nav.ts) |
| `useToolCall` / MCP client hooks | ✅ shipping | [`rubix/packages/rubix-client-react/src/hooks/mcp.ts`](../../../../rubix/packages/rubix-client-react/src/hooks/mcp.ts) |

What this means: every dashboard write **already** lands a row in
`starter_changes` (resource_kind = `dashboard`, op = `created` /
`updated` / `deleted`), and the changelog **already** has a live
tail trait + a PG `LISTEN/NOTIFY` implementation. We do not need
any new write path. We need a **read** path that filters that
tail by tenant and streams it as SSE.

## What we have to build

Four small components, in a strict tenant-scoped read pipeline.

### 1. `GET /api/v1/dashboards/events` — SSE endpoint
**Crate:** `rubix-agent` (new router module
`rubix/crates/rubix-agent/src/sdui/dashboard_events.rs`)

- Wraps the shared `Arc<dyn ChangeTail>` (already built in
  `boot`); on connect, calls `subscribe()` and gets an
  `mpsc::Receiver<Change>`.
- Filters server-side:
  - `change.resource_kind == "dashboard"`
  - `change.tenant_id == current_principal().tenant_id`
    (admin sentinel `"*"` sees all tenants)
- Maps each surviving `Change` to a typed event:
  ```
  event: dashboard.created   data: { "page_id": "...", "title": "...", "revision_id": "..." }
  event: dashboard.updated   data: { "page_id": "...", "title": "...", "revision_id": "..." }
  event: dashboard.deleted   data: { "page_id": "..." }
  ```
- Returns `Sse::from_stream(...).keep_alive(starter_server::sse::keep_alive())`
  so proxies don't kill idle tabs.
- Guarded by the same `with_principal` layer used by
  `/api/v1/ui/resolve`; **no anonymous subscribers**.

**Initial snapshot:** the stream's first frame is a synthetic
`event: dashboard.snapshot` carrying the current
`DashboardStore::list(tenant_id)` result. This removes the
"connect → empty sidebar → wait for first edit" hole and means
the client never needs a separate REST list call.

### 2. `useDashboardSidebar()` — frontend hook
**Package:** `rubix-client-react`
(`rubix/packages/rubix-client-react/src/hooks/dashboards.ts`)

- Opens an `EventSource('/api/v1/dashboards/events')` with
  `withCredentials: true`.
- Reducer over the three event types, keyed on `page_id`.
- Returns `{ items, status: 'connecting' | 'live' | 'reconnecting' | 'error' }`.
- Auto-reconnect on `error`, exponential backoff capped at 30 s,
  re-syncs from the snapshot frame on every reconnect (so we
  cannot drift even if `LISTEN/NOTIFY` drops a message during a
  reconnect window).
- One subscription per app (React context provider); the sidebar
  and any other consumer share the same `EventSource`.

### 3. Dynamic sidebar group
**File:** [`rubix/frontend/src/lib/nav.ts`](../../../../rubix/frontend/src/lib/nav.ts)
+ the sidebar component that consumes it.

- Today `NAV_GROUPS` is a static `const`. Split:
  - `STATIC_NAV_GROUPS` — the existing list.
  - `useNavGroups()` — composes static groups with a new
    **Dashboards** group built from `useDashboardSidebar().items`,
    each item linking to
    `/dashboard/${encodeURIComponent(page_id)}`.
- Empty state: when there are zero pages, render an empty group
  with a "Create your first dashboard" CTA pointing at `/chat`.
- Status badge: when the hook reports `reconnecting`, the group
  header shows a small dot; when `live`, no badge. Never hide the
  group on a transient disconnect.

### 4. Route for an individual dashboard
**File:** new `rubix/frontend/src/routes/dashboard.$pageId.tsx`

- TanStack Router route, reads `$pageId`, calls
  `/api/v1/ui/resolve` with `page_ref` = `$pageId`. (Pure
  SDUI — no special-cased renderer.)
- Same SSE hook keeps the surrounding sidebar fresh while the
  user is viewing.

## Non-goals

- **No** dashboard *body* live updates here (already handled by
  bindings + `SubscriptionPlan`).
- **No** WebSocket. We pick SSE because: one-way is enough, it
  rides standard HTTP/2 multiplexing with the existing
  `with_principal` middleware, and `EventSource` auto-reconnects
  for free.
- **No** new write path. The trigger is the existing
  `starter_changes` row that `PgDashboardStore` already emits.
- **No** cross-tenant fan-out. Filtering is per-subscriber so a
  noisy neighbour cannot inflate another tenant's traffic.

## Edge cases the design must pass

| Case | Required behaviour |
|---|---|
| Tab open through `pkill rubix-agent && restart` | `EventSource` reconnects; **snapshot frame replays full list**, no stale items. |
| AI creates 50 dashboards in 2 s (burst) | Server coalesces nothing — every row is forwarded. Client reducer is idempotent on `page_id`, so duplicate `created` from snapshot + tail collapses. |
| Principal changes tenant mid-session (token swap) | Endpoint closes the stream on `with_principal` boundary change; client reopens and gets the **new** tenant's snapshot. No cross-tenant leak ever. |
| Pool-less laptop run (`InMemoryDashboardStore` + no PG `LISTEN`) | Endpoint must still serve the snapshot; tail can be a no-op `tokio::sync::broadcast` so creates from the same process still surface. |
| `LISTEN/NOTIFY` drops a packet | Reconnect → snapshot resync is the recovery path. We do not promise per-message durability. |
| Browser puts the tab to sleep | `keep_alive()` pings; on resume `EventSource` reconnects naturally. |

## Acceptance test (E2E)

In the existing `e2e` script next to the Bug 1/2/3 verification:

1. Boot rubix-agent + frontend.
2. Open the dashboard page in a headless browser; assert the
   sidebar shows the seeded "Disk overview" entry (snapshot).
3. From a second client, call
   `POST /api/v1/tools/rubix.dashboard.create` for a new page.
4. **Within 2 s**, assert the browser DOM contains the new
   sidebar entry — without reload.
5. Call `rubix.dashboard.delete`; assert the entry disappears in
   the same window.
6. Kill the agent, restart, wait for `EventSource` `open`, assert
   the entry list still matches the DB.

This is the single test that proves all four pieces wired
together. It is the smoke-test equivalent of `RUBIX_AI_NARRATION=0`:
deterministic, no LLM in the loop.

## Why this scope

Today the chat surface can author a dashboard and the SDUI surface
can render it, but the sidebar still listed pages from a static
TypeScript array — operators had to know the page id and type it
into the URL bar to see what the AI just made. SSE closes that
loop with **one** new server route, **one** new client hook, and
**zero** changes to the dashboard write path. It also unlocks
the next two scopes (live "activity" feed, live extensions list)
for free: they reuse the same `ChangeTail` → SSE pattern with
different `resource_kind` filters.
