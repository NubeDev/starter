# Building Nexus Extensions

> Verified live on 2026-06-11 against a running stack (API `127.0.0.1:4780`,
> UI `127.0.0.1:4790`), admin `admin@nexus.local`. Worked examples:
> [`com.nexus.hello`](../../extensions/com.nexus.hello) (a sidebar-nav panel that
> runs its own query-kind) and [`com.nexus.demo`](../../extensions/com.nexus.demo)
> (a full `main`-slot page with sub-routing). This document is the practical
> "how do I build one" guide; for the stack/ports see
> [00_setup/STACK.md](00_setup/STACK.md).

---

## 1. What an extension is

A Nexus extension is a **bundle** — a directory with a `block.yaml` manifest plus
the files it references. The manifest's `contributes:` block is per-field, so a
single bundle can span three planes at once:

| Plane | `contributes` field | What it adds |
|-------|---------------------|--------------|
| **Data** | `warehouse_templates[]` | query-kinds (SQL templates), the dispatcher's *third source* (file pack → **extension** → tenant overlay) |
| | `insights[]` | post-query transforms (Rhai scripts) in the global insight registry |
| **Control** | `tools[]` | tool calls dispatched into the running child over stdio JSON-RPC (process flavour only) |
| **UI** | `ui` | federation components mounted into named host slots |

Two **runtime flavours**:

- **builtin** — no process; the contributions *are* the product (`com.nexus.demo`).
- **process** — a supervised child binary the host spawns, health-checks, and
  restarts per `supervision:` (`com.nexus.hello`, `runtime.bin: nexus-hello-extension`).

### Where bundles live

| Source | Env var | Behaviour |
|--------|---------|-----------|
| In-repo (read-only) | `NEXUS_EXTENSIONS_DIR` (`crates/nexus-api/extensions`) | scanned at boot; `purge` removes DB rows but never the source |
| Uploaded installs | `NEXUS_EXTENSIONS_INSTALLS_DIR` (`.nexus-ext/installs`) | installed via the upload path; `purge` deletes from disk too |

> The registry is **sealed at boot**: `install` persists the bundle and answers
> `pending_restart: true`. Restart `nexus-api` to surface a newly-installed
> bundle live. In-repo bundles are already scanned at boot, so `install` only
> exercises the upload path.

---

## 2. The REST API

All routes are under `/api/v1` and authenticated by a **cookie session** (login
sets it; `EventSource` SSE uses a query-string token instead — see §4). The full
machine-readable surface is the OpenAPI spec at **`GET /openapi.json`** (`Nexus
API 0.1.0`). Note: the `/api/v1/extensions/*` admin routes are mounted by the
kernel and are **not** in that spec — they are listed in §5.

### Logging in

```sh
# Cookie session — what the UI and these examples use.
curl -fsS -c /tmp/nx.cookies -H 'content-type: application/json' \
  -d '{"email":"admin@nexus.local","password":"change-me-admin"}' \
  http://127.0.0.1:4780/auth/login
# → {"csrf_token":"…"}   (200; cookie stored in /tmp/nx.cookies)

C='-b /tmp/nx.cookies'; B=http://127.0.0.1:4780
```

### The route surface (from the live OpenAPI spec)

Grouped by area — every path below is real and returned `200`/valid output when
probed:

**Query & data**
```
POST   /api/v1/query                     run SQL or a kind ({sql,kind,params})
GET    /api/v1/query/kinds                kinds available to the dispatcher
GET    /api/v1/query-kinds                tenant-overlay kinds (CRUD: POST/PUT/DELETE)
GET    /api/v1/query-history             POST …/{id}/star
GET    /api/v1/datasources               POST (create) /test  •  {id}: GET/PUT/DELETE
POST   /api/v1/datasources/{id}/query    query a specific datasource
GET    /api/v1/datasources/{id}/schema   datasource schema introspection
GET    /api/v1/datasources/kinds         → csv, mqtt, parquet, postgres, zenoh
```

