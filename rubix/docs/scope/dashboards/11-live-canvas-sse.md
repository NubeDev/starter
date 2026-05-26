# 11 — Live canvas refresh over SSE

> **Tier:** scope (plan). Lifetime: weeks. Per
> [HOW-TO-CODE.md §0a](../../../../HOW-TO-CODE.md), **no source
> code may reference this file.** Promote landed sections into
> `docs/design/sdui/builder/README.md` (canvas) and
> `docs/design/sdui/renderer/README.md` (read route) once
> shipped.

## Goal

When **anyone** — operator B, the AI assistant
(`com.rubix.dashboard-assistant`), or a future automation —
writes a new revision of a dashboard page, every client
currently *looking at that page* updates within ~1 s without a
manual refresh. Covers:

- The **Puck visual editor canvas** (scope
  [10](./10-puck-builder.md)) while the operator is editing.
- The **read-only dashboard route** while a viewer has it open.

This closes Q6 in scope 10 ("operator opens the editor while
the AI is mid-stream updating the same page") and the
equivalent unstated gap on the read route, by reusing the SSE
channel that already broadcasts to the sidebar.

The sidebar's live behaviour (scope
[09](./09-live-sidebar-sse.md)) ships today and is **not in
scope here** — this doc extends the same wire to per-page body
updates.

## Why a separate scope from Puck (10)

Scope 10 is the editor; this scope is the *liveness*. Two
distinct delivery vehicles:

- 10 can ship without 11 — the editor works, it just doesn't
  notice concurrent writes until Save returns 409.
- 11 can ship without 10 — the read route gains live body
  refresh today, AI-authored changes appear without a reload,
  and the editor inherits the same channel when it lands.

Keeping them split also keeps the SSE wire change reviewable on
its own — the editor PR is already large.

## User stories

> *Operator opens `/dashboards/site-a/edit` and starts dragging
> a chart into a row. The AI assistant runs in a different tab
> ("add a litres KPI to site-a"). Within ~1 s the editor shows
> a non-blocking banner: "AI updated this page just now — your
> view is one revision behind. Reload / Keep my edits."*

> *Operator A leaves `/dashboards/site-a` open on a wall TV in
> the ops room. Operator B renames a KPI from the Puck editor
> in the back office. Within ~1 s the wall TV repaints with
> the new title — no full page reload, no fetch the operator
> had to trigger.*

## Non-goals

- **No new write path.** All revisions still go through
  `rubix.dashboard.update`. This scope only carries *news* of
  the write to readers.
- **No co-editing / CRDT.** Two operators editing the same
  page simultaneously is still "last save wins" via
  `expected_revision_id`. The banner gives them an
  opportunity to reload before saving.
- **No fat frames.** SSE deltas continue to carry metadata
  (`page_id`, `revision_id`, `title`), not `body_json`.
  Clients re-fetch on demand. Keeps the channel cheap, the
  wire shape stable, and the sidebar's existing consumers
  unchanged.
- **No per-page channel.** We do not mint
  `/api/v1/dashboards/$pageId/events`. The existing
  tenant-scoped channel is the carrier; clients filter
  client-side. Justified below.
- **No in-flight AI feedback** in v1 (see B7). The wire only
  fires *after* the AI's write commits. Pre-commit visibility
  ("AI is editing now…", "AI tried and failed") is filed for
  the agent-event-projection follow-up.
- **No live partial updates** (patching a single KPI value
  without re-resolving the page). KPI / chart point ticks
  ride the existing `SubscriptionPlan` channel
  ([`crates/starter-ui-bindings/src/subscription.rs`](../../../../crates/starter-ui-bindings/src/subscription.rs)) —
  out of scope here. This scope is about **structural** page
  changes (a new widget added, a title changed, a chart
  reconfigured), not data ticks.

## What we have today

