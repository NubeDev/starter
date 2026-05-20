// # @nube/starter-ui-kit
//
// shadcn/ui primitives (radix-luma style) + Tailwind v4 design tokens +
// theme switch. R6: zero I/O. No API calls, no stores, no fetches.
//
// Consumers must import the stylesheet once at app entry:
//
//     import "@nube/starter-ui-kit/styles.css";
//
// Layout:
//
// - `components/ui/` — shadcn primitives, one file per primitive.
//   Direct imports work too: `@nube/starter-ui-kit/components/button`.
// - `theme/` — `<ThemeProvider>` + `useTheme()` with light/dark/system.
// - `lib/utils` — `cn()` helper (clsx + tailwind-merge).
// - `hooks/` — visual-only hooks (viewport, focus-trap).

export { cn } from "./lib/utils.js";

export * from "./theme/index.js";
export * from "./hooks/index.js";
export * from "./theme-editor/index.js";

export * from "./components/ui/alert.js";
export * from "./components/ui/alert-dialog.js";
export * from "./components/ui/badge.js";
export * from "./components/ui/breadcrumb.js";
export * from "./components/ui/button.js";
export * from "./components/ui/button-group.js";
export * from "./components/ui/card.js";
export * from "./components/ui/checkbox.js";
export * from "./components/ui/collapsible.js";
export * from "./components/ui/command.js";
export * from "./components/ui/context-menu.js";
export * from "./components/ui/dialog.js";
export * from "./components/ui/dropdown-menu.js";
export * from "./components/ui/empty.js";
export * from "./components/ui/hover-card.js";
export * from "./components/ui/input.js";
export * from "./components/ui/input-group.js";
export * from "./components/ui/item.js";
export * from "./components/ui/kbd.js";
export * from "./components/ui/label.js";
export * from "./components/ui/menubar.js";
export * from "./components/ui/popover.js";
export * from "./components/ui/progress.js";
export * from "./components/ui/radio-group.js";
export * from "./components/ui/resizable.js";
export * from "./components/ui/scroll-area.js";
export * from "./components/ui/select.js";
export * from "./components/ui/separator.js";
export * from "./components/ui/sheet.js";
export * from "./components/ui/sidebar.js";
export * from "./components/ui/skeleton.js";
export * from "./components/ui/slider.js";
export * from "./components/ui/spinner.js";
export * from "./components/ui/switch.js";
export * from "./components/ui/tabs.js";
export * from "./components/ui/textarea.js";
export * from "./components/ui/toggle.js";
export * from "./components/ui/toggle-group.js";
export * from "./components/ui/tooltip.js";