**Insights** (post-query transforms)
```
GET    /api/v1/insights                  registered insights (CRUD)
GET    /api/v1/insights/functions        the Rhai DSL vocabulary (columns, filter_gt, zscore…)
POST   /api/v1/insights/preview          dry-run a script against sample data
```

**Dashboards, panels, folders, nav, variables, tags** — full CRUD; `GET
/api/v1/nav` is the sidebar model (10 entries live), `GET /api/v1/dashboards`,
`/api/v1/folders`, `/api/v1/variables`, `/api/v1/tags/keys`.

**Flows / detections / alerts / agents / findings / audit** — the automation and
AI surface, including the three SSE feeds in §4.

### Running a query-kind (the data loop)

`POST /api/v1/query` in *kind mode*: the server resolves `kind` against its
registries and **ignores `sql`**.

```sh
curl -fsS $C -H 'content-type: application/json' \
  -d '{"sql":"","kind":"com.nexus.hello.ping"}' $B/api/v1/query
# → {"rows":[{"greeting":"hello from com.nexus.hello","server_time":"2026-…"}], …}

curl -fsS $C -H 'content-type: application/json' \
  -d '{"sql":"","kind":"com.nexus.hello.echo","params":{"message":"hi"}}' $B/api/v1/query
# → {"rows":[{"echoed":"hi"}]}      params validate against the kind's JSON Schema
```

### Applying an insight post-query

```sh
curl -fsS $C -H 'content-type: application/json' -d '{
  "sql":"", "kind":"some.kind",
  "insight":{"insight_name":"com.nexus.hello.zscore","params":{"column":"value","threshold":3.0}}
}' $B/api/v1/query
```

The insight runs **after** the query, in the Rhai sandbox, on the result frame.
Available functions: `GET /api/v1/insights/functions` (e.g. `zscore(col)`,
`anomalies(col, threshold)`, `filter_gt`, `select`, `rename`).

---

## 3. Authoring the manifest (`block.yaml`)

```yaml
v: 1
id: com.nexus.hello
version: 0.1.0
display_name: "Nexus Hello"
description_file: docs/README.md
authors: ["you@example.com"]

# builtin: omit runtime. process: declare the supervised binary.
runtime:
  kind: process
  bin: nexus-hello-extension          # exec'd as <bundle>/<bin>

supervision:                          # process flavour only
  restart: on_crash
  max_restarts: 5
  within_seconds: 60
  backoff: { initial_ms: 200, max_ms: 30000, jitter: true }
  health:  { interval_ms: 5000, timeout_ms: 2000 }
  shutdown_grace_ms: 5000

contributes:
  warehouse_templates:                # → query-kinds
    - name: com.nexus.hello.ping
      params_schema: kinds/ping_params.json
      sql_file: kinds/ping.sql
      tables: []                      # [] ⇒ lint requires no $caller_tenant_id predicate
    - name: com.nexus.hello.echo
      params_schema: kinds/echo_params.json
      sql_file: kinds/echo.sql
      tables: []

  insights:                           # → post-query transforms
    - name: com.nexus.hello.zscore
      script_file: insights/zscore.rhai
      params_schema: insights/zscore_params.json

  tools:                              # → dispatched into the running child
    - id: com.nexus.hello.echo_tool
      input_schema:  schemas/echo_in.json
      output_schema: schemas/echo_out.json
      description_file: docs/echo.md

  ui:                                 # → federation components → host slots
    entry: ui/remoteEntry.js
    exposes:
      - { name: HelloNav, module: "./Nav", slot: sidebar-nav }
```

**Query-kind authoring rules**

- `sql_file` is a SQL template; params are **bound, never inlined** (no
  injection). `$message` in the SQL ↔ `message` in `params_schema`.
- `tables: []` tells the lint the kind reads no tenant-scoped table, so it needs
  no `$caller_tenant_id` predicate — what makes `ping`/`echo` robust on a fresh
  DB. Kinds that read real tables must scope by `$caller_tenant_id`.
- `params_schema` is JSON Schema; `required` params are enforced before the kind
  runs.

