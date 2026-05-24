# @nube/starter-ui-kit

shadcn/ui primitives + Tailwind v4 tokens + theme switch. Zero I/O — no
fetch, no auth, no React Query. Pair with `@nube/starter-ui-core` for the
glue and `@nube/starter-client-ts` for the wire.

## Icons

This package uses **[`lucide-react`](https://www.npmjs.com/package/lucide-react)** — the icon library that ships with shadcn/ui by default. One icon family means one consistent visual language across the kit.

Consumers needing a different set can swap icons at the application boundary by re-exporting components with their own preferred renderer.

## What's in the box

`src/components/ui/*.tsx` is the standard shadcn dump (~38 primitives:
button, dialog, input, table, …). They're inert until imported, so
keeping the full set has zero cost for consumers that import only a
subset.

`src/styles/globals.css` carries the Tailwind v4 token layer. Import it
once at your app entry:

```ts
import "@nube/starter-ui-kit/styles.css";
```

### Bring-your-own theme

If you want your own tokens but still need Tailwind to see the kit's
class usage, skip `styles.css` and import the scan shim instead:

```css
/* your app's main Tailwind stylesheet */
@import "tailwindcss";
@import "@nube/starter-ui-kit/scan-source.css";
@import "./your-own-tokens.css";
```

Tailwind v4 skips `node_modules` by default; the shim emits the right
`@source` directive so the kit's `bg-popover`, `data-[side=right]`,
`slide-in-from-right-*`, etc. all end up in your CSS bundle without
needing a brittle relative `@source` path.