| Piece | Status | Source |
|---|---|---|
| `GET /api/v1/dashboards/events` SSE route — emits `snapshot` + `created` / `updated` / `deleted` frames per tenant | ✅ shipping | [`crates/rubix-agent/src/routes/dashboard_events.rs`](../../../crates/rubix-agent/src/routes/dashboard_events.rs) |
| Frame schema (`page_id`, `title?`, `revision_id?`, `tenant_id`) | ✅ shipping | same |
| Changelog-driven trigger (every `rubix.dashboard.{create,update,delete}` tool.invoke → SSE row) | ✅ shipping | same |
| React hook `useDashboardSidebar` consuming the stream | ✅ shipping | [`packages/rubix-client-react/src/hooks/use-dashboard-sidebar.ts`](../../../packages/rubix-client-react/src/hooks/use-dashboard-sidebar.ts) |
| `rubix.dashboard.get` returning a single revision | ✅ shipping | [`crates/rubix-tools/src/dashboard/get.rs`](../../../crates/rubix-tools/src/dashboard/get.rs) |

We have everything we need to extend liveness to the canvas
and read route without a new endpoint.

**Sidebar behaviour is unchanged.** Scope
[09](./09-live-sidebar-sse.md) already lights up the sidebar
on AI writes (the changelog row drives both surfaces). This
scope adds the per-page canvas/read consumers of the same
stream — it does not modify the sidebar's reducer or wire.

## What we have to build

### B1. New React hook: `usePageLiveness(pageRef)`

Lives in `@nube/starter-ui-sdui-react` alongside the existing
resolver hooks. Subscribes to `/api/v1/dashboards/events`
through the same `useEventStream` wrapper the sidebar uses,
then **filters client-side** to the frame's `page_id` matching
`pageRef`.

API:

```ts
export function usePageLiveness(pageRef: string): {
  latestRevisionId: string | undefined;
  /** True while the SSE channel is connected. */
  connected: boolean;
  /** Bumps when the server announces a new revision for this page. */
  changeToken: number;
};
```

`changeToken` is the integration point — consumers `useEffect`
on it to decide what to do. We do not re-fetch the body inside
the hook because what to do on a change is consumer-specific
(see B2 and B3 below).

**Why filter client-side, not a per-page channel.** The
tenant-scoped stream already exists; minting
`/api/v1/dashboards/$pageId/events` would mean a second route,
a second `LISTEN` subscription per connection, and a new
auth/CSRF/keepalive code path. A tenant typically has tens to
low hundreds of pages, and the change rate is low (human
typing speed at most). Client-side filtering of ~1 frame per
update against the page id is cheap. Revisit if a tenant
demonstrates 10k+ pages or constant write volume.

### B2. Read route — auto-refresh body

`<SduiPage pageRef>` (in `@nube/starter-ui-sdui-react`) gains an
internal subscription to `usePageLiveness(pageRef)`. On
`changeToken` bump where `latestRevisionId !==
currentlyRenderedRevisionId`, re-call `/api/v1/ui/resolve` for
the same `pageRef` and swap the tree.

Behaviour:

- **Default (read route): auto-refresh, no banner.** Viewers
  are not editing; silent update is the right UX. The
  rendered tree just changes.
- **Pending interactive state preservation** — if the page
  contains forms or `$page` state the user has typed into,
  preserve those across the swap (they live in client state,
  not in the body). Existing `<SduiPage>` already keeps
  `$page` across re-resolves; this re-uses that path.
- **Visible diff hint** — on swap, briefly flash a 1 s outline
  around any DOM node whose `id` is new or whose props
  changed. Optional UX nicety; can ship without it.

### B3. Editor canvas — non-blocking banner

`<PuckBuilder>` (scope 10) subscribes to
`usePageLiveness(pageRef)`. On a `changeToken` bump where the
incoming `revision_id` does not match the editor's loaded
`expected_revision_id`:

- Show a **persistent non-blocking banner** at the top of the
  canvas: *"AI/operator updated this page just now. You are
  editing revision `abc1234`; the live one is `def5678`.
  **Reload** loses your unsaved edits. **Keep editing** will
  409 on Save until you reload."*
- Two buttons in the banner — Reload, Keep editing.
- **Do not auto-reload the canvas.** The editor's in-memory
  tree is the operator's work-in-progress; clobbering it on a
  background event is the worst UX.
- Existing 409-on-save handling (scope 10 §B4) stays as the
  ultimate safety net for operators who dismissed the banner.

### B4. Frame enrichment — `actor_kind`

