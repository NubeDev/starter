# @nube/starter-theme-tokens

Pure-data source-of-truth for the starter platform's design tokens.

Consumed by **both** web (`@nube/starter-ui-kit`,
`@nube/starter-ui-core/theme-editor`) and mobile
(`@nube/starter-ui-kit-native`) so colour, density, and type scale are
identical by construction.

## Hard constraints

- **No** runtime dependencies (no `peerDependencies`).
- **No** React, React Native, DOM, or styling runtime.
- One file per concept (`palette`, `density`, `radius`, `type`,
  `motion`, `role`); `index.ts` is a barrel of re-exports only.
- All values are plain JS data so they round-trip through `JSON.stringify`.

## Files

| File          | Owns                                                             |
| ------------- | ---------------------------------------------------------------- |
| `palette.ts`  | Platform default light/dark token maps + named preset palettes.  |
| `density.ts`  | Spacing scale, control sizes.                                    |
| `radius.ts`   | Border radius scale + multipliers.                               |
| `type.ts`     | Font stacks, sizes, weights, line heights.                       |
| `motion.ts`   | Duration + easing scales.                                        |
| `role.ts`     | Semantic role → token key mapping.                               |

Web emission (`globals.css`) is generated from this package by
`packages/starter-ui-kit/scripts/generate-css.ts`; the script's output
is asserted byte-identical to the checked-in fixture so a token
edit ripples to CSS without drift.
