# Dashboard API — REST & MCP usage

Hands-on notes for driving the SDUI dashboard system from outside the
browser. Verified against a running `make start` dev stack on
`2026-05-27`.

- Agent: `http://127.0.0.1:8088`
- Frontend: `http://127.0.0.1:5173`
- Bootstrap user (created by `make bootstrap`):
  - email: `op@example.com`
  - password: `rubix-dev-passwd`

## 1. Auth

Login sets two cookies: `starter_session` (HttpOnly) and `starter_csrf`.
Both are session-scoped (no expiry), so curl needs `-c` and `-b` on the
**same** invocation, or the session cookie is dropped.

```bash
# login → writes cookies into /tmp/jar.txt
curl -s -c /tmp/jar.txt -b /tmp/jar.txt \
  -X POST http://127.0.0.1:8088/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"op@example.com","password":"rubix-dev-passwd"}'
# → {"csrf_token":"…"}

# confirm
curl -s -b /tmp/jar.txt http://127.0.0.1:8088/api/v1/auth/me
# → {"subject":"…","email":"op@example.com","role":"admin"}
```

Mutating calls require the CSRF header:

```bash
CSRF=$(awk '/starter_csrf/{print $7}' /tmp/jar.txt)
# pass on every POST: -H "X-CSRF-Token: $CSRF"
```

## 2. Dashboard tools over REST

All tools dispatch through `POST /api/v1/tools/{tool_id}`.

| Tool | Purpose |
|---|---|
| `rubix.dashboard.list` | List dashboards in a tenant |
| `rubix.dashboard.get` | Fetch one page (full `body_json`) |
| `rubix.dashboard.create` | Create a new page |
| `rubix.dashboard.update` | Replace `body_json` / `title` / `tags` (optimistic concurrency) |
| `rubix.dashboard.delete` | Mark active revision superseded |
| `rubix.dashboard.duplicate` | Clone a page under a new `page_id` |
| `rubix.dashboard.page_set` | **Not a page editor** — writes a flow node slot value (e.g. thermostat setpoint). See "Issues" below. |

### List

```bash
curl -s -b /tmp/jar.txt -X POST http://127.0.0.1:8088/api/v1/tools/rubix.dashboard.list \
  -H 'Content-Type: application/json' -H "X-CSRF-Token: $CSRF" \
  -d '{"tenant_id":"system"}'
```

> Note: stored `page_id` is `dashboard.<slug>` (e.g.
> `dashboard.data-flow-site-a`); the URL slug at
> `/dashboards/data-flow-site-a` strips the `dashboard.` prefix.

### Get

```bash
curl -s -b /tmp/jar.txt -X POST http://127.0.0.1:8088/api/v1/tools/rubix.dashboard.get \
  -H 'Content-Type: application/json' -H "X-CSRF-Token: $CSRF" \
  -d '{"tenant_id":"system","page_id":"dashboard.data-flow-site-a"}'
```

Returns `body_json` (SDUI IR v5: `{ir_version, root}` where `root` is a
tree of `row` → `col` → widget).

### Create

```bash
curl -s -b /tmp/jar.txt -X POST http://127.0.0.1:8088/api/v1/tools/rubix.dashboard.create \
  -H 'Content-Type: application/json' -H "X-CSRF-Token: $CSRF" \
  -d '{
    "tenant_id":"system",
    "page_id":"dashboard.hello",
    "owner_principal":"system",
    "title":"Hello",
    "tags":["test"],
    "body_json":{
      "ir_version":5,
      "root":{"type":"page","id":"p","title":"Hello","children":[
        {"type":"row","id":"r","children":[
          {"type":"col","span":12,"children":[
            {"type":"kpi","id":"k1","label":"Answer","format":"number","unit_symbol":"",
             "source":{"type":"static","points":[[0,42]]}}
          ]}
        ]}
      ]}
    },
    "created_by":"op@example.com"
  }'
```

**Source shapes that actually render:**

