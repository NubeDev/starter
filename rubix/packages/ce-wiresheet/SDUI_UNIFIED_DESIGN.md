# SDUI — unified design for live + historical + control surfaces

**Status:** proposal for review. SDUI is still in dev, so the resolve/transport
contract is not yet frozen — this is the moment to widen it before more surfaces
lock in the dashboard-only assumptions.

**The one-line change.** Today SDUI fuses *layout* and *values* into one server
`resolve` call, so the only way to update a value is to re-fetch the whole page.
Split the contract into three independently-updating layers (**Structure / Data /
State**), give the data layer two access patterns (**stream vs query**), and make
the **transport** the single seam that says where subjects come from and how. Then
one IR + one renderer + one Puck authoring surface serve every use case below; each
new surface writes only a transport, a small widget pack, and a surface
registration — no value is ever baked into the tree.

---

## 1. Use cases (confirm these — and tell me what's missing)

The design must serve all of these from the same engine. Axes that actually differ
are called out, because they're what the architecture has to absorb.

| # | Use case | Data plane | Liveness / cadence | Volume | Mutations | Authoring | Scoping |
|---|----------|-----------|--------------------|--------|-----------|-----------|---------|
| **1** | **Energy / utility dashboards** (current) | rubix agent → **Timescale** hypertable + `pg:` dimensions | slow re-query (~15s / SSE); windows of 24h–30d | **large** (history, append-heavy) → **query**, not stream | run a **tool** → toast + refresh | operators, **Puck** + AI builder | multi-tenant, Postgres pages, authz-gated |
| **2** | **Wiresheet component UX** (e.g. scheduler) | **direct browser → CE** REST + binary WS | **live, high-freq** per prop | small (scalars/state) → **stream** | `callAction(uid,…)` / override | static **IR file per component type** | single engine, local, no tenancy |
| **3** | **Zenoh-discovered devices** (incl. CE) | **Zenoh = discovery + auth** (Layer 0), then device-native REST/WS *or* Zenoh pub/sub | **live** + optional bounded history | small live + bounded history | device-native action *or* Zenoh `put`/queryable | static or contributed IR | per-device; **auth brokered by Zenoh** |
| **4?** | **Live SDUI sidebar** (scope `09-live-sidebar-sse.md`) | agent | live-ish (SSE) | small | navigation/state | host-defined | per-tenant |
| **5?** | **Mixed dashboard** (live + historical together) | several transports at once | mixed | mixed | mixed | Puck | per-tenant |

