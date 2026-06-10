# CE Wiresheet — performance notes

How the editor stays fast on large sheets, what was optimized, and what's left.
Each section stands alone — review them separately.

---

## 1. The "full reload" pattern (the thing to avoid)

`reload()` (in `CeEditor.tsx`) is the heavy operation:

1. `GET /nodes` (root) **or** `GET /nodes/uid/{X}?depth=1&nested=true&withEdges=true`
   — pulls **every child of the current folder + every edge** in one request.
2. Rebuilds **every** React Flow node (`buildRfNodes`) and edge (`buildRfEdges`),
   plus ghost nodes for cross-folder edges.
3. Replaces the whole `nodes`/`edges` arrays → React Flow re-renders the lot.

Cost scales with **folder size**. So calling `reload()` after a single mutation
(add one node, add one edge) makes that mutation feel slow on a big sheet — the
"lag spike before it appears". It also briefly races user clicks (a reload landing
in the same React batch as a click can swallow the click).

**Rule of thumb:** only `reload()` when you're loading a *different* view (folder
navigation). For a mutation on the *current* view, update incrementally.

---

## 2. Audit — every place that reloads

| Path | Trigger | Status |
|---|---|---|
| Add node | `onAddNode` (pane menu, drag-drop, palette) | ✅ **incremental** — appends the one node |
| Add edge | `onConnect` (drag handle→handle) | ✅ **incremental** — appends the one edge |
| Remove node/edge | `topologyRemoved` | ✅ already splices out, no reload |
| Move / rename | `topologyChanged` (position/name only) | ✅ already patches in place |
| Folder navigation | `enter`, `goToCrumb`, `goToComponent`, post-nav focus | ⚪ reload — **correct** (loading a new view) |
| Paste | `pasteFromClipboard` → `copyNodes` | 🟡 reload — multi-node + internal edges + ghosts |
| Connect-to (link / new component) | `ConnectPicker` → `connectEdge` / `createComponent` | ✅ **incremental** — reloads only for a cross-folder target (ghost) |
| Reparent / Move-into | `MoveIntoPicker` | 🟡 reload — node leaves the current folder |
| Property add/remove | `topologyChanged` with `addedProperties`/`removedProperties` | 🟡 reload — REST is source of truth for shape |
| Other session's add | `topologyAdded` (not ours) | 🟡 reload — needs backfill |

✅ done · ⚪ legitimate (unavoidable) · 🟡 candidate for incremental later

---

## 3. How the incremental add works

### Optimistic local append
`onAddNode` / `onConnect` do the REST write, then build **just the new
node/edge** from the response and push it onto the existing array:

- `POST /nodes` returns the full `Component` → `upsertComponent` + append one RF node.
- `POST /edge` returns `{uid} & Edge` → `upsertEdge` + append one RF edge.
- `onConnect` only fires for a drag between two **visible** handles, so the edge
  is always in-folder (no ghost) — safe to append directly.

### Skipping the redundant WS reload
Every structural write also produces a WebSocket `topologyAdded` event, which
*used* to call `reload()`. Now the handler skips the reload **iff the store
already contains everything the event adds**:

```
haveAll = msg.components.every(c => store.has(c.uid))
        && msg.edges.every(e => store.edges.has(e.uid))
if (haveAll) return            // we appended it optimistically → nothing to do
else scheduleTopologyReload()  // another session / paste / picker → backfill
```

This is presence-based, **not** origin-based — important, because an
origin-based "skip my own session" rule would break the Connect-to picker and
paste, which rely on the reload to render what they added.

---

## 4. Remaining candidates (if they ever feel slow)

These still `reload()` because they involve **edges + cross-folder ghosts**, which
are fiddlier than a plain node append:

- **Paste** — appends N cloned components + their internal edges; some edges may
  be cross-folder → ghosts. Incremental version: append the returned clones +
  build in-folder edges, fall back to reload only if any clone has a cross-folder
  edge.
- **Reparent** — the moved node leaves the current folder, so it can just be
  *removed* from the local arrays (no full reload).

The generic enabler for all three: make `topologyAdded` **incremental for any
origin** (fetch new components by uid, append in-folder edges, fall back to
reload only when a cross-folder/ghost edge appears).

---

## 5. REST API implications (engine side)

- **Create endpoints must return the full created object.** `POST /nodes` →
  full `Component` (uid, position, properties); `POST /edge` → `{uid} & Edge`.
  The client renders straight from the response with **no follow-up GET**. If
  these were trimmed to just a uid, every add would gain a round-trip.
- **The big remaining O(folder) cost is the initial folder load**
  (`GET /nodes/uid/{X}?depth=1&nested&withEdges`) — the whole folder in one
  request. For very large folders, an API that supports a **windowed / viewport
  fetch** (only the components currently on screen, like the value subscription
  already does) would be the next lever.
- **`setRate` must tolerate rapid updates** (the zoom-rate + perf-scaling feature
  can emit frequent rate changes; the client throttles to 200 ms, but the engine
  should also not stall/crash under a stream — see the WS notes).

---

## 6. Selection authority (related, not strictly perf)

Selection (single-click, shift-toggle, right-drag marquee, edge click) is owned
entirely by the **document-level pointer handler**, not React Flow. RF's own
`select` changes are dropped in `onNodesChange`/`onEdgesChange`, and
`selectionKeyCode` is disabled. This avoids the two systems racing (which made
shift-click need several attempts). Elements stay **interactive**
(`elementsSelectable` left on) so edges remain clickable.