- `static` KPI/chart: `{"type":"static","points":[[ts_ms, value], …]}`
  (a scalar `value` field is silently ignored — widget renders blank).
- `analytics_template`:
  `{"type":"analytics_template","name":"meter_kwh_last_24h",
    "params":{"tenant_id":"site-a"},"map":{"value_field":"kwh"}}`

### Layout: `page` → `row` → `col` → widgets

The renderer is strict about the layout container types. Get any of
these wrong and the page collapses to a vertical stack with no
diagnostic.

**Rules:**

1. **Root must be `page`** (not `row`, not `col`). The frontend's
   `render-page.tsx` wraps children in a `flex flex-col` and is the
   only node type that paints the page title.
2. **`row` is a 12-column CSS grid** (`grid grid-cols-12 gap-4`). Its
   direct children must be `col` nodes; anything else (e.g. a `row`
   nested directly inside a `row`) renders but breaks the grid math.
3. **`col` takes `span: 1..12`** (default 12). It maps to
   `col-span-N`. Children stack vertically inside the col
   (`flex flex-col gap-3`).
4. **Three side-by-side KPIs:** one `row` containing three `col`s
   with `span:4` each, each col holding one KPI.
5. **Stacked KPIs in a single column:** one `col` with `span:12`
   holding multiple KPI children — they stack vertically inside.
6. **Mixed:** alternate `row` siblings under `page` to compose
   multiple horizontal bands.

**Minimal mixed-layout body_json:**

```json
{
  "ir_version": 5,
  "root": {
    "type": "page", "id": "page", "title": "Demo",
    "children": [
      {
        "type": "row", "id": "kpis-row",
        "children": [
          {"type":"col","span":4,"children":[
            {"type":"kpi","id":"k1","label":"A","format":"number","unit_symbol":"",
             "source":{"type":"static","points":[[0,42]]}}]},
          {"type":"col","span":4,"children":[
            {"type":"kpi","id":"k2","label":"B","format":"number","unit_symbol":"",
             "source":{"type":"static","points":[[0,3]]}}]},
          {"type":"col","span":4,"children":[
            {"type":"kpi","id":"k3","label":"C","format":"percent","unit_symbol":"%",
             "source":{"type":"static","points":[[0,12]]}}]}
        ]
      },
      {
        "type": "row", "id": "stacked-row",
        "children": [
          {"type":"col","span":12,"id":"stack","children":[
            {"type":"kpi","id":"s1","label":"Stack #1","format":"number","unit_symbol":"",
             "source":{"type":"static","points":[[0,111]]}},
            {"type":"kpi","id":"s2","label":"Stack #2","format":"number","unit_symbol":"",
             "source":{"type":"static","points":[[0,222]]}},
            {"type":"kpi","id":"s3","label":"Stack #3","format":"percent","unit_symbol":"%",
             "source":{"type":"static","points":[[0,33]]}}
          ]}
        ]
      }
    ]
  }
}
```

Renders as:

```
[ A 42 ] [ B 3 ] [ C 12% ]
[ Stack #1 111 ]
[ Stack #2 222 ]
[ Stack #3 33% ]
```

**Tailwind @source gotcha.** The grid utilities
(`grid-cols-12`, `col-span-{1..12}`) live in
`packages/starter-ui-sdui-react/src/renderer/render-{row,col}.tsx`.
Tailwind v4 only scans files reachable from `@source` directives.
The SDUI package now ships a `scan-source.css` shim that consumer
apps import once from their main Tailwind stylesheet:

```css
@import 'tailwindcss';
@import '@nube/starter-ui-kit/scan-source.css';
@import '@nube/starter-ui-sdui-react/scan-source.css';
```

Without that import, the classes are tree-shaken, the DOM gets the
class strings, but no CSS exists for them — every row collapses to
default block layout and every col fills 100% width (KPIs stack one
per row with no warning). Mirror this pattern for any future SDUI
consumer app.

