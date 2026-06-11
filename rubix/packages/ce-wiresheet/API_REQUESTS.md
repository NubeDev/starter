# CE API — requests from the wiresheet editor

What the `@nube/ce-wiresheet` editor needs from the Control Engine, in priority
order. The client side for each is already wired (or trivial) — these are the
engine-side pieces.

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

## 3. Override → value frame?  — **needs confirmation / likely fix**

**Symptom:** after setting an override (`PATCH /overrides/nodes/uid/{uid}`), the UI
shows the **OVR status flag** (status frame arrives) but the **value doesn't update
to the overridden value**.

**Question:** does setting an override push a **value frame** for the new (pinned)
value, or only a status frame? The editor renders the value purely from the value
stream, so if no value frame is emitted on override, the row keeps showing the old
value (or `—`). Please confirm — and if it's status-only, also emit a value frame
for the overridden property on set/clear.

---

## 4. `reEvaluate` returns the recomputed value  — optional / nice-to-have

`PATCH /edge/uid/{uid}` with `{ "reEvaluate": true }` currently returns the `Edge`.
If it instead **returned the recomputed downstream value(s)**, the editor could
apply them immediately instead of waiting for the next value tick — making
re-evaluate of a loopback edge feel instant regardless of push rate. (Today the
value only appears on the next tick, which is slow at low rates / zoomed out.)

---

## Notes
- Values are scalar (`string | number | boolean | null`) and the binary typeTags
  are all scalar — none of the above needs a new value type.
- CORS for the rubix frontend origin is already handled (separate, env-specific).
