# CE API — requests from the wiresheet editor

What the `@nube/ce-wiresheet` editor needs from the Control Engine, in priority
order. The client side for each is already wired (or trivial) — these are the
engine-side pieces.

**Already landed** (verified against the live engine, removed from this doc):
`/copy/nodes` `uidMap` remap table · per-property `subscribe`/`unsubscribe` on the
WS · `reEvaluate` returning the recomputed downstream value (`reEvaluated`) ·
`POST /nodes/select` batch read with field projection · a `__facets` string
property (systemRole `ROLE_FACETS`) now present on **every** component at the
engine level. The override-by-value-frame report turned out to be a server fault,
not an API gap.

---

## 1. Override by prop uid within a component (exposed ports)  — small ask

(The engine has **no fast prop-uid → component** lookup, so "component uid optional,
resolve from prop uid" is NOT viable. The client therefore sends the correct child
**component uid** — stored in the facet at expose time — for edges and overrides on
exposed ports. Edges already take `{component uid + prop uid}`, so those need **no
API change** once the client sends the child's component uid.)

The one remaining gap is **overrides**, which are keyed by property **name**:
`PATCH /overrides/nodes/uid/{componentUid}` with `setOverrides: [{ property:
"<name>", … }]` (and `clearOverrides` as a list of **names**). The exposed-port row
only knows the child's **prop uid** (its handle), not its real name (the child is
off-canvas). So please accept a **prop uid** form addressed within the component
already in the URL — the component is known, so resolving the prop by uid inside it
is cheap (no global lookup):

```jsonc
PATCH /overrides/nodes/uid/{childComponentUid}
{ "setOverrides":   [ { "propertyUid": <uid>, "value": …, "duration": … } ],
  "clearOverrides": [ <propertyUid> ] }
```

Until that lands, the client **disables override on exposed-port rows** (you
override the real value inside the child). Edges/connect to exposed ports work
client-side by sending the child component uid.

**Note (separate, doc-vs-behavior):** the spec documents override `duration: 0`
(or omitted) as "permanent until cleared", but this engine treats `0` as the
default (~60s) and the override does not hold. Either the doc or the engine needs
to change — the client currently always sends an explicit non-zero duration.

---

## 2. Component UI definition (custom UX)  — new, for declarative component panels

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