Today's SSE frame carries `tenant_id`, `page_id`,
`revision_id?`, `title?`. Add two fields:

- `actor_kind: "operator" | "ai" | "system"` — the *immediate*
  caller.
- `acting_for_principal: string | null` — the operator who
  asked the AI to act, when applicable.

Derivation from the changelog row's principal:

| Principal pattern | `actor_kind` | `acting_for_principal` |
|---|---|---|
| `user.<uuid>` directly invokes the verb (Puck editor save) | `"operator"` | `null` |
| `flow.com.rubix.dashboard-assistant` with the flow run's input carrying `user.<uuid>` (operator chat → AI) | `"ai"` | `user.<uuid>` |
| `flow.<id>` with no carrying operator (a scheduled flow) | `"ai"` | `null` |
| `system.<name>` (bootstrap, seed) | `"system"` | `null` |

The carrying operator is read from the flow run's input
context (`flow_runs.input_json.operator_principal`) at the time
the changelog row is recorded. Banner copy in B3 then renders:

- `actor_kind="operator"` → "Operator updated this page just now"
- `actor_kind="ai"`, `acting_for_principal=null` → "AI updated
  this page just now"
- `actor_kind="ai"`, `acting_for_principal="user.X"` → "AI (on
  behalf of operator X) updated this page just now"

Wire fanout:

- The SSE frame schema in
  [`dashboard_events.rs`](../../../crates/rubix-agent/src/routes/dashboard_events.rs)
  gains the field on `created` / `updated` / `deleted` (not on
  `snapshot`, which is principal-agnostic).
- `DashboardSidebarFrame` in
  [`use-dashboard-sidebar.ts`](../../../packages/rubix-client-react/src/hooks/use-dashboard-sidebar.ts)
  mirrors the field; the existing reducer ignores it.

Used by B3's banner copy ("AI updated this page" vs "operator
B updated this page") and by future telemetry. Backwards
compatible — older clients ignore the new field; the wire
contract on the existing fields is unchanged.

### B7. In-flight AI activity — **deferred**

Everything in B1–B4 fires **after** the AI's
`rubix.dashboard.update` call commits a changelog row. An AI
that thinks for 30 s and then writes shows the operator
nothing for 30 s, then a sudden banner. That gap is
deliberate, not accidental:

- The current flow runtime does not project inner Text /
  ToolUse / ToolResult agent events onto the flow event bus.
  See follow-up note
  [`docs/sessions/data-flow/2026-05-26-data-flow-07-agent-event-projection.md`](../../sessions/data-flow/2026-05-26-data-flow-07-agent-event-projection.md).
- A failing AI run (e.g. the verb 400s before commit) produces
  **no** changelog row and therefore **no** SSE frame.
  Operator sees nothing, AI silently gave up.

What this scope does *not* ship:

- "AI is editing this page now…" pulse on the editor canvas /
  read route while a `com.rubix.dashboard-assistant` flow run
  targets this `page_id`.
- "AI tried and failed" surfaced as a one-shot toast / inline
  error.

What we will need when the agent-event projection lands:

- Subscribe `usePageLiveness` to flow events keyed by the
  flow's `input.page_id` argument, in addition to the
  post-commit dashboard frames it consumes today.
- A new derived event kind, `ai_in_flight` (start) +
  `ai_settled` (success or fail) per `(page_id, run_id)`.
- Failure paths emit `ai_settled` with `error: { code, message }`
  so the editor / read route can render a transient toast.

Tracked as **scope 11.1** (sibling doc to land once the
agent-event projection follow-up commits). Until then, the v1
behaviour is "you find out when the AI commits, or never."
The non-goal in the next section codifies that explicitly.

### B5. Reconnect / catch-up

Inherited from `useEventStream`: on a transient disconnect, the
client auto-reconnects and the server resends the `snapshot`
frame. For per-page liveness this is sufficient — the snapshot
includes every page's current `revision_id`, so a reconnecting
client whose locally-held revision is stale immediately sees
the gap.

No new replay channel, no per-page sequence number. Document
this so the implementer does not invent one.

### B6. Why clients don't re-validate — wire is already authoritative

