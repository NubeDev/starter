// Theme-editor barrel. Public surface only — keep new files out of the
// re-export list until they're meant for consumption.

export * from "./types.js";
export * from "./defaults.js";
export * from "./presets.js";
export * from "./store.js";
export * from "./transport.js";

// Stage 2 additions: layout preferences (density / motion / font-size /
// resolved mode / palette enum) live next to the theme model but are
// kept in their own store so the 38-token undo/redo stays narrow.
export * from "./layout-preferences.js";
export * from "./layout-preferences-store.js";

export * from "./utils/apply-theme.js";
export * from "./utils/apply-preferences.js";
export * from "./utils/color-converter.js";
export * from "./utils/contrast-checker.js";
export * from "./utils/parse-css-input.js";
export * from "./utils/generate-css.js";
export * from "./utils/tailwind-css.js";

export * from "./hooks/use-theme-editor.js";
export * from "./hooks/use-theme-presets.js";
