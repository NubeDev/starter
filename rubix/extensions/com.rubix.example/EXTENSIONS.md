# How rubix extensions work

This doc explains end-to-end how an extension bundle like
[com.rubix.example](.) is discovered, loaded, and rendered — both the
**Rust process half** and the **UI module-federation half** — and what
slots a UI contribution can extend in the rubix shell.

It is the practical complement to:

- [DOCS/extensions/scope/SCOPE.md](../../../DOCS/extensions/scope/SCOPE.md) — the SPI rules (R1–R12).
- [DOCS/backend/extension-manager/README.md](../../../DOCS/backend/extension-manager/README.md) — the host's loader/supervisor pipeline.
- [starter-extensions/packages/starter-ext-sdk-ts](../../../starter-extensions/packages/starter-ext-sdk-ts) — the TS SDK extensions import.

---

## 1. Anatomy of a bundle

```
com.rubix.example/                         ← bundle dir, name is irrelevant
├── block.yaml                             ← manifest (SCOPE R1)
├── rubix-example-extension                ← installed process binary (runtime.bin)
├── process/                               ← Rust source for the binary
│   └── src/main.rs                        ← #[derive(Extension)], handlers
├── ui/                                    ← built UI artefacts (served as static)
│   └── remoteEntry.js                     ← entry the host dynamic-imports
├── ui-src/                                ← TS source for the UI bundle
│   ├── remoteEntry.ts                     ← factory; exposes components
│   ├── main.tsx                           ← Main slot component
│   ├── sidebar.tsx                        ← Sidebar slot component
│   └── vite.config.ts                     ← build config (React externalised)
├── kinds/                                 ← JSON schemas + .md + .sql files
├── skills/                                ← SKILL.md bundles
└── flows/                                 ← contributed flows
```

`block.yaml` is the single source of truth. Every other path in the
bundle is referenced from there (`runtime.bin`, `contributes.tools[*]
.input_schema`, `contributes.ui.entry`, etc.) and must resolve
relative to the bundle dir.

### The manifest hash gate (SCOPE R3)

The proc-macro `#[extension(manifest = "../block.yaml")]` in
`process/src/main.rs` reads `block.yaml` at compile time and embeds
its SHA-256 into the binary. At spawn, the supervisor recomputes the
hash on disk and refuses to launch on mismatch — guaranteeing the
running binary was built **against the exact manifest the host is
about to load**.

> If you edit `block.yaml`, you MUST rebuild the binary. Cargo's
> incremental cache does not always notice (the proc-macro can't emit
> `cargo:rerun-if-changed`). Force it with
> `touch process/src/main.rs && make install`.

---

## 2. Loader → supervisor → contributions

Boot sequence inside `rubix-agent`:

```
                ┌───────────────────────────────────────────────┐
agent boot ───▶ │ starter_ext_host::Loader                      │
                │   1. scan(dir)         find every block.yaml  │
                │   2. validate_all()    serde + schema checks  │
                │   3. commit()          freeze registry        │
                │   4. seal()            no further mutation    │
                └───────────────────────────────────────────────┘
                            │ for each Validated record
                            ▼
                ┌───────────────────────────────────────────────┐
                │ starter_ext_supervisor                        │
                │   - verify manifest hash matches binary       │
                │   - spawn runtime.bin                         │
                │   - speak the JSON-RPC over stdio contract    │
                └───────────────────────────────────────────────┘
                            │ once running
                            ▼
                ┌───────────────────────────────────────────────┐
                │ rubix-agent boot adapters                     │
                │   - register contributes.tools[]              │
                │   - CREATE TABLE com_<id>__<name> for each    │
                │     contributes.warehouse_tables[]            │
                │   - register contributes.warehouse_templates[]│
                │   - wrap contributes.anomaly_rules[] with     │
                │     ToolAnomalyRule and append to RuleRegistry│
                │   - fold contributes.nodes[] into             │
                │     NodeKindRegistry                          │
                │   - mount contributes.ui.entry under          │
                │     /api/v1/extensions/<id>/ui/*              │
                └───────────────────────────────────────────────┘
```

The loader treats failures as **records, not panics**: a malformed
`block.yaml` produces a `Failed` record visible at
`GET /api/v1/extensions/<id>` with `failure` set, so an operator can
diagnose it without bouncing the agent.

---

## 3. The Rust half (tools, tables, rules, nodes)

A process extension is a single Rust binary built with
[`starter-ext-sdk`](../../../starter-extensions/crates/starter-ext-sdk).
Per SCOPE R8 it **only** depends on `starter-ext-sdk` — never on
`rubix-agent`, `rubix-domain`, or anything inside `starter/crates/`.