Component validity is enforced at **write time** by
`rubix.dashboard.update` — the verb deserialises `body_json`
into the Rust `ComponentTree` and rejects invalid variants /
required-field violations before the changelog row is
committed. Therefore:

- Every SSE delta is for a body that has already passed
  server-side schema validation.
- Clients **do not re-validate** before rendering. The read
  route trusts the body; the editor trusts the loaded
  revision.
- The Puck palette is the *authored*-variant subset of
  `Component` (the table in
  [`docs/design/sdui/components/`](../../design/sdui/components/README.md))
  — `Forbidden` / `Dangling` / `Unknown` are not draggable, so
  the operator cannot author an invalid tree in the first
  place.

If a malicious or buggy client somehow lands an invalid body
through `update`, the server's deserialise-on-write check
catches it before it reaches the changelog and therefore
before it reaches SSE. There is no "invalid body delivered
live" failure mode by construction.

## Wire-shape changes (additive)

```diff
  // dashboard_events.rs `DashboardEvent` enum
  Updated {
      page_id: String,
      title: Option<String>,
      revision_id: Option<String>,
      tenant_id: String,
+     actor_kind: ActorKind, // "operator" | "ai" | "system"
+     acting_for_principal: Option<String>, // user.<uuid> when AI is operator-driven
  }
  // (and Created / Deleted, identical addition)
```

```diff
  // use-dashboard-sidebar.ts `DashboardSidebarFrame`
  | {
      kind: "updated";
      page_id: string;
      title?: string;
      revision_id?: string;
      tenant_id: string;
+     actor_kind?: "operator" | "ai" | "system";
+     acting_for_principal?: string | null;
    }
```

Snapshot frame is unchanged. `actor_kind` is optional on the
TS side so older servers continue to type-check against the
hook.

## Dependency order

```
B4 (frame enrichment) ──►  B1 (usePageLiveness hook)  ──►  B2 (read route)
                                                       └──►  B3 (editor banner — depends on scope 10 shipping)

B5 (reconnect) and B6 (validation guarantee) — documentation-only;
ship alongside B1.
```

B4 + B1 are one PR (server + first consumer). B2 is a small
follow-up. B3 ships as part of scope 10's editor PR or
immediately after.

## Open questions

| # | Question | Default if no one answers |
|---|---|---|
| Q1 | Do we throttle the per-page re-resolve in B2? (Rapid AI iteration could trigger many re-fetches.) | Debounce 250 ms on the client side. Re-fetch only the last `revision_id` seen at debounce expiry. |
| Q2 | Does the read route show *who* updated the page (operator name, "AI assistant")? | **No** in v1. `actor_kind` is exposed but not shown as a toast or attribution chip. Re-evaluate when there is a UX ask. |
| Q3 | What happens if the page is deleted while the editor / read route has it open? | Editor shows a blocking modal "This page was deleted — your edits are lost, paste them elsewhere"; read route shows a 404 inline. Both close the SSE filter for that page id. Applies equally to operator-driven and AI-driven deletes (`actor_kind` decides only the copy: *"AI deleted this page"* vs *"Operator deleted this page"*). |
| Q4 | Mobile / Tauri / Flutter clients — do they get the same hook? | Out of scope here. The server-side wire is the same; client hooks land per-platform when those clients pick this up. |
| Q5 | Multi-step AI runs fire N independent SSE frames (5 sequential `update` calls = 5 banners / 5 re-fetches). Coalesce? | **v1.5 hardening.** Add a `run_id` / `trace_id` field on the SSE frame so clients can debounce by run — banner copy then says "AI made 5 changes" with one Reload. Out of scope for v1 because it touches both the changelog row schema and the flow runtime; flagged so the v1 implementer leaves headroom (the SSE frame is additive). |

## How this maps to SCOPE.md

- **Goal 1** — closes the "operator edits in real time" gap in
  the dashboard surface.
- **R7** — no new tool verbs; the AI sees its own writes
  reflected live because it goes through the same
  `rubix.dashboard.update` path that triggers the SSE.

## A note on cited line numbers

Same disclaimer as the parent README — line numbers in scope
files are anchors for the implementer to find the right
symbol, not stable references. Re-grep before quoting.
