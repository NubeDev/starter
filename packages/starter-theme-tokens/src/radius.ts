// Border-radius scale.
//
// The platform default base is `0.625rem` (see
// `palette.platformLightPalette.radius`). The web kit's `@theme inline`
// block derives `--radius-sm/md/lg/xl/2xl/3xl/4xl` as multiples of
// `--radius`; the multipliers below are the source of truth for that
// derivation. `generate-css.ts` consumes `RADIUS_MULTIPLIERS` to emit
// the matching `calc(var(--radius) * N)` lines byte-identically.

export const RADIUS_BASE_REM = 0.625;

/** Multipliers applied to `--radius` to derive named radius steps.
 * Keys are emitted in this exact order into `globals.css`. */
export const RADIUS_MULTIPLIERS: Readonly<Record<string, number>> = {
  sm: 0.6,
  md: 0.8,
  lg: 1.0,
  xl: 1.4,
  "2xl": 1.8,
  "3xl": 2.2,
  "4xl": 2.6,
};

export type RadiusSize = keyof typeof RADIUS_MULTIPLIERS;