On boot, kinds are linted and materialised into `nexus_extension_query_kinds`;
insights are compile-checked into `nexus_extension_insights`.

---

## 4. SSE (server-sent events)

There are **three** real SSE endpoints (`content_type: text/event-stream`).
Everything else under `/api/v1` is request/response JSON — note that
`GET /api/v1/alerts/events` returns a JSON array, **not** a stream, despite the
name.

| Endpoint | Auth | Purpose |
|----------|------|---------|
| `GET /api/v1/streams/{id}?token=…` | **query-string token** | live query subscription |
| `GET /api/v1/agents/sessions/{id}/events?token=…` | query-string token | agent session event feed |
| `GET /api/v1/flows/{id}/debug/stream` | cookie session | flow debug trace (gated by `POST …/debug/enable`) |

### The stream pattern (two-step)

A browser `EventSource` **cannot set headers**, so SSE auth is a signed token in
the query string, not the Bearer/cookie. You first create a subscription (which
mints the token), then connect its feed:

```sh
# 1. create the subscription (cookie-authed) — returns an id + signed token
curl -fsS $C -H 'content-type: application/json' \
  -d '{"sql":"SELECT …", "datasource":"…"}' $B/api/v1/streams
# → { "id":"…", "token":"…" }

# 2. connect the SSE feed (token in the query string)
curl -N "$B/api/v1/streams/<id>?token=<token>"
```

The token binds the exact (spec + datasource + tenant) tuple; a missing/expired/
forged token gives `401`. In the browser:

```ts
const es = new EventSource(`/api/v1/streams/${id}?token=${token}`);
es.onmessage = (e) => render(JSON.parse(e.data));
```

---

## 5. Extension admin & lifecycle

These are the kernel-mounted `/api/v1/extensions/*` routes (cookie-authed). They
map 1:1 to the [`Makefile`](../../extensions/com.nexus.hello/Makefile) targets.

| Route | Make target | Purpose |
|-------|-------------|---------|
| `GET  /api/v1/extensions` | `status` | list: `{id,state,enabled,restart_required}` |
| `GET  /api/v1/extensions/{id}` | — | detail |
| `POST /api/v1/extensions/install` (multipart) | `install` | upload a `.tar.gz` bundle → `pending_restart:true` |
| `POST /api/v1/extensions/{id}/enable` | `load` | flip enablement on |
| `POST /api/v1/extensions/{id}/disable` | `unload` | flip enablement off |
| `GET  /api/v1/extensions/{id}/cleanup` | `cleanup-preview` | dry-run: lists rows/caches/kinds/insights to remove |
| `DELETE /api/v1/extensions/{id}?purge=true` | `uninstall` | run every cleanup provider |
| `GET  /api/v1/extensions/{id}/ui/remoteEntry.js` | — | serve the federation bundle (ETag + `304` revalidation) |

```sh
make -C nexus/extensions/com.nexus.hello test     # full e2e probe
make -C nexus/extensions/com.nexus.hello status
```

The cleanup preview is honest about what `purge` will do, e.g.:

```json
{"id":"com.nexus.hello","items":[
  {"kind":"enablement_row"}, {"kind":"ui_cache","bytes":10223},
  {"kind":"warehouse_table","label":"query-kind com.nexus.hello.echo"},
  {"kind":"warehouse_table","label":"query-kind com.nexus.hello.ping"},
  {"kind":"warehouse_table","label":"insight com.nexus.hello.zscore"}
], "bundle":{"will_delete":true}}
```

---

## 6. Extending the UI: slots, federation, and the host client

### Slots

The frontend host mounts extension components into **named slots**. Two you'll
use:

| Slot | What it is | Example |
|------|-----------|---------|
| `sidebar-nav` | a navigation entry in the primary sidebar | `HelloNav` |
| `main` | a full page in the content area, routed at `/x/:extId/*` | demo `Main` |

The component `name` in `exposes[*]` **must match** the exported component name
the remote registers.

### Federation mechanics (the load path)

