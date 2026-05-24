// Public surface of the ConfigDrawer suite. Graduated from
// test-ui-5 — see GRADUATION-PLAN.md in that directory for context.
//
// The drawer is intentionally state-agnostic: consumers wire it to
// `useLayoutPreferences` (from `@nube/starter-ui-core/theme-editor`)
// or to their own facade. Every label / aria string flows in via
// props so the host can i18n-localize it.

export {
  SectionTitle,
  RadioIconTile,
  RadioTile,
  ComingSoonField,
} from "./shared.js";
export type {
  SectionTitleProps,
  RadioIconTileProps,
  RadioTileProps,
  ComingSoonFieldProps,
} from "./shared.js";

export {
  ThemeSection,
  PaletteSection,
  FontSizeSection,
  TileChoiceSection,
} from "./sections.js";
export type {
  SectionI18n,
  RadioOption,
  ThemeIconItem,
  ThemeSectionProps,
  PaletteItem,
  PaletteSectionProps,
  FontSizeItem,
  FontSizeSectionProps,
  TileChoiceItem,
  TileChoiceSectionProps,
} from "./sections.js";

export { ConfigDrawer } from "./drawer.js";
export type { ConfigDrawerProps, ConfigDrawerTab } from "./drawer.js";