**Puck editor preview is not the same renderer.** The route
`/dashboards/$pageId/edit` mounts a Puck builder for visual editing;
its preview pane does not honor `row`/`col`/`span` the way the
runtime renderer does. To verify a page edit actually works, view
`/dashboards/$pageId` (no `/edit` suffix).

### Update (with optimistic concurrency)

```bash
# fetch first to get expected_revision_id
curl … rubix.dashboard.get … > /tmp/page.json

# build update body reusing revision_id
python3 - <<'PY' > /tmp/upd.json
import json
d = json.load(open('/tmp/page.json'))
body = d["body_json"]
# … mutate body in place …
json.dump({
  "tenant_id":"system",
  "page_id": d["page_id"],
  "expected_revision_id": d["revision_id"],
  "title": d["title"],
  "body_json": body,
  "created_by":"op@example.com",
}, open('/tmp/upd.json','w'))
PY

curl -s -b /tmp/jar.txt -X POST http://127.0.0.1:8088/api/v1/tools/rubix.dashboard.update \
  -H 'Content-Type: application/json' -H "X-CSRF-Token: $CSRF" \
  -d @/tmp/upd.json
```

If `expected_revision_id` is stale:

```json
{"error":"conflict: rubix.dashboard.update.conflict: page_id=… current_revision_id=…"}
```

Re-`get`, re-apply your edit, retry.

### Delete

```bash
curl -s -b /tmp/jar.txt -X POST http://127.0.0.1:8088/api/v1/tools/rubix.dashboard.delete \
  -H 'Content-Type: application/json' -H "X-CSRF-Token: $CSRF" \
  -d '{"tenant_id":"system","page_id":"dashboard.hello","deleted_by":"op@example.com"}'
```

## 3. MCP (JSON-RPC) usage

Endpoint: `POST /api/v1/mcp`. Same cookie + CSRF auth as REST.

### List tools

```bash
curl -s -b /tmp/jar.txt -X POST http://127.0.0.1:8088/api/v1/mcp \
  -H 'Content-Type: application/json' -H "X-CSRF-Token: $CSRF" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
```

37 tools registered; the dashboard ones are `rubix.dashboard.{list,get,
create,update,delete,duplicate,page_set}` plus the
`com.rubix.dashboard-assistant` skill wrapper.

### Call a tool

```bash
curl -s -b /tmp/jar.txt -X POST http://127.0.0.1:8088/api/v1/mcp \
  -H 'Content-Type: application/json' -H "X-CSRF-Token: $CSRF" \
  -d '{
    "jsonrpc":"2.0","id":2,
    "method":"tools/call",
    "params":{
      "name":"rubix.dashboard.update",
      "arguments":{ … same shape as REST body … }
    }
  }'
```

Response carries both `content[].text` (stringified JSON) and
`structuredContent` (parsed). Errors surface as `{"error": …}` inside
`structuredContent`, **not** as a JSON-RPC `error` field — check both.

## 4. Live updates (SSE)

```bash
curl -N -b /tmp/jar.txt http://127.0.0.1:8088/api/v1/dashboards/events
```

- First frame: `{"kind":"snapshot","items":[…]}`
- Per-edit frames: `{"kind":"updated"|"created"|"deleted",
  "page_id":"…","title":"…","tenant_id":"…"}`

## Issues / TODOs

1. ~~**`page_set` is mis-documented as a page edit verb.**~~ Resolved
   2026-05-27. The skill doc at
   `rubix/crates/rubix-skills/skills/dashboard-builder/SKILL.md` now
   describes the `get → mutate body_json → update` loop with
   `expected_revision_id` and explicitly notes that `page_set` writes
   flow slot values, not widget properties. `rubix.dashboard.get` is
   in the `allowed_tools` list.

