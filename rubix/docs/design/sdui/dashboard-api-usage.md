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
      "root":{"type":"row","id":"r","children":[
        {"type":"col","span":12,"children":[
          {"type":"kpi","id":"k1","label":"Answer","format":"number","unit_symbol":"",
           "source":{"type":"static","points":[[0,42]]}}
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
Tailwind v4 only scans files reachable from `@source` directives in
`rubix/frontend/src/styles/theme.css`. The SDUI package **must** be
listed there:

```css
@source "../../../../packages/starter-ui-sdui-react";
```

Without that, the classes are tree-shaken, the DOM gets the class
strings, but no CSS exists for them — every row collapses to default
block layout and every col fills 100% width (KPIs stack one per row
with no warning). Same fix applies to any future SDUI consumer
package whose layout classes Tailwind needs to see.

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

1. **`page_set` is mis-documented as a page edit verb.**
   `rubix/crates/rubix-skills/skills/dashboard-builder/SKILL.md` says
   "prefer `dashboard.page_set` with the changed widgets only" — but
   the tool writes flow node slot values (thermostat setpoints etc.),
   not widget properties. The only way to edit a widget is
   `get → mutate body_json → update`. Fix the skill doc.

2. **`page_set` accepts unknown `node_id` silently.** Calling with
   `node_id=com.acme.thermostat` (no such node registered) returns
   `applied: true`. Should reject unknown nodes.

3. **`static` source schema is undocumented and surprising.**
   `{"type":"static","value":42}` renders blank with no warning. The
   real shape is `{"type":"static","points":[[ts_ms, value], …]}`.
   Either accept `value` as a shorthand or log a render-side warning
   when `points` is missing.

4. **No partial-update verb.** Every widget tweak round-trips the full
   `body_json`. Fine for hand-built pages, awkward for programmatic
   editors. Consider a JSON-patch verb (`rubix.dashboard.patch`).

5. **`page_id` vs URL slug mismatch.** Stored id is
   `dashboard.<slug>`; URL uses `<slug>`. Easy to look up the wrong one
   from logs or the address bar. Either store without the prefix or
   document the mapping prominently.

6. **SSE delta payload is too thin to drive live re-render.** The
   `updated` frame contains only `kind/page_id/title/tenant_id` — no
   `revision_id`, no `body_json` diff. Currently the dashboard *edit*
   route does not refetch on the event, so users must hard-refresh.
   Either (a) include `revision_id` and have the client refetch
   `/ui/resolve` on mismatch, or (b) include a body diff. The sidebar
   list updates fine; the page body does not.

7. **Curl footgun.** Session cookie has no expiry so `curl -c jar`
   alone drops it. You must pass both `-c` and `-b` on the same call.
   Worth a one-liner in the README.

8. **MCP error channel is inconsistent.** Tool-level errors come back
   in `structuredContent.error` rather than the JSON-RPC `error`
   field. Clients written to the spec will treat failed calls as
   successful. Map domain errors onto JSON-RPC `error` with a stable
   `code`.

9. **Layout container types are undocumented and silent on mistakes.**
   Using `row` at the root, nesting `row → row`, or omitting `page`
   all produce a vertical stack with no warning. The resolver
   passes the tree through; the renderer dispatches on `type`; a
   tree that's "valid" but layout-wrong renders happily as a column
   of cards. Either (a) validate at `create`/`update` (reject
   non-`page` roots, reject `row` whose child is not `col`), or
   (b) emit a render-time `data-sdui-layout-warning` attribute so
   the page visually flags the mistake.

10. **Tailwind `@source` is fragile.** SDUI layout classes
    (`grid-cols-12`, `col-span-{1..12}`) only end up in the CSS
    bundle if the consumer app's `theme.css` lists the SDUI package
    under `@source`. Forgetting it produces the same silent
    column-of-cards bug. New SDUI consumer apps will all hit this.
    Options: ship a `scan-source.css` shim from
    `starter-ui-sdui-react` (mirroring `starter-ui-kit`), or move
    layout to inline styles so Tailwind isn't on the critical path.

11. **Two SDUI react packages exist.** ~~Resolved 2026-05-27~~ —
    `packages/starter-sdui-react` and `packages/starter-ui-ai-builder`
    were deleted as a closed dead-code subgraph;
    `packages/starter-ui-sdui-react` is the single SDUI react package.
    See [`sessions/sdui/2026-05-27-sdui-consolidation-final.md`](../../sessions/sdui/2026-05-27-sdui-consolidation-final.md).
