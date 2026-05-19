# Theming extensions

How an extension picks up the host's look, and when you need to read theme
tokens programmatically.

## TL;DR

- Use [`@nube/starter-ui-kit`] components and Tailwind utility classes; you
  will inherit the host's theme automatically through the CSS variable
  cascade. No extra wiring.
- Reach for `useHostTheme()` from [`@nube/starter-ext-sdk-ts`] only when you
  need theme values in JavaScript — chart palettes, canvas fills,
  CSS-in-JS, dynamic SVG attributes.
- Read the active colour mode (`"light"` / `"dark"` / custom) from the same
  hook to swap mode-specific assets (logos, illustrations).

## How host theming flows into extensions

The host (the application that mounts your extension via
`<ExtensionSlot/>`) owns the theme. It writes the resolved token map onto
`document.documentElement` as CSS custom properties — `--primary`,
`--background`, `--radius`, the full token vocabulary defined by
[`@nube/starter-ui-core/theme-editor`]. The browser's cascade does the rest:
anything inside your extension that resolves `var(--primary)` (directly, or
via a `bg-primary` Tailwind utility) gets the live host value with no
JavaScript involved.

In addition, the host *may* pass the same map down through the slot
context as `themeTokens`. When it does, `useHostTheme().token("primary")`
reads from that map directly — useful for tests and for hosts that
pre-resolve tokens before paint.

## Pattern 1 — CSS cascade (the default)

Build with [`@nube/starter-ui-kit`] primitives and/or Tailwind utility
classes. The kit's components reference the same CSS variables the host
sets, so colour/radius/spacing changes flow in for free.

```tsx
// my-extension/src/Panel.tsx
import { Button } from "@nube/starter-ui-kit/button";
import { Card, CardContent, CardHeader, CardTitle } from "@nube/starter-ui-kit/card";

export function Panel() {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Recent activity</CardTitle>
      </CardHeader>
      <CardContent>
        <p className="text-muted-foreground">Last sync 3 minutes ago.</p>
        <Button>Refresh</Button>
      </CardContent>
    </Card>
  );
}
```

When the host operator picks a new preset in the theme editor, the
`Button` and `Card` re-paint on the next frame — no extension re-render
required.

If you need a one-off colour and don't want to pull in a kit component:

```tsx
<div className="rounded-md border bg-card p-4 text-card-foreground">…</div>
```

Both `bg-card` and `text-card-foreground` resolve to `var(--card)` /
`var(--card-foreground)`.

## Pattern 2 — `useHostTheme()` for programmatic reads

Some surfaces can't ride the cascade: third-party chart libraries that
take colour strings as JS arrays, `<canvas>` calls, SVG attributes like
`stroke` that you compute, CSS-in-JS that wants the value at runtime. For
those, use `useHostTheme()`:

```tsx
// my-extension/src/ChartPanel.tsx
import { useHostTheme } from "@nube/starter-ext-sdk-ts";
import { LineChart } from "recharts";

export function ChartPanel({ series }: { series: number[][] }) {
  const theme = useHostTheme();
  const palette = [1, 2, 3, 4, 5].map((i) => theme.token(`chart-${i}`));
  return (
    <LineChart
      data={series}
      colors={palette}
      dark={theme.mode === "dark"}
    />
  );
}
```

`token(key)` resolution order:

1. Host-supplied `themeTokens` map (if the host wired
   `<ExtensionSlot themeTokens=… />`).
2. `getComputedStyle(document.documentElement).getPropertyValue("--<key>")`.
3. Empty string. The caller decides the default (`?? "#000"`).

Re-reading per render is intentionally cheap — `useHostTheme()`
memoises on `slot.theme` and `slot.themeTokens`, and the
`getComputedStyle` fallback only fires when the map doesn't have the key.

### Swapping mode-specific assets

If your extension ships a light-mode and a dark-mode logo:

```tsx
import { useHostTheme } from "@nube/starter-ext-sdk-ts";
import logoLight from "./assets/logo-light.svg";
import logoDark from "./assets/logo-dark.svg";

export function BrandStrip() {
  const { mode } = useHostTheme();
  return <img src={mode === "dark" ? logoDark : logoLight} alt="Brand" />;
}
```

Read `mode` instead of the `(prefers-color-scheme: dark)` media query —
the host may force a specific mode independent of OS preference, and
that override is exactly what `mode` reflects.

## Don't

- **Don't import from `@nube/starter-ui-core/theme-editor` in an
  extension.** That package is the host's editor brain. The dependency
  arrow in `SCOPE.md` is `starter-ext-sdk-ts` → kernel; reaching past it
  couples your extension to private host internals and will break when
  the host swaps editors.
- **Don't read your own theme out of `localStorage` or the network.**
  The host is the single source of truth. Treat the cascade and
  `useHostTheme()` as your only inputs.
- **Don't cache the token map yourself.** Hosts can hot-swap themes
  while your panel is mounted; `useHostTheme()` already invalidates on
  the slot context change.

## Testing

The kernel ships a test harness that lets you mount an extension with a
fixed theme:

```tsx
import { renderWithExtensionHost, ExtensionSlot } from "@nube/starter-ext-ui";

await renderWithExtensionHost(
  <ExtensionSlot
    id="sidebar"
    theme="dark"
    themeTokens={{ primary: "oklch(0.5 0.2 250)" }}
  />,
  { extensions: [{ id: "com.acme.my-ext", ui, factory }] },
);
```

Inside your panel, `useHostTheme().token("primary")` will return the
exact string you passed. See
[`use-host-theme.test.tsx`](../../starter-extensions/packages/starter-ext-ui/src/use-host-theme.test.tsx)
for the round-trip plus the cascade-fallback assertion.

[`@nube/starter-ui-kit`]: ../../packages/starter-ui-kit
[`@nube/starter-ui-core/theme-editor`]: ../../packages/starter-ui-core
[`@nube/starter-ext-sdk-ts`]: ../../starter-extensions/packages/starter-ext-sdk-ts