2. **`page_set` accepts unknown `node_id` silently.** Calling with
   `node_id=com.acme.thermostat` (no such node registered) returns
   `applied: true`. Should reject unknown nodes.

   **Status: deferred — needs graph API.** `GraphStore` in
   [`crates/starter-flow-spi/src/graph.rs`](../../../../crates/starter-flow-spi/src/graph.rs)
   exposes only `write_slot`, `read_slot`, and `subscribe` — there is
   no "does this node exist?" method, so adding the existence check
   in [`rubix/crates/rubix-tools/src/dashboard/page_set.rs`](../../../crates/rubix-tools/src/dashboard/page_set.rs)
   requires either (a) extending the trait with `node_exists` /
   `list_nodes`, then implementing it on every backend (`InMemoryGraphStore`,
   the Postgres-listen backend, etc.), or (b) probing via
   `read_slot` against a known sentinel slot and catching
   `GraphError::UnknownNode` — which requires the trait to actually
   distinguish that case from "node exists, slot doesn't". Pick the
   trait extension; the surface is small.

3. ~~**`static` source schema is undocumented and surprising.**~~
   Partially resolved 2026-05-27. The KPI renderer at
   [`packages/starter-ui-sdui-react/src/renderer/render-kpi.tsx`](../../../../packages/starter-ui-sdui-react/src/renderer/render-kpi.tsx)
   now emits a deduped `console.warn` when a `static` source is
   supplied without `points` (and a different message when `value` is
   present as a hint that the shorthand isn't supported). The chart
   renderer should grow the same warning; tracked as a follow-up
   inside the renderer crate.

4. **No partial-update verb.** Every widget tweak round-trips the full
   `body_json`. Fine for hand-built pages, awkward for programmatic
   editors.

   **Status: deferred — design decision.** A `rubix.dashboard.patch`
   verb has to commit to a patch shape (RFC 6902 JSON-Patch vs.
   RFC 7396 JSON-Merge-Patch vs. a domain-specific
   `{path, op, value}` over the IR tree). It also has to integrate
   with optimistic concurrency (`expected_revision_id` still
   required), the changelog (the `Op::Update` `ChangeDraft` snapshot
   should remain a full `before`/`after` so undo round-trips, not a
   patch), and the SSE emitter. Land #6's `revision_id` field first;
   then patch becomes a small wrapper on top of `update`.

5. **`page_id` vs URL slug mismatch.** Stored id is
   `dashboard.<slug>`; URL uses `<slug>`. Easy to look up the wrong
   one from logs or the address bar.

   **Status: documented, structural change deferred.** Changing the
   stored shape would touch `dashboards_definitions`, every store
   implementation, every existing row, the resolver, the seed flows
   under `rubix/crates/rubix-flows/dashboards/`, and the skill doc.
   Not worth a migration. The mapping is: REST/MCP and the
   `dashboards_definitions` `page_id` column always carry the
   `dashboard.` prefix; the URL at `/dashboards/$slug` strips it; the
   `valid_page_id` check in
   [`rubix/crates/rubix-tools/src/dashboard/create.rs`](../../../crates/rubix-tools/src/dashboard/create.rs)
   enforces the grammar. When in doubt, check the stored row.

6. ~~**SSE delta payload is too thin to drive live re-render.**~~
   Resolved 2026-05-27. The `created` / `updated` wire shape already
   carried an optional `revision_id` field, but the audit middleware
   only saw the request body so the field never populated. The fix:
   [`rubix/crates/rubix-agent/src/middleware/changelog.rs`](../../../crates/rubix-agent/src/middleware/changelog.rs)
   now buffers the response body for the three dashboard write
   verbs and splices `revision_id` / `page_id` / `title` /
   `tenant_id` into the recorded `after` payload (request-side
   fields stay authoritative). On the frontend,
   [`packages/starter-ui-sdui-react/src/headless/sdui-page.tsx`](../../../../packages/starter-ui-sdui-react/src/headless/sdui-page.tsx)
   grew a `revalidateToken` prop wired into the resolve queryKey;
   the runtime route at
   [`rubix/frontend/src/routes/dashboards/$pageId.tsx`](../../../frontend/src/routes/dashboards/$pageId.tsx)
   feeds it `usePageLiveness().changeToken` so an SSE `updated`
   frame triggers a `/ui/resolve` refetch without a hard reload.
   Body diffing (originally option (b) here) remains out of scope —
   tied to #4 and not needed once refetch is in place.

7. ~~**Curl footgun.**~~ Resolved 2026-05-27. Already documented in
   §1 of this doc ("Both are session-scoped (no expiry), so curl
   needs `-c` and `-b` on the **same** invocation, or the session
   cookie is dropped"). Kept here as a cross-reference for anyone
   landing on this section first.

8. **MCP error channel is inconsistent.** Tool-level errors come back
   in `structuredContent.error` rather than the JSON-RPC `error`
   field. Clients written to the spec will treat failed calls as
   successful.

   **Status: deferred — needs cross-tool change.** The fix lives in
   the MCP dispatcher (search the agent for `tools/call` response
   construction; the current path wraps every tool result in a
   success envelope regardless of the inner `Error`). Mapping
   `Error::Invalid` → JSON-RPC `-32602`, `Error::NotFound` →
   `-32004` (custom), `Error::Conflict` → `-32009` (custom),
   `Error::Internal` → `-32603`, with the diagnostic code in the
   `error.data.code` field, is the right shape. This affects every
   tool — verify with the existing tool tests that the diagnostic
   code still surfaces somewhere a client can pattern-match on.

9. ~~**Layout container types are undocumented and silent on
   mistakes.**~~ Resolved 2026-05-27. Both
   [`rubix.dashboard.create`](../../../crates/rubix-tools/src/dashboard/create.rs)
   and [`rubix.dashboard.update`](../../../crates/rubix-tools/src/dashboard/update.rs)
   now run the structural validator in
   [`rubix/crates/rubix-tools/src/dashboard/layout.rs`](../../../crates/rubix-tools/src/dashboard/layout.rs)
   before persisting. It rejects (a) any root whose `type` is not
   `page` and (b) any `row` with a direct child whose `type` is not
   `col`, returning `Error::Invalid` so the transport surface
   reports HTTP 400 with a clear message. Update also now runs
   `validate_layout`; previously it skipped body validation entirely.

10. ~~**Tailwind `@source` is fragile.**~~ Resolved 2026-05-27.
    `@nube/starter-ui-sdui-react` now ships a `scan-source.css` shim
    (mirroring `@nube/starter-ui-kit/scan-source.css`); the rubix
    frontend imports it from
    [`rubix/frontend/src/styles/theme.css`](../../../frontend/src/styles/theme.css).
    New SDUI consumer apps should `@import "@nube/starter-ui-sdui-react/scan-source.css"`
    right after the `@nube/starter-ui-kit` shim, instead of writing a
    hand-rolled `@source` directive that points at the package
    source tree.

11. ~~**Two SDUI react packages exist.**~~ Resolved 2026-05-27 —
    `packages/starter-sdui-react` and `packages/starter-ui-ai-builder`
    were deleted as a closed dead-code subgraph;
    `packages/starter-ui-sdui-react` is the single SDUI react package.
    See [`sessions/sdui/2026-05-27-sdui-consolidation-final.md`](../../sessions/sdui/2026-05-27-sdui-consolidation-final.md).
## See Also

- **[Component Settings & Source Configuration](../../../../packages/starter-ui-sdui-puck/SETTINGS-AND-SOURCE.md)** —
  how the Puck editor generates settings fields from the IR schema,
  the `DATA_SOURCES` curation table, `<DataSourceField>` catalogue
  picker, and server-side source resolution.
- **[Mock Server (starter-ui-core)](../../../../packages/starter-ui-core/src/testing/mock-server.ts)** —
  dependency-free fetch shim for testing `StarterClient` auth flows
  without a running agent. Routes: `GET /auth/me`, `POST /auth/login`,
  `POST /auth/logout`. Strips `/api/v1` prefix automatically.