// Theme-editor view layer for `@nube/starter-ui-kit`. Composes the
// data layer in `@nube/starter-ui-core/theme-editor` with shadcn
// primitives from this package. R6-compliant: the components themselves
// do no I/O — every network call is routed through a `ThemeTransport`
// supplied by the host app.
//
// Usage: drop `<ThemeEditorPage transport={...} />` into a route the
// consumer has gated by their own admin-role check. See the package
// README for the full integration recipe.

export { ThemeEditorPage } from "./theme-editor-page.js";
export type { ThemeEditorPageProps } from "./theme-editor-page.js";

export { ThemeGallery } from "./theme-gallery.js";
export { ColorTokenEditor } from "./color-token-editor.js";
export { BrandingEditor } from "./branding-editor.js";
export { LivePreview } from "./live-preview.js";
export { ImportCssDialog } from "./import-css-dialog.js";
export { ThemeActions } from "./theme-actions.js";

// Stage-2 graduation: ConfigDrawer (Sheet + tabs) + the section
// primitives it composes. State-agnostic — consumers wire each
// section to `useLayoutPreferences` (from
// `@nube/starter-ui-core/theme-editor`) or to their own facade.
export * from "./config-drawer/index.js";
