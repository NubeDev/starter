# com.nube.hello

Minimal demo extension bundled with `examples/notes`. Proves the
end-to-end extension round trip on top of the `starter-extensions`
substrate — including the prefs + i18n singleton handshake every
starter-based product copies from here:

- **Tool** `com.nube.hello.greet` — registered into the host's
  `ToolRegistry` by `starter-ext-mcp::register_tools` and reachable
  over the same `/mcp` endpoint as the consumer's own
  `NoteSearchTool`.
- **REST** `GET /hello` — mounted by `starter-ext-server::rest_router`
  via `BuiltinRestDispatcher`.
- **UI** `HelloPanel` — a hand-written `remoteEntry.js` loaded by the
  notes frontend through `@nube/starter-ext-ui`'s Module-Federation
  runtime and mounted into the `sidebar` slot.

## What the UI panel renders

The panel reads three host singletons off the federation handle —
`react`, `@nube/starter-ui-core/preferences`, and
`@nube/starter-ui-core/i18n` — and renders four localised + formatted
surfaces:

| Surface | Source | Why |
|---|---|---|
| Greeting | `com.nube.hello.greeting` via the host's IntlShape | Catalog lookup — language flip in Settings flips this in one render. |
| Unread count | `com.nube.hello.unread` (ICU plural) | Proves the catalog handles plural forms (`{count, plural, …}`). |
| Today's date | `formatDate(Date.now())` against the host's resolved prefs | Matches the host chrome byte-for-byte (`22/04/2026` for `en-AU`). |
| BBQ temperature | `formatQuantity(22.44, "temperature", "celsius")` | BBQ override (`temperature_unit: F`) flips the display to `72.4 °F`. |

The panel never re-creates an `<IntlProvider>` or refetches
`/v1/me/preferences`. It binds to the host's single instance — one
source of truth, one fetch, one re-render on flip.

## How the catalog works

`block.yaml` declares:

```yaml
requires:
  - "@nube/starter-ui-core/preferences"
  - "@nube/starter-ui-core/i18n"
contributes:
  i18n:
    catalogs:
      en: i18n/en.json
      es: i18n/es.json
```

The notes server serves the JSON files at
`/extensions/com.nube.hello/i18n/<lang>.json`; the host's
`ExtensionCatalogLoader` fetches the **active language only** and
calls `registerExtensionMessages` so the keys land in the host's
`IntlShape` namespaced as `com.nube.hello.*`. Switching language
flips this panel and the chrome in one render — no reload.

## How the federation handshake works

`ui/remoteEntry.js` is a hand-written ES module (no bundler, no
transpile — SCOPE R7: static metadata only). The factory's
`singletons` block names the three host packages we consume; the
host refuses to load on major mismatch (`extension.singleton_mismatch`
telemetry) and warns on minor drift (`extension.singleton_minor_drift`).

```js
const factory = {
  singletons: {
    react: { version: "18.3.1" },
    "@nube/starter-ui-core/preferences": { version: "1.0.0" },
    "@nube/starter-ui-core/i18n": { version: "1.0.0" },
  },
  init(handle) {
    const React = handle.singletons.react;
    const PrefsContext = handle.singletons["@nube/starter-ui-core/preferences"];
    const IntlContext = handle.singletons["@nube/starter-ui-core/i18n"];
    handle.register({ components: { HelloPanel: makePanel(React, PrefsContext, IntlContext) } });
  },
};
```

The panel calls `React.useContext(PrefsContext)` / `useContext(IntlContext)`
against the host's instances — the entire catalog and prefs state
flow through context, no separate ipc.

## Cross-cuts demoed here

The `examples/notes` host applies these to every extension; this one
is the reference:

- **Locale fallback chain** — `es-MX` → `es` → `en` floor (D-NP.6).
- **Missing-key telemetry** — a missing catalog key returns the id
  verbatim and fires `i18n.message_missing` so platform dashboards
  surface the gap.
- **Multi-tab propagation** — a `setPreferences` in one tab is
  picked up by every other same-origin tab in one animation frame
  via `BroadcastChannel("starter-prefs")` (D-NP.9).
- **Render budget** — a single `setPreferences` call causes at most
  one re-render per consumer (`prefs-render-budget.test.tsx`).
- **A11y** — the provider sets `<html lang>` / `<html dir>` and
  announces "Language changed to <name>" via a polite `aria-live`
  region.
- **Dev catalog watcher** — edits to `i18n/en.json` or `i18n/es.json`
  during `vite dev` reload the page automatically.

## What this is intentionally **not**

- No required-fields schemas. The wiring is the only thing worth
  reading.
- No auth gate. The REST round-trip is open.
- No persistence. Calls are stateless.
- No `<IntlProvider>` inside the panel — the host owns the one
  instance, the panel consumes it.

## See also

- Extension i18n guide: [DOCS/extensions/guides/i18n.md](../../../../DOCS/extensions/guides/i18n.md)
- Operator-facing prefs guide: [DOCS/user/guides/prefs-in-extensions.md](../../../../DOCS/user/guides/prefs-in-extensions.md)
- End-to-end scope: [`examples/notes/user-pref.md`](../../../user-pref.md)
