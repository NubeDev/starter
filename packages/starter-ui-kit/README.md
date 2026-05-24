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
