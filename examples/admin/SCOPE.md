# test-ui-5 — Scope

## Purpose

A demo frontend that explores the **visual + interaction language** for the eventual `flow-agent`-style product, while staying aligned with the `starter` repo's mandate: *reusable libraries first, demos second*. Everything new that proves itself here is expected to graduate into [packages/](../packages/) (notably `starter-ui-core`, `starter-ui-kit`, `starter-sdui-react`).

This is **not** the production app. It is a sandbox for the design system and shell so the look-and-feel can be locked in before it's promoted into shared packages.

## Design brief (from user)

- **Hybrid feel**: modern marketing/website polish mixed with a shadcn/ui dashboard. IoT is the primary use case, but it must *stand out* — not look like another stock shadcn admin.
- **Core layout / structure** comes from [test-ui/](../test-ui/) (mature shadcn-admin shell: sidebar, header, command menu, theme switch, routing, data tables).
- **Visual flair / motion / glass touches** come from [test-ui-3/app/src/](../test-ui-3/app/src/) (floating sidebar, glassmorphism hero, metric cards, radial progress, performance chart, boot-intro animation).
- **End goal reference**: [examples/flow-agent/](../examples/flow-agent/) — once look-and-feel is signed off, patterns flow back into [packages/](../packages/) and eventually power that app.

## Stack (locked)

