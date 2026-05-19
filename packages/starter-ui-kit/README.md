# @nube/starter-ui-kit

shadcn/ui primitives + Tailwind v4 tokens + theme switch. Zero I/O — no
fetch, no auth, no React Query. Pair with `@nube/starter-ui-core` for the
glue and `@nube/starter-client-ts` for the wire.

## Icons

This package depends on **HugeIcons**:

- [`@hugeicons/react`](https://www.npmjs.com/package/@hugeicons/react)
  — the renderer component.
- [`@hugeicons/core-free-icons`](https://www.npmjs.com/package/@hugeicons/core-free-icons)
  — the free icon set used throughout the kit.

A single icon family was chosen so the kit ships one consistent visual
language. Consumers needing a different set can either: (a) swap the
peer dep and re-import in their own copy of the components, or
(b) wrap the HugeIcon component behind a thin adapter at the
application boundary.

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
