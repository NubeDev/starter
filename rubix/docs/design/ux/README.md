# UX, Design & Theming — Frontend (React) and Flutter

This document is the **single source of truth for visual identity, theming
architecture, and component conventions** across the two Rubix clients
that ship in this repo:

- `rubix/frontend/` — React + Vite + Tailwind + shadcn/ui (the web console).
- `rubix/flutter/` — Flutter app for iOS, Android, and web.

> **The React frontend is the reference implementation.** It is more
> mature, has a fully fleshed-out token system, and has been through more
> design iteration. The Flutter app **mirrors the React tokens 1:1** and
> the React design choices should be used to guide unresolved Flutter
> design questions.
>
> **Out of scope:** the React Native app under `rubix/mobile/` and the
> RN scope under `rubix/docs/scope/mobile/` are **legacy and not
> maintained**. Do not use them as a reference. Flutter has replaced
> React Native as the mobile target — see
> [`../flutter/README.md`](../flutter/README.md) for the rationale.

---

## 1. Brand foundation

Both apps render the **Nube iO brand** (parent brand of Rubix). The brand
is intentionally restrained: one teal, one yellow callout, a pure-grey
neutral ramp, and four status colours. Everything else is composition.

| Token            | Hex        | Role                                      |
|------------------|------------|-------------------------------------------|
| Teal-500 (brand) | `#4497A2`  | Primary, focus ring, selected, links.     |
| Yellow-400       | `#FBBD41`  | Callout only — underlines, small badges.  |
| Grey ramp        | `#FAFAFA → #0A0A0A` | Backgrounds, surfaces, text, borders. |
| Green-500        | `#21C45D`  | Success status only.                       |
| Amber-500        | `#F59F0A`  | Warning status only.                       |
| Red-500          | `#EF4343`  | Error/destructive only.                   |
| Blue-500         | `#3C83F6`  | Info status only.                          |

Hard rules — these are visible across both clients and reviewers should
reject violations:

- **Yellow is never a primary action colour.** It appears as a callout
  underline or a small badge — nothing else. Teal owns CTAs.
- **Status colours are reserved for status.** Green/amber/red/blue are
  not used for decoration, illustration, or chart series.
- **Neutrals are pure grey** (zero saturation). No warm/cool tints.
- **Brand voice is "calm, technical, professional."** No emoji in
  product UI. Sparing motion. Icons are clean lines (Lucide on web,
  `lucide_icons` on Flutter — same library, same proportions).

Brand reference material (taglines, illustration style, imagery rules)
lives in the user-memory brand guide and is summarised in the React
design-system doc — both apps must stay consistent with it.

---

## 2. Three-layer token architecture

Both apps use the **same three-layer token model**. The names and ramps
are identical; only the syntax differs between CSS and Dart.

```
┌───────────────────────────────────────────────┐
│ 3. Component layer                            │
│    Buttons, cards, badges, list rows.         │
│    Reference SEMANTIC tokens only.            │
├───────────────────────────────────────────────┤
│ 2. Semantic tokens (theme.ts / NubeTokens)    │
│    background, foreground, primary, accent,   │
│    success, warning, destructive, border, …   │
│    Light/dark variants live here.             │
├───────────────────────────────────────────────┤
│ 1. Primitives (raw ramps)                     │
│    teal-50…950, yellow-50…900, grey-50…950,   │
│    green-500, amber-500, red-500, blue-500.   │
│    NEVER referenced from feature code.        │
└───────────────────────────────────────────────┘
```

**Rule:** feature code only ever touches the **semantic** layer.
Reaching into primitives directly (e.g. `bg-teal-500` in a feature file,
or `NubePrimitives.teal500` in a Flutter widget) is a review-fail.

### 2.1 React (web) implementation

Files under [`rubix/frontend/src/styles/`](../../../frontend/src/styles/):

| File              | Layer                                  |
|-------------------|----------------------------------------|
| `primitives.css`  | Raw ramps as `H S% L%` triples.        |
| `tokens.css`      | Semantic tokens, gated on `[data-mode][data-palette]`. |
| `theme.css`       | Tailwind `@theme` block + legacy `--tk-*` bridge for older consumers. |
| `puck-theme.css`  | SDUI/Puck-specific overrides (kept isolated). |

The bridge layer is historical: the codebase predates the shadcn naming
convention and uses `--tk-*` variables in ~200 places. New code should
use the shadcn semantic names (`hsl(var(--primary))`, `bg-card`,
`text-foreground`, …) and let the bridge map them.

Tailwind v4 is configured to scan source packages explicitly (see the
`@source` directives in `theme.css`) so utility classes shipped by
sibling packages like `@nube/starter-ui-kit` and
`@nube/starter-ui-warehouse-explorer` are picked up.