The host loads `ui/remoteEntry.js` as an ES module. It is **not**
Module-Federation shaped — do **not** use `@originjs/vite-plugin-federation`. The
host expects the SDK-shape factory `{ singletons, init(handle) }`:

```ts
// ui-src/remoteEntry.ts
import { registerExtensionContributions, type ExtensionRemoteHandle }
  from "@nube/starter-ext-sdk-ts";
import HelloPanel from "./panel";
import HelloNav from "./nav";

export default {
  singletons: { react: { version: "19.1.0" } },   // host enforces matching major
  init(handle: ExtensionRemoteHandle) {
    registerExtensionContributions(handle, { components: { HelloPanel, HelloNav } });
  },
};
```

**React is external, shared by the host.** The host publishes its own React on
`globalThis.__rubixReact*` and declares an importmap (`nexus/ui/index.html`)
pointing `react` / `react-dom` / `react/jsx-runtime` at `/shims/*.mjs`. Your
Vite build externalises React so the browser resolves those bare specifiers to
the host's instance — bundling React reintroduces the two-React-copies hook
crash that singleton negotiation exists to prevent.

```ts
// ui-src/vite.config.ts (essentials)
export default defineConfig({
  build: {
    outDir: "../ui", emptyOutDir: true, cssCodeSplit: false,
    lib: { entry: "remoteEntry.ts", formats: ["es"], fileName: () => "remoteEntry.js" },
    rollupOptions: {
      external: ["react", "react-dom", "react/jsx-runtime", "react-dom/client"],
      output: { inlineDynamicImports: true },     // one file: the host loads exactly remoteEntry.js
    },
  },
});
```

### The host SDK (`@nube/starter-ext-sdk-ts`)

| Export | Use |
|--------|-----|
| `BlockShell` | standard panel wrapper; provides slot context + error boundary. Wrap every exposed component. |
| `useHostClient()` | typed client over `@nube/starter-client-ts`; `client.apiPrefix` is `/api/v1`. |
| `useSlotContext()` | read `slotId`, host theme, feature flags. |
| `useExtensionRoute()` | for `main`-slot pages: the path tail after `/x/:extId/`, for sub-routing. |
| `useHostPrefs()` | host user preferences. |
| `registerExtensionContributions(handle, {components})` | the single registration call in `init`. |
| `fetchJson(client, url, init)` | cookie-authed fetch returning parsed JSON. |

**The data loop** — a panel that runs the extension's own query-kind:

```tsx
import { fetchJson } from "@nube/starter-client-ts";
import { BlockShell, useHostClient } from "@nube/starter-ext-sdk-ts";

export default function HelloPanel() {
  const client = useHostClient();
  const [row, setRow] = React.useState<any>(null);
  React.useEffect(() => {
    fetchJson(client, `${client.apiPrefix}/query`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ sql: "", kind: "com.nexus.hello.ping" }),
    }).then((r) => setRow(r.rows?.[0]));
  }, [client]);
  return <BlockShell>{row ? <div>{row.greeting} — {row.server_time}</div> : "…"}</BlockShell>;
}
```

This is the full WS-14 loop in one component: if it renders, federation load,
singleton negotiation, slot mounting, cookie auth, and third-source kind dispatch
all work end-to-end.

### A `sidebar-nav` entry

The host does **not** share `react-router-dom` as a singleton, so a remote's own
`NavLink` wouldn't see the host Router. Render a **plain `<a href>`** — the host
wraps the `sidebar-nav` slot in a click interceptor that turns it into SPA
navigation:

```tsx
export default function HelloNav() {
  return (
    <a href="/extensions"
       className="flex h-8 items-center gap-2 rounded-md px-2 text-sm
                  text-sidebar-foreground/80 hover:bg-sidebar-accent">
      <span aria-hidden>👋</span><span className="truncate">Hello Nav</span>
    </a>
  );
}
```

### Pages & shadcn/ui — the important caveat

