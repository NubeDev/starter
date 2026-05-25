// Density tokens — spacing scale and control sizes.
//
// `globals.css` does not declare a custom spacing scale today; the kit
// inherits Tailwind v4's default `--spacing` step (0.25rem). The
// values below mirror that default and expose them as plain data so
// the RN kit can build a matching `useSpacing()` hook without needing
// to parse CSS.
//
// If the web kit ever introduces a custom `--spacing-*` ramp, add
// values here first, then let `generate-css.ts` emit them.

export const SPACING_BASE_REM = 0.25;

/** Multipliers applied to `SPACING_BASE_REM` to derive the canonical
 * Tailwind-aligned spacing scale (`0`, `0.5`, `1`, …, `96`). Only the
 * steps the platform actually uses are listed; extend as needed. */
export const SPACING_SCALE: Readonly<Record<string, number>> = {
  "0": 0,
  "0.5": 0.5,
  "1": 1,
  "1.5": 1.5,
  "2": 2,
  "3": 3,
  "4": 4,
  "5": 5,
  "6": 6,
  "8": 8,
  "10": 10,
  "12": 12,
  "16": 16,
  "20": 20,
  "24": 24,
};

/** Standard control heights (rem) used by `Button`, `Input`, etc.
 * Mirrors the `size` variants in `starter-ui-kit/src/components/ui/button.tsx`. */
export const CONTROL_HEIGHT_REM = {
  sm: 2,
  md: 2.25,
  lg: 2.5,
} as const;

export type DensitySize = keyof typeof CONTROL_HEIGHT_REM;