Mode + palette are applied as attributes on `<html>`:

```html
<html data-mode="dark" data-palette="nube" data-motion="reduced">
```

This makes mode/palette switching a runtime toggle (no rebuild), and
the `data-motion="reduced"` selector wires up the prefers-reduced-motion
override in one place.

### 2.2 Flutter implementation

Files under [`rubix/flutter/lib/core/theme/`](../../../flutter/lib/core/theme/):

| File                  | Layer                                  |
|-----------------------|----------------------------------------|
| `nube_colors.dart`    | `NubePrimitives` (raw ramps) + `NubeTokens` (semantic). |
| `app_theme.dart`      | `ThemeData` builders for light and dark, plus `Material 3` `ColorScheme` mapping and per-widget themes. |
| `theme_providers.dart`| Riverpod providers for mode switching. |

`NubeTokens` is a `ThemeExtension<NubeTokens>` so feature widgets reach
semantic tokens via `Theme.of(context).nube.leaf`, never
`NubePrimitives.teal500`. The same naming as the React `--tk-*` layer
is used (`bg`, `bg2`, `surface`, `surface2`, `border`, `text`, `muted`,
`subtle`, `leaf`, `leaf2`, `callout`, `success`, `warning`, `danger`,
`info`) so anyone moving between the two codebases recognises the API.

The hex values in `NubePrimitives` are the rendered output of the React
`H S% L%` triples. **When the React palette changes, the Dart constants
must be updated in the same PR.** This is enforced by code review — see
the "stay in sync" rule below.

---

## 3. De-Material: making Flutter look like the React app

Flutter's default Material 3 look (ripples, tonal elevation, rounded
pill chips, surface tinting) does **not** match the web app. The web app
is a flat, shadcn-style surface with hairline borders and no ripples.
Several deliberate "de-Material" levers are applied in
[`app_theme.dart`](../../../flutter/lib/core/theme/app_theme.dart) to
close the gap:

| Lever                                | Effect                                    |
|--------------------------------------|-------------------------------------------|
| `splashFactory: NoSplash.splashFactory` | Removes ink ripple from every button/tile. |
| `splashColor`, `highlightColor: transparent` | No water-drop on tap. |
| `hoverColor: surface2`               | Flat hover tint (web parity).             |
| `focusColor: leaf @ 12% alpha`       | Soft focus ring instead of Material's blue. |
| `applyElevationOverlayColor: false`  | Disables M3 tonal surface tinting.        |
| `surfaceTintColor: transparent` (per widget theme) | Cards/AppBars/Dialogs don't tint with elevation. |
| `shadowColor: transparent`           | Cards are bordered, not shadowed.         |
| `CardTheme { side: BorderSide(border), borderRadius: 12 }` | Hairline-bordered cards, web parity. |
| Page transitions: `FadeForwards` on Android/Linux/Windows, `Cupertino` on iOS/macOS | Replaces bouncy zoom. |

**When in doubt, ship the flatter option.** Material defaults pull the
app away from the brand; React parity pulls it toward.

---

## 4. Component conventions

### 4.1 React

- Built on **shadcn/ui** primitives (Radix + Tailwind). Components live
  in [`rubix/frontend/src/components/`](../../../frontend/src/components/).
- Higher-order primitives (toasts, sheets, command palette) come from
  `@nube/starter-ui-kit` — reusable across every starter-based
  frontend, so do not fork them in `rubix/frontend`.
- Icons: `lucide-react`. No mixing with other icon sets.
- Fonts: `Geist` (sans) → `Inter` fallback, `Instrument Serif`,
  `Geist Mono`. The Tailwind `@theme` `--font-*` tokens drive these.

### 4.2 Flutter

- **Don't use Material widgets directly in feature code** for buttons,
  badges, cards, or inputs. Use the Nube-flavoured wrappers in
  [`lib/shared/widgets/nube_widgets.dart`](../../../flutter/lib/shared/widgets/nube_widgets.dart):
  - `NubeButton` (variants: `primary`, `secondary`, `outline`, `ghost`,
    `destructive`; sizes: `sm`, `md`, `lg`) — mirrors shadcn's button.
  - More wrappers added as needed; one file per widget once the file
    grows past the 400-line ceiling
    (see [`../flutter/FILE-LAYOUT.md`](../flutter/FILE-LAYOUT.md)).
- Icons: `lucide_icons` (the Dart port). Same Lucide library as web —
  same stroke weight, same proportions.
- Font: **Inter** via `google_fonts`. The React app prefers `Geist`,
  but Geist is not on Google Fonts; Inter is the next fallback and is
  visually near-identical (geometric humanist sans, matching x-height).
  Brand-internal docs use Roboto, but Inter has been chosen for the app
  to match the web console more closely — see
  [`../flutter/DECISIONS.md`](../flutter/DECISIONS.md).

