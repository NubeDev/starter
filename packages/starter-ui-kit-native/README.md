# @nube/starter-ui-kit-native

React Native primitives whose prop API mirrors
[`@nube/starter-ui-kit`](../starter-ui-kit/) one-to-one. Foundation:
React Native core + [`react-native-svg`](https://github.com/software-mansion/react-native-svg)
+ [`moti`](https://moti.fyi/) (which sits on
[`react-native-reanimated`](https://docs.swmansion.com/react-native-reanimated/)).

This is the **mobile half** of phase 3 in
[`rubix/docs/scope/mobile/NEW-PACKAGES.md`](../../rubix/docs/scope/mobile/NEW-PACKAGES.md).

## Surface

One file per primitive, mirroring the web kit's verb-per-file
discipline:

| File             | Web equivalent                          |
|------------------|------------------------------------------|
| `button.tsx`     | `starter-ui-kit/components/ui/button`    |
| `card.tsx`       | `…/card`                                 |
| `input.tsx`      | `…/input`                                |
| `tabs.tsx`       | `…/tabs`                                 |
| `badge.tsx`      | `…/badge`                                |
| `switch.tsx`     | `…/switch`                               |
| `slider.tsx`     | `…/slider`                               |
| `select.tsx`     | `…/select`                               |
| `sheet.tsx`      | `…/sheet`                                |
| `dialog.tsx`     | `…/dialog`                               |
| `spinner.tsx`    | `…/spinner`                              |
| `skeleton.tsx`   | `…/skeleton`                             |
| `tooltip.tsx`    | `…/tooltip`                              |

## Theming

Every primitive reads tokens via [`useTheme()`](./src/theme.ts).
`useTheme` is backed by:

- `@nube/starter-theme-tokens` — palette, density, radius, type,
  motion, role tokens. Single source of truth, shared with the
  web kit.
- `@nube/starter-ui-core/theme-editor` — the `useLayoutPreferences`
  zustand store (mode / density / fontSize / motion / palette).
  This is the **same store instance** the web theme editor writes
  to; a server-pushed preference change reaches both runtimes.

No `className`, no `StyleSheet.create()` outside the primitive
that uses it.

## Accessibility — kit acceptance criterion

Every primitive ships `accessibilityRole` + a resolvable
`accessibilityLabel` (or accepts one as a prop, for non-textual
controls like `Slider`/`Switch`/`Input`). This is not a polish
item: a reviewer is entitled to block a primitive PR that ships a
`Pressable` without `accessibilityRole="button"` or a `TextInput`
without an `accessibilityLabel` resolution path.

## Bounds

- MUST NOT import `@nube/starter-ui-kit`.
- MUST NOT do network I/O.
- MUST NOT own application state.
- A future swap to Tamagui / gluestack-ui would change the styling
  runtime model and the snapshot baseline; the scope plan commits
  to the RN-core path. That swap is an ADR, not a phase deviation.

## Example harness

See [`example/`](./example/) for a story-style harness rendering
each primitive in light, dark, and two named palettes.