The minimal shape (see [process/src/main.rs](process/src/main.rs)):

```rust
use starter_ext_sdk::{Extension, serde_json::Value};

#[derive(Extension)]
#[extension(manifest = "../block.yaml")]
pub struct Example;

// What ctx capabilities this extension wants the host to satisfy.
starter_ext_sdk::requires! {
    name = ExampleCtx,
    capabilities = [warehouse_write, tracing],
}

// Generated trait — one method per contributes.tools[*].id, with the
// id mangled to a snake-case Rust ident.
impl ExampleToolHandlers for Example {
    type Ctx = ExampleCtx;

    fn handle_com_rubix_example_echo(
        &self,
        _ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        Ok(params)
    }
    // … one fn per declared tool
}

starter_ext_sdk::register_process_main!(Example);
```

Adding a new tool is a three-step edit:

1. Append to `contributes.tools[]` in `block.yaml` with its
   `input_schema` / `output_schema` / `description_file`.
2. Drop the schemas + markdown under `kinds/`.
3. Implement the generated `handle_<id>` method on `Example`.

The host wires `warehouse_write` so `ctx.warehouse_write(table, row)`
INSERTs into `com_<id>__<table>` with `tenant_id` stamped from
`ctx.caller()` — extensions cannot spoof cross-tenant writes (SCOPE R6).

---

## 4. The UI half (custom federation, not Webpack MF)

The "Module Federation" the rubix host implements is **not** the
Webpack/`@originjs/vite-plugin-federation` shape. It is a small,
SDK-defined factory contract that maps cleanly onto a dynamic
`import()` of a single ESM file. Concretely:

```
operator clicks "Load UI" on /extensions
        │
        ▼
host:  await import("/api/v1/extensions/<id>/ui/remoteEntry.js")
        │
        ▼  default export is an ExtensionRemoteFactory:
        │  { singletons: { react: { version }, "react-dom": { … } },
        │    init(handle: ExtensionRemoteHandle): void }
        │
        ▼
host:  ExtensionHostManager.registerExtensionRemote(id, ui, factory)
        │
        ▼  - reject if a declared singleton's major ≠ host's major
        │  - call factory.init(handle); the handle carries the host's
        │    React, react-dom, query client, intl, etc.
        │
        ▼
extension:  registerExtensionContributions(handle, {
              components: { Main, Sidebar },
            })
        │
        ▼
host:  <ExtensionSlot id="main">    renders the contributed `Main`
       <ExtensionSlot id="sidebar"> renders the contributed `Sidebar`
       each wrapped in <SlotContextProvider> + <HostBindingsProvider>
       so useSlotContext / useHostTheme / useHostTranslate just work.
```

### Why React is externalised (and how)

Two copies of React on the page break hooks: the `useContext` an
extension calls reads from its own React instance, while
`<SlotContextProvider>` was rendered by the host's React. The
extension would see `null` and crash with *"useSlotContext() called
outside `<SlotContextProvider>`"*.

The rubix host solves this with an importmap + window-globals:

| file | role |
|---|---|
| [rubix/frontend/public/shims/react.mjs](../../frontend/public/shims/react.mjs) (and friends) | re-export `globalThis.__rubixReact*` |
| [rubix/frontend/index.html](../../frontend/index.html) | `<script type="importmap">` maps `react`, `react-dom`, `react-dom/client`, `react/jsx-runtime` to those shims |
| [rubix/frontend/src/main.tsx](../../frontend/src/main.tsx) | publishes `globalThis.__rubixReact = React` (etc.) before any extension can load |
| [ui-src/vite.config.ts](ui-src/vite.config.ts) | `rollupOptions.external: ["react", "react-dom", "react/jsx-runtime", "react-dom/client"]` so the extension bundle leaves the imports bare |

Net effect: when the browser dynamic-imports `remoteEntry.js`, the
extension's `import "react"` resolves through the importmap to a
shim that returns the host's React instance. One React, one set of
contexts, hooks work.

The SDK contexts (`SlotContext`, `HostBindingsContext`,
`HostClientContext`) are likewise stashed on `globalThis` inside the
SDK itself so multiple bundled copies share the same `React.Context`
identity.

### The extension's `remoteEntry.ts`

See [ui-src/remoteEntry.ts](ui-src/remoteEntry.ts). Component keys
in `components: { … }` MUST match `contributes.ui.exposes[*].name` in
`block.yaml` — that is how `<ExtensionSlot/>` looks them up.

```ts
import {
  registerExtensionContributions,
  type ExtensionRemoteHandle,
} from "@nube/starter-ext-sdk-ts";

import Main from "./main";
import Sidebar from "./sidebar";

export default {
  singletons: {
    react: { version: "19.1.0" },
    "react-dom": { version: "19.1.0" },
  },
  init(handle: ExtensionRemoteHandle) {
    registerExtensionContributions(handle, {
      components: { Main, Sidebar },
    });
  },
};
```