### 4.3 Layout primitives

Both apps converge on the same layout shape so users moving between
them recognise the chrome:

- A left navigation rail / sidebar (collapsible on narrow viewports).
- A thin top bar (search, user menu, connection indicator).
- A scrolling main pane with cards as the primary grouping unit.
- Cards: 12 px radius, hairline border, no shadow.
- Spacing scale: 4 / 8 / 12 / 16 / 24 / 32 px.

When designing a Flutter screen, look at the React equivalent first:

| Flutter feature              | React reference                                              |
|------------------------------|--------------------------------------------------------------|
| Home / dashboard             | [`frontend/src/routes/dashboard.tsx`](../../../frontend/src/routes/dashboard.tsx) |
| Login                        | [`frontend/src/routes/login.tsx`](../../../frontend/src/routes/login.tsx) |
| Connection list (multi-tenant) | The settings/account drawer in `frontend/src/components/` |
| Error/empty/loading states   | `frontend/src/components/ui/` + `starter-ui-kit` primitives  |
| Sidebar + nav-group          | [`frontend/src/components/layout/`](../../../frontend/src/components/layout/) |

If a screen exists on web, **port the visual language wholesale**:
same spacing, same hierarchy, same colour roles. Diverge only where a
platform constraint forces it (e.g. bottom navigation on phones).

### 4.4 Secondary visual reference — `holi-demo/rubix_app`

A second Flutter app lives outside this repo at
`/Users/linasilvera/code/holi-demo/rubix_app`. It is a **visual
reference only** — we mine it for *application shell*, *menu/nav*,
and *screen layouts*, but **never** import its logic, state, or
theme files. The authoritative implementation, tokens, routing,
data layer, and de-Material rules all stay in `rubix/flutter/`.

Use it to answer "what should this screen look/feel like?" when
neither the React app (§4.3) nor `uipro` (§7) gives a clear answer.

| What to take from `holi-demo/rubix_app`         | What to ignore                                  |
|-------------------------------------------------|-------------------------------------------------|
| Overall app shell (nav rail / bottom bar shape) | `lib/theme/` — use `rubix/flutter` tokens.      |
| Menu structure and grouping                     | `lib/main.dart` bootstrap, routing wiring.       |
| Screen layouts under `lib/screens/` (home, energy, security, thermostat, scan, add-device) | Any state mgmt, API calls, package choices. |
| Widget composition patterns in `lib/widgets/` (charts, dials, sheets, toasts, animations) | Direct copies of widget code without re-theming. |
| Iconography weight, spacing rhythm, motion feel | Material defaults it pulls in — apply §3 de-Material rules. |

**Porting rule:** when adapting a screen from `holi-demo/rubix_app`
into `rubix/flutter/`:

1. Re-implement against `NubeTokens` — no hex literals, no
   `NubePrimitives.*` in feature code.
2. Use `NubeButton` and the wrappers in
   [`lib/shared/widgets/nube_widgets.dart`](../../../flutter/lib/shared/widgets/nube_widgets.dart),
   not raw `ElevatedButton`/`FilledButton`.
