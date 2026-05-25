# @nube/starter-ui-dashboard-native

React Native ports of the four widgets in
[`packages/starter-ui-dashboard/`](../starter-ui-dashboard/) —
**identical prop APIs**, so a feature consumed on web can ship on
mobile by changing only the import:

```diff
- import { MetricCard } from "@nube/starter-ui-dashboard";
+ import { MetricCard } from "@nube/starter-ui-dashboard-native";
```

Widgets:

| widget              | file                       |
|---------------------|----------------------------|
| `MetricCard`        | `src/metric-card.tsx`      |
| `RadialProgress`    | `src/radial-progress.tsx`  |
| `ActivityFeed`      | `src/activity-feed.tsx`    |
| `PerformanceChart`  | `src/performance-chart.tsx`|

## Architecture

Same discipline as `@nube/starter-ui-sdui-native`: widget files
import only

- `@nube/starter-ui-kit-native` (styling seam — primitives, layout,
  `useTheme()`),
- `react-native-svg` (charts, rings, sparklines),
- `moti` (animation peer of the kit; sits on `react-native-reanimated`),
- `react`.

**No direct `react-native` primitive imports.** The kit owns the
`<View>` / `<Text>` / `<Pressable>` surface so the test harness can
swap a mock kit in, and so visual style stays consistent across
widgets without each widget reimplementing it.

## Mapping rules applied (per `rubix/docs/scope/mobile/NEW-PACKAGES.md`)

- `<div>` + Tailwind → kit-native `Card` / `Box` / `Row` / `Column`
  + theme tokens via `useTheme()`.
- Inline `<svg>` → `react-native-svg` (`Svg`, `Circle`, `Path`,
  `Line`, `Polyline`, `Defs`, `LinearGradient`, `Stop`).
- `motion/react` (`motion.div`, `AnimatePresence`) → `moti`
  (`MotiView`, `AnimatePresence`).
- `lucide-react`'s `LucideIcon` type is structurally compatible with
  `lucide-react-native`; consumers pass the appropriate icon
  component for their platform.

## Prop-API parity (MUST)

The exported `Props` type for each widget is the same name and shape
as the web export. Mobile-only props are forbidden — if mobile needs a
new prop, it lands on the web component first.
