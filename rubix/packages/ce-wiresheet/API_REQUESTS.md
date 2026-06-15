# CE API — requests from the wiresheet editor

What the `@nube/ce-wiresheet` editor needs from the Control Engine, in priority
order. The client side for each is already wired (or trivial) — these are the
engine-side pieces.

---

## 0a. `/copy/nodes` returns a uid remap table  — small, generic

**Why.** `__facets` is an opaque string property to the engine, so a deep copy
duplicates its value verbatim — leaving the copied component's exposed-port records
(`c`=child component, `f`=child __facets prop, and the record **key**=child prop
uid) pointing at the **original** uids, not the copies. (Same class as edges, but
the engine can't remap facets because it has no concept of their format.)

The engine already computes the old→new uid mapping during copy (it uses it to
remap internal edges). Just **expose it** so the client — which owns the facet
format — can rewrite the uid references. One generic addition; reusable for ANY
uid-referencing value, so the engine never needs to learn the facet format.

Add `uidMap` to the `/copy/nodes` response (uids are three independent pools):

```jsonc
{ "data": {
    "nodes": [ /* new components */ ],
    "edges": [ /* new edges */ ],
    "uidMap": {
      "components": { "<oldUid>": <newUid>, … },
      "properties": { "<oldUid>": <newUid>, … },
      "edges":      { "<oldUid>": <newUid>, … }
    } } }
```

Client side: `remapFacetUids(facet, compMap, propMap)` rewrites the copied
components' `__facets` after the copy. Wired (no-op until `uidMap` is present).
Only **copy** needs this — reparent/move (Group, Move-into) preserve uids.

---

## 0. Override by prop uid within a component (exposed ports)  — small ask

(The engine has **no fast prop-uid → component** lookup, so "component uid optional,
resolve from prop uid" is NOT viable. The client therefore sends the correct child
**component uid** — stored in the facet at expose time — for edges and overrides on
exposed ports. Edges already take `{component uid + prop uid}`, so those need **no
API change** once the client sends the child's component uid.)

The one remaining gap is **overrides**, which are keyed by property **name**:
`PATCH /overrides/nodes/uid/{componentUid}` with `setOverrides: [{ property:
"<name>", … }]`. The exposed-port row only knows the child's **prop uid** (its
handle), not its real name (the child is off-canvas). So please accept a **prop
uid** form addressed within the component already in the URL — the component is
known, so resolving the prop by uid inside it is cheap (no global lookup):

```jsonc
PATCH /overrides/nodes/uid/{childComponentUid}
{ "setOverrides":   [ { "propertyUid": <uid>, "value": …, "duration": … } ],
  "clearOverrides": [ <propertyUid> ] }
```

Until that lands, the client **disables override on exposed-port rows** (you
override the real value inside the child). Edges/connect to exposed ports work
client-side by sending the child component uid.

---

## 1. Property-level subscribe / unsubscribe  — ✅ **LIVE** (confirmed in spec)

**Why.** "Exposed ports" (a folder showing a child's prop as its own port, see
`FACET_DESIGN.md` §9) needs a *single, off-canvas* property's value to stream —
not its whole component. The value stream is already keyed by **prop uid**, so
subscribing per-property is the natural granularity (minimal bandwidth, no need to
subscribe the whole child component just to read one port).

**WS messages** — mirror the existing component subscribe, with a `properties`
array of **prop uids**:

```jsonc
{ "type": "subscribe",   "properties": [5001, 5002] }
{ "type": "unsubscribe", "properties": [5001] }
```

**Behavior:**
- Push **value frames** (and status, if applicable) for subscribed properties,
  keyed by prop uid — **same binary frame format** as component subscriptions
  (just include these prop uids in the frame). No new wire format.
- **Additive with component subs:** a property's value should stream if EITHER its
  component is component-subscribed OR the property is property-subscribed.
- **Per session, honoured on reconnect.** The client clears its local view of the
  server's prop-sub set on (re)connect and re-sends the desired set via the same
  `subscribe` message — so the server can treat a resumed session's existing subs
  as already-present (no-op) and a fresh session re-subscribes cleanly. Same model
  as components today.

Client status: **done** — `CeRestWs.setDesiredPropSubscription(Set<propUid>)` diffs
and sends these messages; the editor calls it with the exposed prop uids per view.

---

## 2. `__facets` property on every component — prerequisite for presentation metadata

The whole `__facets` feature (labels, units, formatting, aliases, exposed ports —
`FACET_DESIGN.md`) reads/writes one property per component:

- Name **`__facets`**, **`input`** category, type **`string`**, systemRole
  **`ROLE_FACETS`** (2).
- **Writable** via `PATCH /nodes/uid/{uid}` with `{ properties: { "__facets": {
  "value": "<string>" } } }` (read-modify-write; the UI preserves fields it didn't
  touch).
- Streams automatically (it's an input). Components may also write their own
  `__facets` at runtime (e.g. to publish runtime dropdown options later).

Client status: **done & working** (labels/units/aliases persist today where the
prop exists). This item is just "make sure every component carries it."

---

## 3. Override → value frame?  — ✅ **RESOLVED (works; was a server fault)**

Originally reported as: after `PATCH /overrides/nodes/uid/{uid}` the UI showed the
OVR status flag but the value didn't update. **Re-tested against a healthy engine
and it works correctly** — setting `in1`/`in2` overrides streams the new values
*and* recomputes the output, and clearing returns to engine control. The earlier
symptom coincided with the engine restarting; no value frame was being emitted
because the engine wasn't computing at all. Locked in by an integration test
(`src/itest/dataflow.itest.ts` → "an override sets the live value…"): set in1=4,
in2=3 → out streams 7; clear → out streams 0.

---

## 4. `reEvaluate` returns the recomputed value  — optional / nice-to-have

`PATCH /edge/uid/{uid}` with `{ "reEvaluate": true }` currently returns the `Edge`.
If it instead **returned the recomputed downstream value(s)**, the editor could
apply them immediately instead of waiting for the next value tick — making
re-evaluate of a loopback edge feel instant regardless of push rate. (Today the
value only appears on the next tick, which is slow at low rates / zoomed out.)

---

## 5. Component UI definition (custom UX)  — new, for declarative component panels

**Why.** Some component types need a richer UI than the default prop rows (e.g. a
scheduler's week grid). The wiresheet renders these from a prebuilt SDUI widget
library, driven by a layout the extension author ships — **no JavaScript from the
extension**. The layout (an SDUI IR document) is stored as a separate file per
component type, and the frontend fetches it lazily when a panel is opened. See
`COMPONENT_UX_DESIGN.md`.

Add a read endpoint that returns the IR for a type (404 when the type has none):

```jsonc
GET /api/v0/ui/{type}        // type = "NubeIO-control::scheduler"
→ { "data": <SDUI IR document> }     // 200
→ 404                                 // type has no custom UI
```

**Update — UI-list + tabs plan** (see `SDUI_UNIFIED_DESIGN.md` §10). The wiresheet
loads a **list of UIs** at first connect and allocates a drawer tab per UI (one
Table per extension; manual tab switching; each UI declares a `selection` mode
`ignore | follow | drive | sync`). Add a list endpoint alongside the per-type fetch:

```jsonc
GET /api/v0/ui/list
→ { "data": { "version": 1, "uis": [
      { "id": "components-table", "label": "Table", "icon": "table",
        "selection": "sync",
        "view": { "type": "collection", "source": "components", "fullBleed": true } },
      { "id": "scheduler", "label": "Schedule", "icon": "calendar",
        "selection": "ignore",
        "view": { "type": "layout", "children": [ /* … */ ] } } ] } }
```

A `view` is a high-level `collection | record | layout` doc (the authoring DSL),
**not** raw IR — columns/forms derive from **field descriptors** (`__facets` +
`/schema`). Until this ships, the client stubs the manifest in
`src/lib/ui/root-ext-stub.ts` (`getUiManifest()`).

- **Static per type**, like `/schema`. Co-locating with `/schema` (same CE) is the
  natural home; the IR files live alongside the extension.
- The panel's data binds to the component's existing **prop values** (already on
  the WS stream) and its **actions** (already callable via
  `POST /call/nodes/uid/{uid}`), so no other API is needed — only the IR fetch.
- Optional later: include a `uiTypes: string[]` (or a per-component-type flag) in
  the `/schema` response so the UI knows which types have a panel **without** a
  probe request.

## Notes
- Values are scalar (`string | number | boolean | null`) and the binary typeTags
  are all scalar — none of the above needs a new value type.
- CORS for the rubix frontend origin is already handled (separate, env-specific).
