# DESIGN.md — Nube iO

> The design system for every Nube iO digital product: dashboard, Rubix app, and what comes next.
> This file is the source of truth. Drop it in the repo root. Any AI coding agent reads it to build matching UI.
>
> **Implementation lives in:** `src/styles/primitives.css`, `src/styles/tokens.css`, `src/styles/theme.css` (React/Vite) and `lib/core/theme/nube_colors.dart`, `lib/core/theme/app_theme.dart` (Flutter twin).
> This document describes what those files implement. If they ever disagree, the CSS/Dart wins and this file should be updated to match.

---

## 1. Visual Theme & Atmosphere

**Mood:** Calm, precise, editorial. An operating layer for the physical world — confident but never loud. The interface gets out of the way so the data leads.

**Philosophy:**
- **Dark-first.** The product is designed in dark mode first; light is a fully-supported second theme, not an afterthought.
- **Editorial type pairing.** A clean geometric sans (Geist) carries the work; an italic serif (Instrument Serif) accents key phrases in display headings. This single pairing is the signature of the brand's product surfaces.
- **Ambient depth, not heavy chrome.** Soft radial glows, frosted glass, and 1px gradient hairlines create depth. No hard drop shadows in dark mode, no skeuomorphism.
- **Teal does the work. Yellow whispers.** One brand colour (teal) carries the identity. Yellow appears only as a small callout accent — never a fill or decoration.
- **Numbers are the hero.** On data surfaces, the metric is the largest thing; labels and context are quiet.

**Density:** Comfortable on the web dashboard, tighter on mobile. Spacing and radii are scaled by runtime variables (`--density-scale`, `--radius-scale`).

---

## 2. Color Palette & Roles

Colours are stored as `H S% L%` triplets in `primitives.css`, aliased to semantic roles in `tokens.css`, and bridged into Tailwind v4 `@theme` as `--tk-*` runtime vars in `theme.css`.

Mode and palette switch via attributes on `<html>`: `data-mode="light|dark"` and `data-palette="nube"`. Alternate palettes `ocean` and `sunset` exist with the same token names.

### Primitive ramps

| Token | HSL | Hex |
|---|---|---|
| `--teal-50` | `188 25% 96%` | `#F2F7F7` |
| `--teal-100` | `188 28% 88%` | `#D8E7E9` |
| `--teal-200` | `188 30% 77%` | `#B3D1D6` |
| `--teal-300` | `188 31% 63%` | `#83B6BE` |
| `--teal-400` | `188 32% 53%` | `#61A3AE` |
| **`--teal-500`** ⭐ brand | `187 41% 45%` | `#4497A2` |
| `--teal-600` | `187 44% 38%` | `#36828C` |
| `--teal-700` | `187 43% 31%` | `#2D6971` |
| `--teal-800` | `187 41% 27%` | `#295A61` |
| `--teal-900` | `187 40% 23%` | `#234D52` |
| `--teal-950` | `188 43% 14%` | `#142F33` |
| **`--yellow-400`** ⭐ callout | `40 96% 62%` | `#FBBD41` |
| `--grey-50 … 950` | `0 0% 98%` → `0 0% 4%` | `#FAFAFA` → `#0A0A0A` |
| `--green-500` | `142 71% 45%` | `#21C45D` |
| `--amber-500` | `38 92% 50%` | `#F59F0A` |
| `--red-500` | `0 84% 60%` | `#EF4343` |
| `--blue-500` | `217 91% 60%` | `#3C83F6` |

### Semantic roles (`data-palette="nube"`)

| Token | Light | Dark |
|---|---|---|
| `--background` | `#FAFAFA` (grey-50) | `#0A0A0A` (grey-950) |
| `--foreground` | `#262626` (grey-800) | `#F5F5F5` |
| `--card` | `#FFFFFF` | `#171717` (grey-900) |
| `--card-foreground` | `#262626` | `#F5F5F5` |
| `--popover` / `-foreground` | `#FFFFFF` / `#262626` | `#171717` / `#F5F5F5` |
| `--primary` | `#4497A2` (teal-500) | `#61A3AE` (teal-400 — lifted) |
| `--primary-foreground` | `#FFFFFF` | `#0A0A0A` |
| `--secondary` / `--muted` | `#F5F5F5` (grey-100) | `#262626` (grey-800) |
| `--secondary-foreground` | `#262626` | `#F5F5F5` |
| `--muted-foreground` | `#737373` (grey-500) | `#A3A3A3` (grey-400) |
| `--accent` | `teal-50` | `teal-950` |
| `--accent-foreground` | `teal-700` | `teal-300` |
| `--callout` | `#FBBD41` (yellow-400) | `#FBBD41` |
| `--callout-foreground` | `#262626` | `#0A0A0A` |
| `--success` | `#21C45D` | `#21C45D` |
| `--warning` | `#F59F0A` | `#F59F0A` |
| `--destructive` | `#EF4343` | `#EF4343` |
| `--destructive-foreground` | `#FFFFFF` | `#FFFFFF` |
| `--info` | `#3C83F6` | `#3C83F6` |
| `--border` / `--input` | `#E6E6E6` (grey-200) | `#262626` (grey-800) |
| `--ring` (focus) | `#4497A2` | `#61A3AE` |