---

## 5. Available UI slots

A slot is a named mount point the host renders with
`<ExtensionSlot id="…">`. The id is a free-form string in the SPI;
the rubix shell renders these today:

| slot id        | rendered by                                                                                                                              | intended for                                                                                       |
|----------------|------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------|
| `main`         | [rubix/frontend/src/routes/extensions.tsx](../../frontend/src/routes/extensions.tsx) (admin index) and [rubix/frontend/src/routes/extensions.$extId.$.tsx](../../frontend/src/routes/extensions.$extId.$.tsx) (per-extension route view)                                                     | the full extension dashboard. On `/extensions/<id>/<sub-route>` the slot is filtered to one extension and the sub-route is passed through `SlotContext.route` (read via `useExtensionRoute()`).                          |
| `sidebar-nav`  | [rubix/frontend/src/components/layout/app-sidebar.tsx](../../frontend/src/components/layout/app-sidebar.tsx)                              | a nav-tree (Extension name → tab → nested tab) rendered alongside the host's static/live NavGroups. |
| `sidebar`      | [rubix/frontend/src/components/layout/app-sidebar.tsx](../../frontend/src/components/layout/app-sidebar.tsx)                              | a compact status panel inside the global `AppSidebar`. Visible on every route while the sidebar is open. |

### Per-extension routes

The host serves `/extensions/<id>/<rest>` as a catch-all. It renders
the extension's `main` contribution filtered to that id and hands
`<rest>` to the extension via `useExtensionRoute()`:

```tsx
import { useExtensionRoute } from "@nube/starter-ext-sdk-ts";

function MainRouter() {
  const route = useExtensionRoute();   // "" on /extensions/<id>/,
                                       // "customers/by-country" on the deep link,
                                       // null when mounted in a non-route slot
  if (route === "customers/by-country") return <CustomersByCountry />;
  return <Dashboard />;
}
```

Sidebar nav-tree contributions deep-link into these routes by
emitting plain `<a href="/extensions/<id>/<rest>">` anchors. Full
page navigation is intentional — the extension bundle cannot import
`@tanstack/react-router` (not a published host singleton today), and
sidebar clicks are infrequent enough that an SPA-style navigation is
not worth the extra coupling.

An extension contributes to a slot by adding an entry under
`contributes.ui.exposes[]` with the matching `slot:` value. To
contribute to a slot that does not exist yet, the host shell must add
a corresponding `<ExtensionSlot id="…">` mount point first — slot
names are not magic, but a contribution targeting an unmounted slot
simply has nowhere to render.

---

## 6. Extending the sidebar — step by step

This is the exact pattern the example uses. The `sidebar` slot expects
small, low-chrome content (the AppSidebar is a navigation surface,
not a dashboard).

### 6.1 Declare the contribution in `block.yaml`

```yaml
contributes:
  ui:
    entry: ui/remoteEntry.js
    exposes:
      - name: Main
        module: "./Main"
        slot: main
      - name: Sidebar          # ← key MUST match remoteEntry.ts components.Sidebar
        module: "./Sidebar"
        slot: sidebar          # ← target the AppSidebar slot
```

`name` is the lookup key the host uses against the `components`
record passed to `registerExtensionContributions`. `module` is
informational today (it documents the source-tree origin) — the host
does not import it directly.

### 6.2 Write the component

Use `BlockShell` from the SDK so the host's error boundary, loading
skeleton, and slot context are wired automatically. See
[ui-src/sidebar.tsx](ui-src/sidebar.tsx) for the full file; the
shape is:

```tsx
import * as React from "react";
import { BlockShell, useSlotContext } from "@nube/starter-ext-sdk-ts";

export default function Sidebar(): React.ReactElement {
  return (
    <BlockShell>
      <SidebarInner />
    </BlockShell>
  );
}

function SidebarInner(): React.ReactElement {
  const slot = useSlotContext();   // { slotId, extensionId, theme, themeTokens, flags }
  // … render compact content. Use CSS vars (var(--color-*)) so the
  // panel inherits the host theme automatically.
  return <section data-ext-slot={slot.slotId}>…</section>;
}
```

### Contributing a nav-tree instead of a panel