3. Strip ripples and tonal surface tints (§3 levers already applied
   globally in `app_theme.dart` — don't re-introduce them per-screen).
4. Swap any non-Lucide icons for `lucide_icons` equivalents.
5. Record any non-obvious adaptation in
   [`../flutter/DECISIONS.md`](../flutter/DECISIONS.md).

Priority order when designing a Flutter screen:

1. **React app** (`rubix/frontend/`) — exact mirror if a screen exists.
2. **`holi-demo/rubix_app`** — visual reference for app shell, menu,
   and screens with no React equivalent.
3. **`uipro` skill** — net-new surfaces with no precedent in either.

---

## 5. Light, dark, and accessibility

- Both apps support light and dark. Dark is the default for Rubix
  internal tools; light is supported and must be visually polished.
- Dark mode is **not** light with inverted colours — the teal is lifted
  one ramp step (`teal-400` instead of `teal-500`) so it stays legible
  on dark backgrounds. Both `tokens.css` and `NubeTokens.dark` apply
  this lift.
- Minimum contrast: WCAG AA for body text, AAA where practical. The
  semantic-layer pairs (`primary` + `primary-foreground`, etc.) are
  pre-checked; respect them.
- Reduced-motion: React honours `data-motion='reduced'`. Flutter
  honours `MediaQuery.disableAnimations`. Long-running animations
  (>200 ms) must check the flag and collapse to an instant transition.
- Focus rings are always visible (teal at ~12% alpha on web; soft teal
  outline on Flutter via `focusColor`). Never remove focus outlines.

---

## 6. Staying in sync (the hard part)

The React tokens are the source of truth. The Flutter tokens are a
**mirror**. To prevent drift:

1. **Palette changes start in `primitives.css`.** A PR that bumps a
   teal stop must also update `NubePrimitives` in the same change.
2. **Semantic-token renames or additions** must be reflected in
   `NubeTokens` in the same PR. Both layers have matching names.
3. **New component variants on web** that the Flutter app will need
   (e.g. a new button variant, a new badge tone) get a matching entry
   in `nube_widgets.dart` — opened as an issue if not implemented in
   the same PR.
4. **Brand-guide changes** are recorded in the user-memory brand notes
   (`nube-branding.md` / `nube-io-brand.md`); both apps follow the
   guide, not local custom.

When a Flutter design question is unresolved (spacing, a hover state,
an empty-state illustration), the answer is always: **look at the
React app, mirror it, then write the decision down here or in
[`../flutter/DECISIONS.md`](../flutter/DECISIONS.md).**

---

## 7. Tooling — `uipro` (UI/UX Pro Max CLI)

`uipro-cli` is installed globally on the dev machine and is the
shortest path to a "what does good look like" answer when designing a
new screen or component. It installs the **UI/UX Pro Max** skill into
the project so the AI assistant in this repo gets domain-specific
guidance on palettes, font pairings, layouts, and component patterns
(shadcn/ui, Tailwind, Flutter, etc.).

### Install / update

```bash
# One-off global install (already done on this machine)
npm install -g uipro-cli

# Install the skill into a project (run from repo root)
uipro init -a copilot          # for VS Code + Copilot Chat
uipro init -a claude           # for Claude Code
uipro init -a all              # install for every supported assistant

# Update to latest skill version
uipro update
uipro versions                 # list available versions
```

Supported assistants: `claude`, `cursor`, `windsurf`, `antigravity`,
`copilot`, `roocode`, `kiro`, `codex`, `qoder`, `gemini`, `trae`,
`opencode`, `continue`, `codebuddy`, `all`.

### When to use it

Use `uipro` (or invoke the installed skill via the assistant) when:

- **Designing a new Flutter screen** and the React app has no direct
  equivalent to mirror. Ask the assistant to "design a [screen] using
  the UI/UX Pro Max skill, constrained to the Nube tokens in
  `nube_colors.dart`."
- **Picking a layout pattern** (bento grid, split-pane dashboard,
  card list with filters) — the skill has 67 styles and 25 chart
  patterns curated and won't suggest patterns that fight the Nube
  brand if you keep it pinned to the existing palette.
- **Reviewing or refactoring** an existing component for accessibility,
  spacing rhythm, or hover/focus state correctness.
- **Picking font pairings or chart palettes** — the skill ships 96
  palettes and 57 pairings; constrain the pick to teal + neutrals so
  the result stays on-brand.

### How to use it well in this repo

- **Tokens are non-negotiable.** Always tell the assistant the result
  must consume the semantic layer (`Theme.of(context).nube.*` on
  Flutter, `hsl(var(--primary))` / shadcn classes on React). Don't let
  it emit raw hex colours.
- **Mirror first, generate second.** If a React screen exists, port
  it; only use `uipro` for net-new surfaces. This keeps the two apps
  visually aligned per §6.
- **Reject Material flavours on Flutter.** The skill will sometimes
  suggest M3-tinted surfaces or pill chips — the "de-Material" rules
  in §3 override.
- **Capture decisions.** If `uipro` informs a non-obvious design
  choice (e.g. a new badge variant, a specific list density), record
  it in [`../flutter/DECISIONS.md`](../flutter/DECISIONS.md) or
  inline in this doc, so the next session doesn't re-derive it.

The skill itself lives at
`/Users/linasilvera/code/starter/.claude/skills/ui-ux-pro-max/SKILL.md`
once installed via `uipro init`; the AI assistant reads it on demand.

---

## 8. Related docs

- React frontend layering: [`../frontend/README.md`](../frontend/README.md)
- Flutter scope and chassis: [`../flutter/README.md`](../flutter/README.md)
- Flutter file/layering rules: [`../flutter/FILE-LAYOUT.md`](../flutter/FILE-LAYOUT.md)
- Flutter technical decisions: [`../flutter/DECISIONS.md`](../flutter/DECISIONS.md)
- SDUI surface (web): [`../sdui/`](../sdui/)
- React primitives + tokens (source): [`rubix/frontend/src/styles/`](../../../frontend/src/styles/)
- Flutter tokens + theme (source): [`rubix/flutter/lib/core/theme/`](../../../flutter/lib/core/theme/)
- Flutter shared widgets: [`rubix/flutter/lib/shared/widgets/`](../../../flutter/lib/shared/widgets/)
