# Theme editor

A reusable, in-browser theme editor that lets an admin restyle a
starter-based app — colours, typography, shape, shadow, branding —
without writing CSS or YAML. Built on top of `@nube/starter-ui-kit`
shadcn primitives and persisted via a pluggable `ThemeTransport` so
the same editor works against a starter-server REST backend, against
`localStorage`, or against a custom seam the consumer writes.

This doc covers the **frontend** half. The backend REST contract is
still pending — see [Backend pending](#backend-pending) and the
matching entry in [TODO.md](../../../TODO.md) (Phase 9).

---

## Where it lives

```
packages/starter-ui-core/src/theme-editor/   <- data + state + transport
  types.ts                token model (38 tokens + ShellConfig + ThemeDocument)
  defaults.ts             defaultLightThemeStyles / defaultDarkThemeStyles
  presets.ts              DEFAULT_PRESETS — 10 curated themes
  store.ts                Zustand store, 30-step undo/redo, dirty flag
  transport.ts            ThemeTransport interface + 3 impls
                          (http / localStorage / in-memory)
  utils/
    color-converter.ts    culori wrappers (any CSS colour → OKLCH / hex)
    contrast-checker.ts   WCAG 2.x ratio + AAA/AA/fail tiering
    parse-css-input.ts    `:root { … }` / `.dark { … }` → token map
    generate-css.ts       token map → CSS / YAML export strings
    apply-theme.ts        stamp tokens onto a DOM element (preview / runtime)
  hooks/
    use-theme-editor.ts   load + save lifecycle against a ThemeTransport
    use-theme-presets.ts  returns DEFAULT_PRESETS (hook so consumers can swap)

packages/starter-ui-kit/src/theme-editor/    <- React components
  theme-editor-page.tsx   top-level page; composes everything
  theme-gallery.tsx       horizontal-scroll preset cards
  color-token-editor.tsx  grouped token rows w/ WCAG contrast badges
  branding-editor.tsx     nav_title / hide_features / logo / favicon upload
  live-preview.tsx        sidebar + card + chart palette under live tokens
  import-css-dialog.tsx   paste a `:root { … }` blob, get a populated editor
  theme-actions.tsx       toolbar: light/dark, undo/redo, export, save

packages/starter-client-ts/src/endpoints/theme.ts   <- typed HTTP wrappers
  themeGet / themeSave / themeUploadLogo / themeDeleteLogo /
  themeUploadFavicon / themeDeleteFavicon
```

Per [SCOPE.md](../../../SCOPE.md) R6: `starter-ui-kit` stays I/O-free
— every network call is funnelled through a `ThemeTransport` the host
app constructs and hands in. `starter-ui-core` owns the brain.

---

## Quick start

```tsx
import { StarterClient } from "@nube/starter-client-ts";
import { httpThemeTransport } from "@nube/starter-ui-core/theme-editor";
import { ThemeEditorPage } from "@nube/starter-ui-kit/theme-editor";

const client = new StarterClient({ baseUrl: "/" });

export function ThemeRoute() {
  return (
    <ThemeEditorPage
      transport={httpThemeTransport({ client })}
      hideFeatureOptions={[
        { id: "global-ai-chat", label: "Global AI Chat" },
        { id: "page-builder",   label: "Page Builder" },
      ]}
      onNotify={(kind, msg) => toast[kind === "save_failed" ? "error" : "success"](msg)}
    />
  );
}
```

Gate the route behind your own admin-role check — the editor does no
authorisation of its own.

---

## Token surface

Thirty-eight CSS custom properties, identical to the set
[`globals.css`](../../../packages/starter-ui-kit/src/styles/globals.css)
declares on `:root` / `.dark`. Editing them restyles every primitive
in the kit:

| Group | Keys |
|---|---|
| Brand | `primary`, `primary-foreground`, `accent`, `accent-foreground`, `ring` |
| Surface | `background`, `foreground`, `card`/`-foreground`, `popover`/`-foreground`, `muted`/`-foreground`, `secondary`/`-foreground`, `border`, `input` |
| Status | `destructive`, `destructive-foreground` |
| Sidebar | `sidebar`, `sidebar-foreground`, `sidebar-primary`/`-foreground`, `sidebar-accent`/`-foreground`, `sidebar-border`, `sidebar-ring` |
| Charts | `chart-1` … `chart-5` |
| Shape | `radius` (drives the `--radius-{sm,md,lg,xl,2xl,3xl,4xl}` chain) |
| Typography | `font-sans`, `font-serif`, `font-mono`, `letter-spacing` |
| Shadow | `shadow-color`, `shadow-opacity`, `shadow-blur`, `shadow-spread`, `shadow-offset-x`, `shadow-offset-y` |

Plus the `ShellConfig` sidecar:

```ts
interface ShellConfig {
  nav_title: string;
  hide_features: string[];   // consumer-defined IDs
}
```

Colours can be typed in any CSS colour syntax (hex, `rgb()`, `hsl()`,
`oklch()`); `apply-theme.ts` normalises to `oklch(...)` at apply-time
because `globals.css` expects whole colour values (not channel
triplets). Non-colour tokens pass through verbatim — see
`NON_COLOR_KEYS` in `defaults.ts`.

---

## ThemeTransport

The persistence seam. Implement this to back the editor with any
storage you like:

```ts
interface ThemeTransport {
  load(): Promise<ThemeDocument>;
  save(input: { theme_styles: ThemeStyles; shell: ShellConfig }): Promise<ThemeDocument>;
  setLogo(file: File | null): Promise<void>;      // null = delete
  setFavicon(file: File | null): Promise<void>;
}
```

Three impls ship out of the box:

| Factory | Use when |
|---|---|
| `httpThemeTransport({ client })` | You have a `StarterClient` and a starter-server backend with the theme routes wired in. (Backend pending — see below.) |
| `localStorageThemeTransport({ key? })` | No backend yet. Single-tenant, asset uploads are no-ops. Good for demos and local dev. |
| `inMemoryThemeTransport(initial?)` | Tests. Resets on reload. |

Writing your own transport (gRPC, IPC, fleet orchestration, a config
file on disk) is the supported extension point — implement the four
methods and the editor doesn't notice the difference.

---

## REST contract (pending)

`httpThemeTransport` expects six endpoints. They are not yet
implemented in `starter-server` — the work is tracked in
[TODO.md Phase 9](../../../TODO.md#phase-9--theme-persistence-backend).
Until then, consumers can either ship the routes themselves or use the
`localStorage` transport.

| Method | Path | Body | Returns |
|---|---|---|---|
| `GET` | `/api/v1/ui/theme` | — | `ThemeDocument` JSON |
| `PUT` | `/api/v1/ui/theme` | `{ theme_styles, shell }` JSON | `ThemeDocument` JSON (or `204 No Content`) |
| `POST` | `/api/v1/ui/theme/logo` | raw bytes; `Content-Type` = file MIME | `204` |
| `DELETE` | `/api/v1/ui/theme/logo` | — | `204` |
| `POST` | `/api/v1/ui/theme/favicon` | raw bytes; `Content-Type` = file MIME | `204` |
| `DELETE` | `/api/v1/ui/theme/favicon` | — | `204` |

```json
// ThemeDocument
{
  "theme_styles": {
    "light": { "primary": "oklch(0.55 0.22 257)", "...": "..." },
    "dark":  { "primary": "oklch(0.72 0.18 257)", "...": "..." }
  },
  "shell": { "nav_title": "My App", "hide_features": ["page-builder"] },
  "logo_url":    "/static/theme/logo.png",
  "favicon_url": "/static/theme/favicon.ico"
}
```

Asset limits the frontend enforces (server should enforce too): logo
PNG / SVG ≤ 256 KiB; favicon PNG / ICO ≤ 64 KiB.

---

## Runtime apply (separate from the editor)

The editor stamps tokens on its preview pane only. To apply a saved
theme at app startup so it affects the whole UI, wire a small bootstrap
that reads `/api/v1/ui/theme` once and calls `applyThemeToElement`
against `document.documentElement` for the user's resolved mode. A
shared `useTheme()`-style hook for this lives outside this package
because it depends on the consumer's mode-selection strategy
(`<ThemeProvider>` from `@nube/starter-ui-kit`, an external system
preference, an account setting, …).

---

## Imports

```ts
// data + transport + hooks
import {
  useThemeEditor,
  useThemePresets,
  useThemeEditorStore,
  httpThemeTransport,
  localStorageThemeTransport,
  inMemoryThemeTransport,
  applyThemeToElement,
  generateCssString,
  parseCssInput,
  DEFAULT_PRESETS,
} from "@nube/starter-ui-core/theme-editor";

import type {
  ThemeTransport,
  ThemeDocument,
  ThemeStyles,
  ThemeStyleKey,
  ShellConfig,
} from "@nube/starter-ui-core/theme-editor";

// components
import {
  ThemeEditorPage,
  ThemeGallery,
  ColorTokenEditor,
  BrandingEditor,
  LivePreview,
  ImportCssDialog,
  ThemeActions,
} from "@nube/starter-ui-kit/theme-editor";

// HTTP wrappers (auto-attached to StarterClient via declaration merge)
import "@nube/starter-client-ts";  // ensures theme endpoints are loaded
```

> **Note:** the `theme-editor` subpath exports are declared in each
> package's `exports` field but are not yet re-exported from the main
> barrel (`./src/index.ts`). Import via the subpath form. Hoisting to
> the main barrel is tracked in TODO Phase 9.

---

## Wiring still needed

- [ ] Re-export `./theme-editor` from `packages/starter-ui-core/src/index.ts`
- [ ] Re-export `./theme.js` from `packages/starter-client-ts/src/endpoints/index.ts`
- [ ] Add `culori` (and `@types/culori`) to `packages/starter-ui-core/package.json`
- [ ] Declare `@nube/starter-ui-core` as a peer of `@nube/starter-ui-kit`
- [ ] Tests: store undo/redo invariants, parse-css round-trip,
      contrast tiering boundaries
- [ ] Backend (see [TODO Phase 9](../../../TODO.md#phase-9--theme-persistence-backend))

---

## Keyboard shortcuts

| Combo | Action |
|---|---|
| `Ctrl/Cmd + S` | Save |
| `Ctrl/Cmd + Z` | Undo |
| `Ctrl/Cmd + Shift + Z` | Redo |
| `Ctrl + Y` | Redo (Windows convention) |

History keeps the last 30 logical edits; rapid edits within 500 ms
collapse into a single entry (so dragging a slider doesn't blow the
ring buffer).

---

## Attribution

Colour science (`culori`), preset palettes, CSS parser, and contrast
checker patterns are adapted from
[tweakcn](https://github.com/jnsahaj/tweakcn) (Apache-2.0,
Copyright © 2024 Sahaj Jain). Each ported file carries the license
header inline. No tweakcn source code beyond the utility helpers and
preset data is included.

---

## Out of scope

- **External font CDN imports.** Font tokens accept system-font stacks
  only — no Google Fonts loader, no `@import url(...)`. Keeps CSP
  clean and avoids FOUC. (When the backend lands, the validator should
  also reject values containing `url(` or `@import`.)
- **Per-user themes.** The editor edits one shared org-level theme.
  Light/dark *mode* is a separate user preference owned by
  `<ThemeProvider>` in `@nube/starter-ui-kit`.
- **AI theme generation.** Out of scope; would require a server-side
  provider call. If a consumer wants it, wire a new gallery source via
  a custom `useThemePresets` replacement.
- **Per-component overrides.** v1 covers global tokens only.