**The host shares React, but NOT its shadcn/ui component library.** You cannot
`import { Button } from "@/components/ui/button"` from the host. Two supported
ways to get the shadcn *look*:

1. **Tailwind design tokens (recommended, zero deps).** The host ships its
   Tailwind theme as CSS custom properties. Use the host's token classes
   directly and your markup inherits the host theme automatically — this is what
   `com.nexus.demo` does for its full `main`-slot page:

   ```tsx
   <div className="mx-auto flex max-w-5xl flex-col gap-6">
     <p className="text-sm text-muted-foreground">Nexus Demo extension</p>
     <h1 className="text-2xl font-semibold tracking-tight">Overview</h1>
     <div className="rounded-lg border bg-card p-4">…</div>
   </div>
   ```

   Tokens like `bg-card`, `text-muted-foreground`, `border`, `bg-primary`,
   `bg-sidebar-accent` resolve against the host's `--background` / `--primary` /
   etc. — the shadcn aesthetic without importing a single host component.

2. **Bundle your own shadcn/Tailwind.** Add `@tailwindcss/vite` +
   `vite-plugin-css-injected-by-js` (the `com.rubix.example` pattern), generate
   shadcn components into your own `ui-src`, and let the injected CSS reference
   the host's CSS variables so it still themes correctly. Heavier bundle; only
   do this if you need components the token approach can't express.

**Full-page routing** (`main` slot): use `useExtensionRoute()` to read the path
tail and dispatch sub-pages yourself — the host mounts your component at
`/x/:extId/*` and forwards the tail:

```tsx
function MainRouter() {
  const route = useExtensionRoute();                 // "", "readings", "about/…"
  const page = route?.startsWith("readings") ? "readings"
             : route?.startsWith("about")    ? "about" : "overview";
  return <>{page === "overview" && <Overview/>}{page === "readings" && <Readings/>}…</>;
}
```

### Build & ship the UI

```sh
make -C nexus/extensions/com.nexus.hello ui-build   # vite build ui-src → ui/remoteEntry.js
make -C nexus/extensions/com.nexus.hello ui-dev     # watch mode
```

The built `ui/remoteEntry.js` is **committed** so the backend pack is complete
without a frontend toolchain.

---

## 7. Accessing data: insights, datasources, query-kinds

An extension reaches Nexus data three ways, all over the same `POST /api/v1/query`
the panels use:

- **Query-kinds** — your own contributed SQL templates (the third dispatcher
  source). Author them in `kinds/`, list them in `warehouse_templates[]`. This is
  the primary way an extension exposes data.
- **Datasources** — `GET /api/v1/datasources` (kinds: csv, mqtt, parquet,
  postgres, zenoh), `POST /api/v1/datasources/{id}/query` to query one,
  `GET …/{id}/schema` to introspect. Create with `POST /api/v1/datasources`
  (test first with `POST /api/v1/datasources/test`).
- **Insights** — Rhai post-query transforms. Contribute via `insights[]`, apply
  via `{"insight":{"insight_name":…,"params":…}}` on a query. Discover the DSL
  with `GET /api/v1/insights/functions`; dry-run with `POST /api/v1/insights/preview`.

All of these honour tenant scoping and the query caps; insights are
row-count-preserving by sandbox rule, so a result can never exceed the query caps.

---

## 8. Quick start checklist

1. Copy `com.nexus.hello/` (panel + nav) or `com.nexus.demo/` (full page) as a
   template.
2. Edit `block.yaml`: `id`, `version`, and the `contributes:` fields you need.
3. Author `kinds/*.sql` + `*_params.json` for data; `insights/*.rhai` for
   transforms.
4. Build the UI: `make ui-build`. Confirm `ui/remoteEntry.js` exists.
5. In-repo: drop under `NEXUS_EXTENSIONS_DIR`, restart `nexus-api`. Uploaded:
   `make pack && make install`, then restart.
6. Verify: `make test` (list → detail → UI bytes 200+304 → run both kinds).
7. Open the UI (`:4790`); your `sidebar-nav` / `main` contribution is mounted.