### `--tk-*` bridge (the names you actually use in Tailwind)

- `--color-bg`, `--color-surface`, `--color-surface-2` → background / card / secondary
- `--color-text`, `--color-muted`, `--color-subtle` → foreground / muted-foreground / grey-500|400
- `--color-leaf` = primary; `--color-leaf-2` = teal-300|600 per mode
- `--color-aqua`, `--color-mist`, `--color-sky` → cool teal-shifted secondaries (chart series, accents)
- `--color-sun` = callout (dark) / yellow-700 (light)
- `--color-border`, `--color-border-hi` = teal-800|200 per mode
- Status: `--color-ok #22c55e`, `--color-warn #fde68a`, `--color-danger #fb7185`

---

## 3. Typography Rules

> **Note on the font system across Nube iO contexts.** Product UI (dashboard, Rubix app) uses the stack below — Geist + Instrument Serif. Presentations stay Roboto; external marketing stays Lexend Bold + Poppins. The product stack is documented here because it's what ships in the apps.

Declared in the Tailwind v4 `@theme` block of `theme.css`:

```css
--font-sans:  'Geist', 'Inter', ui-sans-serif, system-ui, sans-serif;
--font-serif: 'Instrument Serif', ui-serif, Georgia, serif;
--font-mono:  'Geist Mono', 'JetBrains Mono', ui-monospace, monospace;
```

### The signature: italic-serif display accent

Key phrases inside display headings render in **Instrument Serif italic 400**, `letter-spacing: -0.02em`, via `.serif-italic`. This is the "physical world." / "glance." look.

```css
.serif-italic {
  font-family: var(--font-serif);
  font-style: italic;
  font-weight: 400;
  letter-spacing: -0.02em;
}
```

Flutter mirrors this with `accentItalicTextStyle()` → `GoogleFonts.instrumentSerif(fontStyle: italic, letterSpacing: -0.5)`.

### Type scale

| Role | Size (mobile / desktop) | Weight | Tracking / notes |
|---|---|---|---|
| Display hero | 48 / 72–88 | 500 (medium, **not** bold) | `-0.04em`, `leading-1.02` |
| Section H2 | 36 / 48 | 500 | `-0.03em`, `leading-1.05` |
| Card H3 | 24 | 500 | `-0.02em`, tight |
| Big stat number | 48 | 500 | tabular-nums (`.tabular`), `-0.04em` |
| Body | 16 / 18 | 400 | `leading-relaxed`, muted colour |
| Card body | 14 | 400 | `leading-relaxed`, muted |
| Eyebrow / section label | 11 | 600 | UPPERCASE, `tracking 0.22em`, teal or subtle |
| Stat caption | 11 | 500 | UPPERCASE, `tracking 0.18em`, subtle |
| Tile eyebrow | 10 | 500 | UPPERCASE, `tracking 0.2em`, subtle |

Body sets `font-feature-settings: 'cv11', 'ss01'` for Geist stylistic alternates.

**Display is medium (500), not bold.** This is deliberate — the lighter weight at large sizes is what reads as premium/editorial. Don't bump display headings to 700.

---

## 4. Component Stylings

Built on **shadcn/ui** (`style: new-york`, `baseColor: slate`, `cssVariables: true`, icons: **lucide**).

### Vendored shadcn primitives
`avatar`, `badge`, `button`, `card`, `collapsible`, `dropdown-menu`, `input`, `separator`, `sheet`, `sidebar`, `skeleton`, `tooltip`.

### Buttons
- **Primary:** `--primary` bg, `--primary-foreground` text, radius `--radius-md` (10px). Soft teal glow underneath on the main CTA (see §6).
- **Secondary:** `--secondary` bg, `--foreground` text.
- **Ghost:** transparent, `--accent` on hover.
- **Destructive:** `--destructive` bg, white text.
- Standard hover-lift + transition on the custom easing `cubic-bezier(0.22, 1, 0.36, 1)`.

