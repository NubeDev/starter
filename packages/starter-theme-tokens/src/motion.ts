// Motion tokens — duration + easing scales.
//
// The web kit relies on Tailwind v4's animation utilities + tw-animate-css
// and does not currently declare a custom motion token set. The values
// below are conservative defaults pulled from Material/Apple HIG
// guidance and surfaced so the RN kit (`moti`) has a shared scale
// from day one. Adopt them on web by referencing these constants from
// any future `transition-*` token.

export const DURATION_MS = {
  instant: 0,
  fast: 120,
  normal: 200,
  slow: 320,
  slower: 480,
} as const;

/** CSS `cubic-bezier(…)` values — re-expressed as four-number tuples
 * so RN consumers can feed them to Reanimated/`Easing.bezier`. */
export const EASING = {
  linear: [0, 0, 1, 1] as const,
  standard: [0.2, 0, 0, 1] as const,
  emphasized: [0.3, 0, 0, 1] as const,
  decelerate: [0, 0, 0, 1] as const,
  accelerate: [0.3, 0, 1, 1] as const,
};

export type DurationKey = keyof typeof DURATION_MS;
export type EasingKey = keyof typeof EASING;
