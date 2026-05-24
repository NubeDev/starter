// App-shell layout primitives shared across starter apps. Imported via
// `@nube/starter-ui-core/layout`.
//
// - `LayoutProvider` / `useLayout` — header-vs-sidebar shell, sidebar
//   variant + collapsibility. Cookie-persisted.
// - `DirectionProvider` / `useDirection` — `ltr`/`rtl` writer that mirrors
//   onto `<html dir>` and into Radix's direction context. Cookie-persisted.
// - `useIsMobile` — SSR-safe `(max-width: 767px)` matcher.

export {
  LayoutProvider,
  useLayout,
} from "./layout-provider.js";
export type {
  Collapsible,
  LayoutContextValue,
  LayoutMode,
  LayoutProviderProps,
  Variant,
} from "./layout-provider.js";

export {
  DirectionProvider,
  useDirection,
} from "./direction-provider.js";
export type {
  Direction,
  DirectionContextValue,
  DirectionProviderProps,
} from "./direction-provider.js";

export { useIsMobile } from "./use-mobile.js";