**Candidate use cases to confirm or drop** (flagging so we don't miss them):

- **Mobile / React Native** — the renderer is already split `./headless` (no
  ui-kit) precisely so RN can supply native widgets via the same registry. Is a
  native client in scope? It constrains "no DOM in the IR."
- **Alarms / events feed** — a *push* stream that's neither a scalar nor a windowed
  series (it's an append-only event log). Does it need a third data shape, or is it
  just a query subject with incremental append?
- **AI-built control panels** — the AI builder (`06-ai-builder.md`) today targets
  dashboards; should it also author control surfaces (use case 2/3)?
- **Offline / edge-local rendering** — controller reachable but agent/cloud not.
  Does a surface need to resolve structure without the agent? (Use case 2 already
  does — static IR file.)
- **Write-heavy forms** (commissioning, setpoint batches) — staged edits + a single
  commit, larger than one field. State layer covers it, but confirm it's wanted.

> **Action for review:** confirm 1–3, decide 4–5, and tell me which of the
> candidates are real. Everything below is sized to 1–3 + mixed (5).

---

## 2. Why today's design can't span these

`/ui/resolve` returns a tree with **values already inlined**, and "liveness" =
`subscribe` → `invalidateQueries` → **re-fetch the whole `UiComponentTree`**. So:

- A single value tick re-pulls the **entire page layout** (wasteful even for a
  dashboard; impossible at 20 Hz for a control panel).
- All data is forced through one access pattern. Energy history (large, windowed,
  query-shaped) and a CE prop (tiny, live, stream-shaped) can't both be right.
- Binding is resolved **server-side only** — there's no client-side reactive value
  path, so high-frequency / direct-to-device is structurally excluded.

The data *source* definition (`analytics_template` + params + map) is **orthogonal**
to all this — it says *where* a value comes from, not *how* it's delivered. The
waste lives in the transport/hook layer, which is exactly what we're free to
replace.

---

## 3. The model: three layers, updated independently

| Layer | Changes | Authority | Delivered by |
|-------|---------|-----------|--------------|
| **Structure** | rarely (open, nav, edit/publish, conditional layout) | **server** — authz-trimmed, `show_when`, `repeat` expansion | snapshot on connect + occasional **structure deltas** |
| **Data** | constantly (stream) or per-window (query) | **per-subject** | **stream frames** or **query results** (see §4) |
| **State** | on user input | **client** (`page_state`, already exists) | local, optional commit via action |

The renderer materialises Structure once, then patches Data into the live widgets
and re-renders only what changed. Structure and Data are never re-shipped together.

### Layer 0 — Connection: discovery + auth (Zenoh is here, not in the data layer)

Before any structure resolves, a surface needs a **connection to a device**. This is
a distinct, **pluggable** layer that does three things: **discover → authenticate →
hand back a concrete transport**.

- **Zenoh is the device substrate.** *Every device is a Zenoh node*, so it is
  **discoverable** on the Zenoh network — the client enumerates devices instead of
  hand-registering `ip:port` (this supersedes the `ce_devices` ip/port table in the
  original plan).
- **Zenoh brokers auth.** Authenticating **through Zenoh** is what
  **unlocks/starts the device's REST API + sockets** (and issues whatever
  token/endpoints the device-native transport then uses). **Zenoh is the
  gatekeeper.**
- **Layer 0 runs in the rubix agent, not the browser and not the extension.** The
  **agent** speaks Zenoh (Rust-native), discovers devices, performs auth, *and is
  the data pass-through* to devices. The browser talks to **exactly one endpoint —
  the agent** — over the existing tenant session. No browser Zenoh client, no CORS,
  no mixed-content, no direct device-reachability requirement.
  - **Placement matters:** this lives in the **agent core** (an Axum route, like the
    SDUI router), **not** the `com.nubeio.ce` extension process — the extension SDK
    cannot proxy raw WS (`http_out` is a single JSON round-trip). This **supersedes
    "Connection Model A (direct browser→CE)"** from the original plan, which was only
    forced by that extension limitation.
  - **Cost to size:** the agent becomes a **real-time fan-in/out relay** — a hop on
    every high-freq frame and per-session device connections to hold. Fine on LAN;
    forward bytes for binary streams. A capacity consideration, not a blocker.
- **Two device tiers (downstream of the agent):**
  - **Thin device** → Zenoh *is* the whole API; agent relays Zenoh pub/sub
    (stream subjects) and queryable/storage (query subjects).
  - **Rich device (CE)** → Zenoh for discovery + auth, then the device's **own REST
    + binary WS** carry structure/data/actions; the agent relays them.

So a **ConnectionProvider in the agent** turns a discovered+authed device into a
server-side transport, and the **client sees a single transport — the agent
socket** (§5). Backend routing (Zenoh / CE / Timescale, stream / query) is a
server-side concern. The provider is pluggable per surface: **Zenoh** for devices
(use cases 2, 3); the **agent's own session + data plane** for the Timescale
dashboards (use case 1).

---

## 4. Data layer: two access patterns (this is the energy-volume correction)

A **subject** is an opaque addressable handle `{ kind, ref }`. Its `kind` picks the
access pattern — they have **opposite wire economics**, so both are first-class:

| Pattern | For | Wire shape | Update model | Example backends |
|---------|-----|-----------|--------------|------------------|
| **Stream** | live scalar / state | small frame per change | push every tick | CE binary WS prop; **Zenoh subscriber** |
| **Query** | windowed history / aggregates | one array per window | **re-query on window change**, or **incremental append** of new buckets; never re-ship full history | **Timescale**; `pg:`; Zenoh queryable/storage |

Key rule: **you never subscribe-stream a hypertable.** A query subject is fetched
for a *bounded window*; its subscription only signals "new data — append / re-query
this window." That's a per-subject version of today's invalidate, scoped to the
widget instead of the whole page.

In the IR, a widget binds to a subject (value is **not** inlined):

```jsonc
{ "type": "kpi",        "bind": { "subject": "analytics:meter_kwh_last_24h?tenant=site-a&map=kwh" } }   // query
{ "type": "chart",      "bind": { "subject": "analytics:meter_value_30d_15m?meter=site-a.elec.main" } }  // query (windowed, append)
{ "type": "live_value", "bind": { "subject": "ce:prop:5001" } }                                          // stream
{ "type": "gauge",      "bind": { "subject": "zenoh:site-a/ctrl/ahu1/supplyTemp" } }                     // stream
{ "type": "schedule",   "bind": { "subject": "ce:prop:5012" }, "commit": { "action": "setSchedule" } }   // stream + action
```

---

## 5. Transport: the substitution seam + capability advertisement

Steady state is **a persistent subscription (a socket), not request/response** —
the CE model generalised. **The client opens one socket: to the agent.** The
*agent* holds the per-surface transports below and routes/relays downstream
(devices via Zenoh, Timescale, `pg:`). So "transport" below is mostly a
**server-side** abstraction; the client just sees subjects arriving on its one
agent socket. A surface may still **mix subjects from several backends** (a mixed
dashboard, use case 5) — the agent fans them in.

```ts
interface Transport {
  // which subject prefixes / kinds this transport owns, and stream-vs-query each
  capabilities(): TransportCaps

  // STRUCTURE — server-authoritative, slow. Snapshot now, deltas later.
  openStructure(surfaceRef, ctx): StructureChannel   // emits: snapshot, then patches

  // DATA — stream subjects: push values; query subjects: fetch window + change-notify
  subscribeStream(subjects, onFrame): Unsub          // onFrame(subject, value, status)
  query(subject, window): Promise<Series>            // returns array for a window
  watchQuery(subject, window, onChange): Unsub       // "re-query / appended" signal

  // ACTIONS — mutations; response can targeted-patch, not force full refresh
  action(handler, args): Promise<ActionResponse>
}
```

The HTTP `resolve`/`action` endpoints stay for **first paint / SSR / one-shots**;
the socket carries steady-state deltas.

### Frame kinds on the socket

- `structure.snapshot` — full materialised tree (on connect / reconnect).
- `structure.patch` — add / remove / replace a node or subtree (Puck publish, a
  `repeat` set changing, an authz change).
- `value` — `{ subject, value, status }`, batched (CE-style binary for hot paths).
- `series.result` / `series.append` — a query window's rows, or just new buckets.
- `action.response` — toast / patch / dialog / redirect (the existing union).

Reconnect = CE's model exactly: client re-declares desired subjects + surface;
server re-sends `structure.snapshot` + a value snapshot.

---

## 6. Worked backends

| Transport | Connection (Layer 0) | Structure source | Stream subjects | Query subjects | Actions |
|-----------|----------------------|------------------|-----------------|----------------|---------|
| **Agent / dashboards** | agent tenant session | Pg dashboard pages (Puck/AI), authz-trimmed | (few) | **Timescale** analytics templates; `pg:` dims | tool registry → toast+refresh |
| **CE / wiresheet** (rich device) | **Zenoh discover + auth** → unlocks CE REST/WS | static IR file per **component type** | **prop uids** on binary WS (existing store) | (none, or short history later) | `callAction(uid,…)` / override |
| **Thin Zenoh device** | **Zenoh discover + auth** | static or contributed IR | Zenoh **key-expr subscribers** | Zenoh **queryable / storage** | Zenoh `put` / queryable |

Zenoh plays two roles, and it's worth keeping them separate:
1. **Layer 0 (always):** discovery + auth for *every* device — including the CE,
   whose REST/WS are unlocked by Zenoh auth.
2. **Data transport (thin devices only):** when a device has no richer API, its
   data also rides Zenoh — pub/sub (stream) + queryable/storage (query), over a
   router reached by a WebSocket bridge (`zenoh-ts` / remote-api). The CE does *not*
   use Zenoh for data; it uses its own binary WS after Zenoh auth.

The wiresheet panel is then the degenerate surface: Zenoh-authed CE connection, one
transport, all stream subjects, static structure.

---

## 7. The genuinely tricky parts (structure that depends on data)

- **`show_when`** (visibility driven by a value): push the driving value as ordinary
  **data** and evaluate the predicate **client-side** — cheap, no structure churn.
- **`repeat`** (list length driven by a set): **server** watches the driving subject
  and emits a `structure.patch` when the set changes (client can't synthesise nodes
  safely). 
- **Authz / multi-tenant**: stays a **Structure** concern — computed server-side on
  connect and on change, *not per value*. The live data path then carries no
  per-tick auth cost. For **device surfaces, the agent brokers auth via Zenoh
  (Layer 0)** and proxies the connection: the browser holds only its tenant session
  to the agent, and the agent holds the Zenoh-authed device connection — so device
  access is authz'd at the agent like every other surface.
- **First paint / SSR**: HTTP `resolve` returns a structure snapshot (optionally
  with a first value batch inlined) so the page paints before the socket warms.
- **Backpressure / coalescing**: hot stream subjects are rAF-coalesced client-side
  (the wiresheet value store already does this); query subjects are debounced on
  window change.

---

## 8. Widgets: shared core + lazy domain packs

The registry already supports `registerRenderer`. Split it so bundles stay lean:

- **Core** (all surfaces): row/col/grid/card/tabs, text/heading/badge,
  form/field/select/toggle/slider/button, `live_value`, `table`.
- **Dashboard pack**: kpi / chart / sparkline / date_range (pulls uplot — lazy).
- **Control pack**: schedule / gauge / override-cell / point-list (pulls nothing
  heavy — lazy; used by the wiresheet/Zenoh surfaces).

---

## 9. Migration from current SDUI (incremental, not a rewrite)

1. **Subject-scope the existing invalidate.** Make `subscribe` deliver per-subject
   instead of whole-page invalidate — immediate win for dashboards, no IR change.
2. **De-inline values.** `resolve` returns subjects + a first value batch; widgets
   read from a client value store. Dashboards keep working (slow subjects).
3. **Add the socket transport** with `structure.snapshot/patch` + `value` frames;
   keep HTTP resolve for first paint.
4. **Add the CE transport + control widget pack** — lands use case 2 on the same
   renderer (replaces the `custom`/`renderer_id` escape-hatch plan in
   `COMPONENT_UX_DESIGN.md`).
5. **Add the Zenoh transport** — use case 3, reusing the stream/query seam.

Until step 2 lands, the wiresheet panel can still ship via the `custom` variant +
a CE-backed transport (per `COMPONENT_UX_DESIGN.md`) — but it'd be a second-class
citizen, which is the thing this redesign removes.

---

## 10. Shell — tabs, selection, and the root-ext UIs

How UIs surface in the wiresheet's right drawer (and, later, any host shell).
**Decided.**

**Audiences (kept separate — "split now, combine later"):**
- **Dashboards** — integrators, runtime-authored, per-tenant in DB.
- **Device / extension UIs** — extension devs, authored at dev time, **shipped as a
  file in the extension**.
Both target the same renderer + IR + field-descriptor; only the *source* differs.
We build the device path now and **defer** unifying the authoring surface.

**Tab host (right drawer):**
- On **first load** the host fetches the **UI list** (manifest) and **allocates tabs
  once** — stable, not per-selection.
- Single-content: the active tab **fills the whole drawer**; never table + UI at once.
- **One Table per extension** (covers all its components).
- Tab switching is **manual** — selecting a component never auto-switches tabs.

**Selection contract (shared host state; each UI declares participation):**
- `ignore` — independent surface (scheduler).
- `follow` — highlights the host's selected record.
- `drive` — interacting selects/focuses the node on the canvas.
- `sync` — both (the Table).
Selection is bidirectional shared state between the graph and the UIs; the UI
definition opts in.

**Everything is a UI — including the Table.** Because selection behaviour is a
per-UI concern, the Table is just a UI that declares `sync`, **shipped by the root
extension**. To avoid reimplementing its rich interactions declaratively:
- Keep the feature-rich table as a **built-in `collection` widget** (today's
  `ComponentTable`, adapted to read columns from field descriptors + bind the
  selection context).
- The root-ext **Table UI** is a one-line declarative page that places that widget
  full-bleed with `selection: sync`.
The renderer *hosts* the widget; it never expresses ctrl/shift-select etc. in the
DSL. The same pattern later yields the default **Configure** (record) UI.

**Field descriptors unify `__facets`.** A CE component is a collection whose fields
are its properties; `__facets` is its field schema (label/unit/decimals/alias/
hidden/order) layered on `/schema` (type/readonly). Columns and form editors render
from one `FieldDescriptor` regardless of origin. **View = per-type structure
(shipped); facets = per-instance presentation (live).**

**Manifest shape (stubbed until the API ships).** The root ext's UI list is stubbed
locally in `src/lib/ui/root-ext-stub.ts` behind `getUiManifest()`, mirroring the
future `GET /api/v0/ui/list` (see `API_REQUESTS.md` §5):
`{ version, uis: [{ id, label, icon, selection, view }] }`, where `view` is
`collection | record | layout`. Swap the shim for a real fetch when the engine
serves it.

## 11. Open questions

- **Subject addressing grammar** — URI-ish strings (`ce:prop:5001`,
  `zenoh:site-a/...`, `analytics:tmpl?params`) vs structured `{kind,ref,params}`.
  Strings author/serialise nicely; structured validates better. Lean structured in
  the IR, string as shorthand.
- **Who owns the subject registry** for Puck pickers across transports (the
  `Catalogue` seam today is dashboard-centric).
- **Event/log shape** (alarms) — third data pattern or query-with-append?
- **Zenoh Layer-0 contract** — what does "auth through Zenoh unlocks REST/WS"
  return: a token the device-native transport presents, dynamically-opened
  endpoints, or both? And what does discovery advertise per device (id, type,
  endpoints, capabilities, which subject kinds)?
- **Browser → Zenoh reachability** — *decided:* the **agent** brokers Zenoh
  (discovery + auth) and **proxies** device traffic; the browser holds one session
  to the agent only. Open follow-ons: where this lives (**agent core** Axum route,
  confirmed not the extension) and how the agent's **WS relay** is sized for
  high-freq binary streams (per-session device connections, byte-forwarding,
  backpressure).
- **RN in scope?** — decides whether the IR may carry any web-isms.
