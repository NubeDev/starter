// Typography tokens — font stacks, sizes, weights, line heights.
//
// The web `@theme inline` block in `globals.css` references
// `--font-sans` and aliases `--font-heading` to it. The kit also ships
// `@fontsource-variable/inter` and prefers it; the literal stack below
// matches what `globals.css` emits today.

export const FONT_SANS_STACK =
  "'Inter Variable', ui-sans-serif, system-ui, -apple-system,\n" +
  "                 BlinkMacSystemFont, \"SF Pro Text\", \"Segoe UI\", Roboto,\n" +
  "                 \"Helvetica Neue\", Arial, sans-serif";

/** Generic font stacks used by the theme editor's defaults. The web
 * `--font-sans` value (above) takes precedence when the Inter
 * webfont is loaded. */
export const FONT_STACKS = {
  sans: "ui-sans-serif, system-ui, sans-serif",
  serif: "ui-serif, Georgia, serif",
  mono: "ui-monospace, SFMono-Regular, Menlo, monospace",
} as const;

/** Type ramp in rem. Tailwind v4 defaults; surfaced here so RN can
 * mirror without depending on Tailwind. */
export const FONT_SIZE_REM = {
  xs: 0.75,
  sm: 0.875,
  base: 1,
  lg: 1.125,
  xl: 1.25,
  "2xl": 1.5,
  "3xl": 1.875,
  "4xl": 2.25,
} as const;

export const FONT_WEIGHT = {
  regular: 400,
  medium: 500,
  semibold: 600,
  bold: 700,
} as const;

export const LINE_HEIGHT = {
  tight: 1.25,
  normal: 1.5,
  relaxed: 1.75,
} as const;

export const LETTER_SPACING_EM = "0em";

export type FontSize = keyof typeof FONT_SIZE_REM;
export type FontWeight = keyof typeof FONT_WEIGHT;