### Cards — two surface treatments

**Glass** (`.glass`) — the signature container. Use for hero panels, floating tiles, anything over the ambient gradient.
- Dark: frosted — `backdrop-filter: blur(22px) saturate(150%)`, translucent surface tint at 62%, inset top highlight, deep soft drop shadow.
- Light: flat — solid surface, hairline border, soft low drop shadow (glass "downgrades" gracefully to a clean card in light mode).

**Solid** — default `card`. White (light) / grey-900 (dark), `--border`, subtle shadow. Use for dense data: tables, lists, body content. **Glass competes with data — keep glass on heroes, solid on the body.**

### FeatureTile (the accent-ring tile)
`.glass` + `rounded-3xl` (32px) + a 44×44 `rounded-2xl` icon square with a `ring-1` accent tint:
- `leaf` → `ring-leaf/30 text-leaf bg-leaf/10` (teal)
- `aqua` → aqua variant
- `sun` → yellow callout variant (sparingly)

Eyebrow + title (24, medium) + body (14, muted) + arrow CTA. Framer Motion entrance + hover-lift.

### Metric / stat cards
Big tabular number (48, medium, `-0.04em`) + UPPERCASE caption (11) + a soft corner halo behind the number (see §6) + an inline sparkline. Optional trend chip (`↗ 12%` success / `↘ 5%` destructive).

### Navigation
- **Web:** left sidebar OR floating pill nav (backdrop-blur, fully rounded, sits above content). Active item = `--accent` bg + `--accent-foreground` text.
- **Mobile (Rubix):** bottom tab bar — see §8.

### Charts
- Web: **recharts** is installed but the shipped dashboard visuals are pure CSS gradients + Framer Motion. Flutter uses **fl_chart** (`NubeMiniSparkline`, `NubeAreaChart`, `NubeDonut`).
- Single series → teal. Multi-series → teal, then `--color-aqua`, `--color-sky`, then status colours.
- `NubeDonut` uses a hairline track ring + teal arc.

---

## 5. Layout Principles

Spacing follows a 4px base, scaled by `--density-scale`:

```css
--pad-card:    calc(1.5rem * var(--density-scale, 1));  /* 24px */
--pad-card-sm: calc(1rem   * var(--density-scale, 1));  /* 16px */
--gap-stack:   calc(1rem   * var(--density-scale, 1));  /* 16px */
```

- Generous outer margins on web; edge-to-edge with 16–20px gutters on mobile.
- Content leads with an eyebrow (UPPERCASE 11) → display heading (with serif-italic accent) → supporting body.
- Hero sections get the ambient gradient + a glass panel. Everything below is solid cards on the plain background.
- Whitespace is a feature. Don't fill every region; let sections breathe.

---

## 6. Depth & Elevation

### Border radii (scaled by `--radius-scale`)

| Token | Value |
|---|---|
| `--radius-sm` | 6px |
| `--radius-md` | 10px (default control) |
| `--radius-lg` | 16px |
| `--radius-xl` | 20px |
| `--radius-2xl` | 24px |
| `--radius-3xl` | 32px (FeatureTile) |
| bespoke CTA | 40px (`rounded-[2.5rem]`) |

### Glass card
See §4. Dark = frosted blur + inset highlight + deep drop; light = flat soft surface.

```css
.glass {
  background:
    linear-gradient(180deg, rgba(var(--tk-glass-highlight), 0.04), transparent 60%),
    rgba(var(--tk-glass-tint), 0.62);
  backdrop-filter: blur(22px) saturate(150%);
  border: 1px solid color-mix(in srgb, var(--color-muted) 12%, transparent);
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.04), 0 30px 80px -40px rgba(0,0,0,0.7);
}
:root[data-mode='light'] .glass {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  box-shadow: 0 1px 2px rgba(0,0,0,0.04), 0 4px 12px -4px rgba(0,0,0,0.08);
}
```

### Ambient page gradient
`body` carries three fixed, stacked radial gradients (top-right leaf, bottom-left aqua, centre border-hi). This is the soft brand glow behind everything.

### Soft corner halos
A 128×128 `rounded-full`, `blur-2xl` (40px) circle, offset partly off-canvas behind a stat, filled `color-mix(in oklab, <accent> 50%, transparent)`. CTA panels stack two larger radial gradients + one animated "breathing" halo.

