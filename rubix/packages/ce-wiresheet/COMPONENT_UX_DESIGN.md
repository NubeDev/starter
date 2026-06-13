# Component custom UX (declarative, manifest-driven)

Some components need a richer UI than the default prop rows — e.g. a **scheduler**
needs a week grid to view/edit. Goal: let a component declare a **custom panel**
that the wiresheet renders from **prebuilt FE elements**, with **no JavaScript
shipped by the extension**. The layout is data; values come from the component's
props; buttons trigger the component's actions.

## Decisions (settled)

- **Reuse the existing SDUI stack.** `@nube/starter-ui-sdui-react` is a headless
  renderer that dispatches one renderer per IR variant; the variant library
  already covers `form / table / grid / row / col / section / card / select /
  number_field / text_field / checkbox / toggle / slider / segmented /
  radio_group / button / tabs / repeat / list / json_table / kpi / chart /
  date_range / detail / dialog / drawer / divider / markdown`. `sdui-puck` gives a
  visual authoring editor for free. We do **not** build a new declarative UI
  framework — we point this renderer at a wiresheet component.
- **IR lives in a separate file, served by the API on request** (not inline in the
  manifest). The wiresheet fetches it lazily when a panel opens and caches it.
- **Writes go through component actions** (`callAction`) — the component validates
  and applies. Inputs stage locally in the panel; a button commits.
- **Panel opens from right-click → "Open UX"** on the node (and the table row),
  shown only for component types that have a UI file. Renders in a modal/drawer.
- **A dedicated `schedule` widget** is added to the renderer registry (the generic
  widgets aren't enough for a week grid).
- **One standard schedule schema** — the widget owns a fixed JSON contract; any
  scheduler-like component conforms. One widget serves all schedulers.

## Architecture

```
 component type ──(GET /ui/{type})──► SDUI IR (JSON)
                                          │
 wiresheet "Open UX"  ───────────────►  SduiPage(renderer)
                                          │  via a wiresheet SduiTransport:
   data refs  {$prop:"x"}  ◄──────────────┤   resolve to LIVE prop value (WS store)
   field refs {$field:"id"} ◄─────────────┤   resolve to the panel's staged edits
   on_press   {action,params} ────────────►  callAction(uid, name, params)
```

Three new pieces (the SDUI renderer + widgets already exist):

1. **UI-file API** — `GET /api/v0/ui/{type}` returns the IR for a component type
   (404 if none). Engine/host dependency — see `API_REQUESTS.md` §5.
2. **Binding transport** — a wiresheet `SduiTransport` that resolves data refs to
   live prop values (reusing the value store / WS stream), field refs to the
   panel's local staged state, and dispatches `on_press` to `callAction`.
3. **Panel host** — opens a component's IR in a drawer; subscribes to the
   component's props so bound widgets update live.

## Binding ref syntax (in the IR)

- `{ "$prop": "<propName>" }` — the component's live prop value (read-only stream).
- `{ "$field": "<widgetId>" }` — a widget's staged edit value within the panel.
- `"on_press": { "action": "<actionName>", "params": { ... refs ... } }` —
  resolve refs, then `callAction(componentUid, actionName, resolvedParams)`.

## The schedule contract

The component exposes its schedule as one structured prop (JSON string). The
`schedule` widget reads it, renders a week grid + entry editor, stages edits, and
Save commits via an action.

```jsonc
// the component's `schedule` prop value
{ "entries": [
    { "days": ["mon","tue","wed","thu","fri"], "start": "08:00", "end": "18:00", "value": 21 },
    { "days": ["sat","sun"],                    "start": "00:00", "end": "24:00", "value": 16 } ],
  "default": 16 }
```

The IR served for the scheduler type:

```jsonc
{ "variant": "section", "title": "Schedule", "children": [
    { "variant": "schedule", "id": "sched", "bind": { "$prop": "schedule" } },
    { "variant": "row", "children": [
        { "variant": "button", "label": "Save",
          "on_press": { "action": "setSchedule", "params": { "schedule": { "$field": "sched" } } } },
        { "variant": "button", "label": "Reset",
          "on_press": { "action": "resetSchedule" } } ] } ] }
```

## Build milestones (vertical slice first)

1. **API** `GET /ui/{type}` (engine) + a client fetch + cache in the wiresheet.
2. **Panel host + transport** — "Open UX" → drawer → SDUI renderer wired to the
   live value store and `callAction`. Prove with a generic IR (a couple of
   `text`/`number_field`/`button` widgets bound to real props/actions).
3. **`schedule` widget** end-to-end against a real scheduler component.
4. Polish: Puck authoring, more domain widgets as needed.

## Feasibility (spiked — confirmed)

The renderer mounts `<SduiPage>` under `<SduiProvider>` with an injected
`SduiTransport`. Its four methods map directly onto our needs — the live-data and
action wiring is exactly the seam the renderer was built around, not a workaround:

| `SduiTransport` method | wiresheet implementation |
|---|---|
| `resolve(req)`     | fetch the type's IR (`GET /ui/{type}`, or a local stub) |
| `subscribe(subjects, onUpdate)` | bridge to the WS value store — subjects = the component's props; fire `onUpdate` on each value tick (live data for free) |
| `action(req)`      | → `callAction(componentUid, name, params)` |
| `table(req)`       | from the component's props (or unused initially) |

Staged form edits use the package's `usePageState` / `usePageStateKey` (so
`$field` refs and Save-on-action come built in). The `schedule` widget is a new
entry in the renderer `registry`.

**Caveat — bundle weight.** `@nube/starter-ui-sdui-react` pulls
`@nube/starter-ui-kit`, `@tanstack/react-query`, `uplot`, `@nube/starter-ui-ir`.
**Lazy-load** the renderer (dynamic `import()` on first panel open) so it stays
out of the main wiresheet bundle.

## Open / to confirm

- Who serves `/ui/{type}` — the **CE** (co-located with `/schema`, natural) or the
  rubix-agent / extension host? Where the IR files are stored.
- Bundling: `@nube/starter-ui-sdui-react` pulls `starter-client-ts`,
  `starter-ui-kit`, `uplot` — confirm it bundles cleanly into the extension, or
  cherry-pick the renderer + the variants we use.
- The exact `schedule` JSON schema (days as names vs bitmask; overnight spans;
  timezone) — to finalise with the engine's scheduler component.