| Concern        | Choice                                           | Why |
|----------------|--------------------------------------------------|-----|
| Framework      | React 19 + Vite                                  | Matches test-ui and packages/* |
| Router         | TanStack Router (file-based)                     | User preference; matches existing extensions/test-ui |
| Styling        | Tailwind CSS v4 (`@tailwindcss/vite`)            | v4 token-driven theming is core to the "easy to re-theme" requirement |
| Components     | shadcn/ui (Radix primitives, copy-in)            | Standard for the repo |
| Animation      | [motion.dev](https://motion.dev/) (Framer Motion successor) | For the "stand out" motion layer — page transitions, micro-interactions, boot intro |
| Data/state     | TanStack Query + Zustand                         | Matches test-ui; also the host-provided singletons extensions expect |
| Icons          | lucide-react                                     | Matches test-ui |
| Charts         | Recharts                                         | Matches test-ui; sufficient for IoT dashboards in this phase |
| **MF host**    | **`@module-federation/vite` + `@nube/starter-ext-ui`**  | **test-ui-5 IS a Module-Federation host. See "Extension host" section below.** |
| i18n           | Deferred — wire seam only (EN/ES later per [[i18n_and_unit_prefs]]) | Out of scope for v1 of test-ui-5 |
| Auth           | Mock only (no Clerk in this demo)                | Keeps the sandbox standalone |

## Extension host (load-bearing — this changes everything)

**Missed on first pass — corrected here.** The starter has a real
extension system at [starter-extensions/packages/starter-ext-ui/](../starter-extensions/packages/starter-ext-ui/),
built on Module Federation. Any demo that pretends extensions don't
exist is wrong: the flow-agent endgame at [examples/flow-agent/](../examples/flow-agent/)
relies on contribution slots, and the whole point of test-ui-5 is to
de-risk the look-and-feel of *that* shell — slots included.

**Therefore test-ui-5 will be a Module-Federation host from day one.**

### What that buys us

- **Real wiring at design time.** The sidebar, header, dashboard, and
  even the appearance settings page expose `<ExtensionSlot id="..." />`
  drop zones. Whatever look-and-feel survives is one a real extension
  actually loads into.
- **The existing `hello-ui` example becomes the smoke test.** If
  `hello-ui` mounts into `sidebar` in test-ui-5 without code changes to
  the extension, the host shape is right.
- **Singletons get exercised early.** React, react-dom,
  `@tanstack/react-query`, `zustand` — all already declared as
  singletons by `starter-ext-ui`. test-ui-5 has to provide them
  (which it would anyway).

### Slots test-ui-5 will expose (v1)

| Slot id              | Where in the UI                                | Notes |
|----------------------|------------------------------------------------|-------|
| `sidebar`            | Bottom of the main sidebar                     | Matches what `hello-ui` already targets |
| `header.actions`     | Right of the header, before user dropdown      | For quick-action injections |
| `dashboard.widgets`  | Bottom strip of `/dashboard`                   | Where extensions can drop their own metric/widget cards |
| `settings.sections`  | Bottom of `/settings/appearance`               | So extensions can contribute settings panels |
| `command.commands`   | Inside the `cmdk` command palette              | Extension-provided commands (deferred until cmdk lands) |

Slot ids are free-form strings — the host owns the namespace. The above
is a starting set; we add more if a use case appears.

### Federation build setup

- **Bundler**: Vite + `@module-federation/vite` plugin (the official
  one). Rspack works too, but we already have Vite everywhere — staying
  on Vite keeps dev experience consistent.
- **Singletons exposed by host**: `react`, `react-dom`, `@tanstack/react-query`,
  `zustand`. Versions tracked in [package.json](./package.json); singleton
  negotiation handled automatically by `starter-ext-ui`.
- **Remote loading**: dynamic `import()` of `remoteEntry.js`. test-ui-5
  reads a small JSON manifest (`public/extensions.json`) at boot listing
  enabled extensions for the demo (URL, manifest path). Real registry
  comes later from the Rust side — this is the design sandbox.
- **Theme propagation**: `<ExtensionSlot theme={...} themeTokens={...} />`
  receives the resolved palette + mode so extensions visually match —
  this is the second reason the theming work needs to land first.

### How this affects look-and-feel

- Slot containers must look *intentional* even when empty (a calm
  "no extensions loaded" placeholder, not a broken hole).
- Slot containers must visually frame whatever loads — so even a
  plain panel rendered by `hello-ui` looks like it belongs.
- The appearance switcher in `/settings/appearance` writes tokens
  *and* passes them through to `<ExtensionSlot themeTokens={...}/>`,
  so we can visibly prove extensions re-skin with the host. **This
  is the test that the theming approach is real.**

## Theming requirement (first-class)

The user explicitly called out: **must be quick + easy to swap colours, fonts, radii — a theme switcher is coming.** Therefore:

- All design tokens live in **one** CSS file (`src/styles/theme.css`) using Tailwind v4 `@theme` + CSS custom properties.
- No hard-coded hex/rgb values inside components. Components consume tokens only (`bg-background`, `text-foreground`, `border-border`, semantic tokens like `--color-accent-glow`).
- Theme switcher supports: `light` | `dark` | `system` + a **palette selector** (e.g. `default`, `ocean`, `forest`, `sunset`) implemented as a `data-theme="<name>"` attribute on `<html>`, so swapping a palette = swapping one attribute, not editing components.
- Font is a single CSS variable (`--font-sans`) with one fallback variable for display (`--font-display`) to support the marketing-hero touch.
- Radii / spacing / shadow scales are tokens too — opens the door to "soft" vs "sharp" theme presets later.

## Layout shell

Borrowed from [test-ui/src/components/layout/](../test-ui/src/components/layout/), refreshed with test-ui-3 touches:

- **Sidebar**: collapsible (icon-only ↔ expanded), with a "floating" variant option (from `floating-sidebar.tsx`) toggleable by user preference. Default = traditional shadcn sidebar; floating = the standout variant for marketing screens / dashboards that want breathing room.
- **Header**: search (cmdk command menu), theme/palette switcher, user dropdown, breadcrumbs.
- **Layout toggle**: from `test-ui-3/components/layout-toggle.tsx` — lets the demo flip shell styles to make the "mix" tangible during review.
- **Boot intro**: subtle motion.dev splash on first load only (sessionStorage gated), inspired by `boot-intro.tsx`. Disable-able via theme switcher → "reduced motion" honoring `prefers-reduced-motion`.

## Pages (minimum to validate the design)

1. **/** — Marketing-style landing inside the app shell (glassmorphism hero from test-ui-3, value props, "Enter dashboard" CTA). Proves the "modern website" half of the brief.
2. **/dashboard** — IoT overview: metric cards, radial progress (device health %), performance chart (time-series), activity feed, feature tiles. **Includes `<ExtensionSlot id="dashboard.widgets"/>` at the bottom.** Proves the "shadcn dashboard" half + slot pattern.
3. **/devices** — Data table (from test-ui's `data-table/`) listing mock IoT devices, filters, row drawer. Proves the "real admin work" pattern.
4. **/settings/appearance** — Theme + palette + font + density switcher. **Plus `<ExtensionSlot id="settings.sections"/>`.** Proves the theming requirement is real, not aspirational — *and* propagates to extensions.
5. **/extensions** *(dev-only)* — Lists loaded extensions, their lifecycle state, and which slots they contribute to. Sourced from `useExtensionHost()`. Disabled in production builds. Cheap to add, huge for diagnosing the host shape.

No auth pages, no errors pages, no Clerk — those exist in test-ui already and aren't what's being validated here.

## Directory layout

```
test-ui-5/
├── SCOPE.md                      ← this file
├── package.json
├── vite.config.ts                ← includes @module-federation/vite host config
├── tsconfig.json
├── components.json               ← shadcn config
├── index.html
├── public/
│   └── extensions.json           ← demo manifest: which remotes to load
└── src/
    ├── main.tsx                  ← mounts ExtensionHostProvider before app
    ├── routes/                   ← TanStack Router file-based
    │   ├── __root.tsx
    │   ├── index.tsx             ← landing
    │   ├── dashboard.tsx         ← + <ExtensionSlot id="dashboard.widgets"/>
    │   ├── devices.tsx
    │   ├── settings.appearance.tsx  ← + <ExtensionSlot id="settings.sections"/>
    │   └── extensions.tsx        ← dev-only: useExtensionHost() viewer
    ├── components/
    │   ├── ui/                   ← shadcn primitives (copy-in)
    │   ├── layout/               ← sidebar, header, app-shell, slot wrappers
    │   ├── marketing/            ← glass hero, feature tiles
    │   ├── dashboard/            ← metric cards, charts, radial, activity
    │   ├── extensions/           ← <SlotFrame/>, empty-state, error-fallback
    │   └── theme/                ← theme + palette switcher
    ├── styles/
    │   ├── index.css             ← tailwind v4 entry + base layer
    │   └── theme.css             ← ALL design tokens (palettes live here)
    ├── lib/
    │   ├── utils.ts              ← cn() helper
    │   ├── motion.ts             ← motion.dev presets (fades, slides, stagger)
    │   └── extension-host.ts     ← bootstrap ExtensionHostManager + load remotes
    ├── stores/
    │   └── theme-store.ts        ← zustand: mode, palette, density, font
    └── mock/
        └── devices.ts            ← fake IoT data
```

## Reuse policy (the starter-repo bit)

This demo **must not** invent throwaway components. The flow:

1. Build a thing here as fast as possible to nail the look.
2. Once approved, lift the stable bits into:
   - `packages/starter-ui-core` → tokens, theme provider, palette mechanism, motion presets.
   - `packages/starter-ui-kit` → the visual flair components (glass hero, metric card variants, radial progress, floating sidebar).
   - `packages/starter-sdui-react` → SDUI-shaped wrappers when SDUI integration lands.
   - **`@nube/starter-ext-ui` is already a separate package** — no need to lift, just consume it.
3. test-ui-5 is then refactored to consume from `packages/*` instead of local files — proves the package APIs are real.

We will *not* update [packages/](../packages/) until the user has signed off on look-and-feel inside test-ui-5 (user's explicit instruction).

## Non-goals (v1 of test-ui-5)

- Real backend integration / API calls (mock data only).
- Auth / Clerk / sign-up flows.
- i18n strings (English literals; structure leaves room for `t()` later).
- SDUI runtime (comes after look-and-feel is signed off).
- Per-user unit preferences ([[i18n_and_unit_prefs]]) — wired later in `packages/`.
- AuthZ-gated pages ([[authz_scope]]) — out of scope for the design sandbox.

## Open questions for the user (please answer before I scaffold)

1. **Palettes**: Start with how many built-in palettes? My suggestion: 3 (`default`, `ocean`, `sunset`) + light/dark for each = enough to prove the switcher works without busywork.
2. **Floating sidebar**: default-on or behind the layout toggle? I'd default to *traditional* shadcn sidebar (familiar) and let the toggle reveal the floating variant — but happy to flip it.
3. **Boot intro**: keep it, or skip the splash and put the motion budget into page transitions instead? I lean *keep but make it ≤800ms and skippable*.
4. **Font**: stick with system-ui for v1, or pick a display font now (e.g. Geist, Inter Tight, Space Grotesk for the marketing hero)? Display font materially changes the "stand out" feel.
5. **pnpm workspace**: test-ui-5 **must** join [pnpm-workspace.yaml](../pnpm-workspace.yaml) immediately (so it can `workspace:*` import `@nube/starter-ext-ui`). I'll add the appropriate glob — flagging here so it's explicit.
6. **Which extension to load as the demo?** Default plan: load [starter-extensions/examples/hello-ui/](../starter-extensions/examples/hello-ui/) into the `sidebar` slot since it's already built for that. If you'd rather demo with [examples/notes/](../starter-extensions/examples/notes/) or skip the live mount until later, say so.
7. **`@module-federation/vite` vs Rspack**: I'm picking the Vite plugin to stay consistent with the rest of the repo. Flag if you want Rspack instead — would change the build tooling but not the host API.

## Success criteria

- One look at `/` and `/dashboard` and the user says "yes, that's the feel."
- Swapping the active palette is a single click in `/settings/appearance` and the entire app re-skins with zero flicker.
- Nothing in `src/components/` references a literal colour or font value — only tokens.
- The bits worth keeping have a clear, named home in `packages/*` (called out in this doc + in component file headers).
- **`hello-ui` (or chosen demo extension) mounts into the `sidebar` slot without changes to the extension.** Singleton negotiation passes. Theme tokens flow through. `/extensions` shows it as Loaded.
