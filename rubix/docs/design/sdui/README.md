# SDUI — server-driven UI surface for rubix dashboards

> Cites: [`SCOPE.md`](../../SCOPE.md), [`THIN-SLICE.md`](../../scope/THIN-SLICE.md).

The SDUI surface lets operators *and* the dashboard-assistant AI
flow author the same dashboard pages, rendered uniformly on web,
mobile, and Tauri without per-platform widget code. Goal 1 of
SCOPE landed via the branch `codeless/rubix-dashboards-goal-1` in
five phases (A–E); this directory holds the present-tense design
for every sub-area.

## Sub-design index

| Area | Owns |
|---|---|
| [`storage/`](./storage/README.md) | `dashboards_definitions` PG table, revisions, NOTIFY, page-level authz registration. |
| [`bindings/`](./bindings/README.md) | The six `starter-ui-bindings` / `starter-ui-ir` substrate fixes (per-variant `Bindable`, qualifiers, `Repeat`, synthetic ids, portable-subset flag, `$msg` source). |
| [`host-glue/`](./host-glue/README.md) | Rubix's four trait impls (`EntityGraph`, `PageProvider`, `QueryEngine`, `HandlerRegistry`) and the `sdui_router` mount under `/api/v1/ui`. |
| [`tools/`](./tools/README.md) | The seven `rubix.dashboard.*` verbs (`get`, `list`, `create`, `update`, `duplicate`, `delete`, `page_set`) with undo. |
| [`renderer/`](./renderer/README.md) | `@nube/starter-ui-sdui-react` — `<SduiPage>`, transport seam, per-IR-variant renderers. |
| [`ai-builder/`](./ai-builder/README.md) | The `com.rubix.dashboard-assistant` flow, the `dashboard-builder` skill, and the JSON dialect the LLM emits via tool args. |

Phase 4 (batched historical pulls) is scoped but deferred — see
[`docs/scope/dashboards/07-fetch-plan.md`](../../scope/dashboards/07-fetch-plan.md)
for the v2 hand-off.

## Per-user dynamic resource authz

User-defined SDUI pages are **resources** in starter-authz's
`ResourceRegistry`, not static routes. Each page registration adds
a `ResourceSpec`; reads and writes go through
`PolicyEngine::decide` with the calling principal. A user without
grant on a page sees a 404, not a 403 (deny leakage). This makes
"alice has dashboard X but bob doesn't" enforceable without a
hand-rolled authz layer.

## Resource URI scheme intersection

The MCP resource URI scheme (`rubix://<goal>/<resource>` per R12)
overlaps SDUI page ids. Rubix's convention:

- `rubix://dashboard/pages` — MCP resource listing all dashboards
  the caller can see.
- SDUI page ids are `dashboard.<slug>` (no `rubix://` prefix —
  that's MCP's namespace).

The two namespaces stay explicit so extensions don't conflate them.