If you want a navigation tree (the `Extension Name → tab → nested
tab` shape used by the host's built-in NavGroups), target the
`sidebar-nav` slot instead of `sidebar`:

```yaml
contributes:
  ui:
    exposes:
      - name: NavTree
        module: "./NavTree"
        slot: sidebar-nav
```

The host renders `<ExtensionSlot id="sidebar-nav" />` directly above
`<ExtensionSlot id="sidebar" />` in `app-sidebar.tsx`, so a nav-tree
contribution appears before any compact panels from the same
extension. See [ui-src/nav-tree.tsx](ui-src/nav-tree.tsx) for a
reference implementation — plain JSX styled with `var(--color-*)` so
it inherits the host theme. Avoid importing the host's shadcn
`Sidebar*` primitives directly; they are project-aliased and would
not resolve from the extension bundle.

Guidelines for sidebar contributions:

- **Keep it compact.** The AppSidebar collapses on mobile; keep the
  resting height under ~120px and avoid horizontal overflow.
- **Theme via CSS variables.** The host writes `--color-surface`,
  `--color-border`, `--color-foreground` etc. to `:root`. Reading
  them keeps the panel consistent across the host's theme modes for
  free.
- **No raw `fetch` for host APIs.** Use `useHostClient()` from the
  SDK (SCOPE R11). The example sidebar uses `fetch` only because the
  `/api/v1/extensions/<id>` endpoint is operator-shaped and outside
  the typed client surface today.
- **Deep-link to your `main` panel** rather than packing data tables
  into the sidebar.

### 6.3 Register it in `remoteEntry.ts`

```ts
import Sidebar from "./sidebar";

export default {
  singletons: { react: { version: "19.1.0" }, "react-dom": { version: "19.1.0" } },
  init(handle) {
    registerExtensionContributions(handle, {
      components: { Main, Sidebar },  // ← key matches exposes[].name
    });
  },
};
```

### 6.4 Build and reload

```sh
cd ui-src && pnpm run build      # writes ../ui/remoteEntry.js
# in the rubix frontend tab: hard-reload (Ctrl+Shift+R)
```

The rubix frontend auto-loads every enabled, UI-contributing
extension on first authenticated render — see
[rubix/frontend/src/lib/extension-autoloader.tsx](../../frontend/src/lib/extension-autoloader.tsx),
which calls `bootstrapExtensions()` once `AuthProvider` reports a
session. A hard-reload picks up a freshly built bundle without any
manual "Load UI" click. The "Reload UI" button on `/extensions` is
still useful for re-importing a bundle without reloading the page.

### Enablement persistence (end-to-end)

The "enabled" toggle on the admin row is durable across both agent
and frontend restarts:

1. **Backend (rubix-agent).** `useExtensionEnable` POSTs to the
   agent, which writes an `enabled` row into the enablement store
   (see [rubix/crates/rubix-agent/src/boot/extensions.rs](../../../rubix/crates/rubix-agent/src/boot/extensions.rs)).
   At boot, when `[extensions].autostart_enabled_records = true`, the
   agent calls `store.list_all()`, filters to `EnablementState::Enabled`,
   and respawns each supervisor. Disabled rows are skipped — they
   stay validated in the registry but no process runs.

2. **Frontend (rubix-frontend).** `bootstrapExtensions()` reads
   `GET /api/v1/extensions`, filters by `enabled !== false`, and
   dynamic-imports each `remoteEntry.js`. The list endpoint reflects
   the persisted state on every fetch, so a user who logs in after
   an agent reboot sees the same UI contributions they had before.

3. **"Load UI" button.** Operator-facing affordance for the
   *current page session* only — useful for re-importing a bundle
   after an in-place rebuild. It does not change the persisted
   enabled flag.

No agent restart is needed for UI-only changes — the agent serves
`ui/*` as static files. A binary change (anything in `process/`) or a
`block.yaml` edit requires `make install && make load`, since the
loader seals the manifest registry at agent boot.

---

## 7. End-to-end checklist for a new extension

1. `cp -r com.rubix.example com.acme.thing` and edit `block.yaml`'s
   `id`, `display_name`, `runtime.bin`.
2. Rename the binary crate in `process/Cargo.toml` and update
   `runtime.bin` to match.
3. Trim `contributes.*` to what you actually ship — every section is
   optional and additive.
4. Implement the generated `handle_*` methods in
   `process/src/main.rs`.
5. Author UI components under `ui-src/`, expose them in
   `remoteEntry.ts`, and add matching `contributes.ui.exposes[]`
   entries.
6. `make all` — builds the binary, installs it next to `block.yaml`,
   and restarts the agent so the loader rescans.
7. Visit `/extensions`, find your id, press **Load UI**. Hard-reload
   if the host frontend was running before the UI bundle existed.

If something doesn't load, `GET /api/v1/extensions/<id>` is the
single best diagnostic — it surfaces the lifecycle state, validation
failure, supervisor error, and the resolved manifest in one payload.