### Hairline gradient sweep
`.hairline::before` paints a 1px top strip sweeping transparent → muted → leaf → aqua → transparent. Use to top off panels.

### Motion
- Standard easing: `cubic-bezier(0.22, 1, 0.36, 1)`.
- `breathe` 6s (halos), `marquee` 40s (tickers).
- Both `[data-motion='reduced']` and `prefers-reduced-motion` flatten all durations to ~0. **Always honour reduced motion.**

---

## 7. Do's and Don'ts

**Do**
- Use teal as the one brand colour. Reference `--primary` / `--color-leaf`, never raw hex.
- Use the serif-italic accent on one key phrase per display heading.
- Keep display headings at weight 500 (medium).
- Put glass on heroes/floating panels; solid cards on data.
- Use the ambient gradient + halos for depth instead of hard shadows.
- Ship light + dark from day one (tokens already cover both).
- Honour reduced-motion.

**Don't**
- Don't use yellow as a fill, background, or chart series. Callout accents only.
- Don't bold the display type to 700 — it kills the editorial feel.
- Don't put glass on dense data surfaces (tables, long lists).
- Don't introduce new fonts in product UI — Geist + Instrument Serif + Geist Mono only.
- Don't hard-code hex outside `primitives.css`.
- Don't reach for the alternate `ocean`/`sunset` palettes unless explicitly themed.

---

## 8. Responsive Behavior — incl. Mobile (Rubix app)

The Rubix Flutter app is the same design system adapted to a phone. The aesthetic is identical; the layout collapses.

| Web dashboard | Mobile (Rubix) |
|---|---|
| Left sidebar / floating pill nav | **Bottom tab bar** (Home, Dashboards, Connections, Settings) |
| Multi-column grids | **Single column**, stacked cards, 16px gutters |
| Hover-lift interactions | Tap states + press feedback; no hover |
| 72–88px display hero | 32–40px display hero, serif-italic accent kept |
| Glass hero panel | Glass header/hero kept; body cards solid |
| Dense tables | Card lists / rows; avoid horizontal scroll |
| Corner halos at 128px | Scale halos down (~80–96px) so they don't dominate |

**Mobile rules**
- Minimum touch target 44×44.
- Keep the ambient gradient on scaffold background; keep one glass hero per screen max.
- Status, empty, and error states are first-class — theme them (see Agent Prompt Guide). "Agent offline" and "Could not load" should look intentional, not raw.
- Bottom nav active item: teal icon + teal label + a short teal top-rule on the active tab.

Breakpoints (web): mobile < 640, sm 640, lg 1024. Mobile-first; collapse multi-column to single below 640.

---

## 9. Agent Prompt Guide

### Quick reference
```
BRAND        Teal #4497A2 (--primary)  ·  Yellow #FBBD41 (--callout, accents only)
FONTS        Geist (sans) · Instrument Serif italic (display accent) · Geist Mono
DISPLAY      weight 500, tracking -0.04em, serif-italic on one phrase
THEME        dark-first, light supported · data-mode + data-palette on <html>
SURFACES     .glass on heroes/floating · solid cards on data
DEPTH        ambient radial gradient + soft corner halos + 1px hairline (no hard shadows in dark)
RADII        control 10px · card 16–24px · tile 32px · CTA 40px
MOTION       ease cubic-bezier(0.22,1,0.36,1) · honour reduced-motion
ICONS        lucide
STACK        Vite + React 19 + TanStack Router + Tailwind v4 + shadcn (new-york) · Flutter twin via fl_chart
```

### Ready-to-use prompts

> "Build a [screen] for the Nube iO app. Dark-first. Geist sans with an Instrument Serif italic accent on one phrase of the heading (weight 500, tracking -0.04em). Teal `--primary` as the only brand colour; yellow only as a small callout. One `.glass` hero panel over the ambient radial-gradient background; solid cards below for data. Soft teal corner halos behind hero stats. lucide icons. Honour reduced-motion."

> "Theme this mobile screen: bottom tab bar (teal active state), single-column stacked cards, 16px gutters, 44px touch targets, glass header + solid body cards, 32–40px display heading with serif-italic accent."

> "Style the empty/error state on-brand: muted icon in a soft rounded square, quiet heading, one clear action. No raw exception text in the UI."

---

*v2.0 · May 2026 · Owner: Lina · Reconciled from implemented tokens (primitives/tokens/theme.css + Flutter twin) and the Nube iO brand system. Supersedes NUBE_IO_DESIGN_SYSTEM_v1.md.*
