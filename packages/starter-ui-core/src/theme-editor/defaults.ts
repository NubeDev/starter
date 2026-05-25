// Built-in defaults.
//
// Before stage 4 this file hard-coded a duplicate of the `:root` and
// `.dark` blocks in `@nube/starter-ui-kit/src/styles/globals.css`. The
// two copies were *required* to be kept in lockstep by hand.
//
// Stage 4 introduced `@nube/starter-theme-tokens` as the single
// source-of-truth: the kit's `globals.css` is generated from the same
// constants this file now re-exports, so the editor's "Reset to
// platform default" can never drift from the kit defaults again.

import {
  NON_COLOR_KEYS as TOKENS_NON_COLOR_KEYS,
  platformDarkPalette,
  platformLightPalette,
} from "@nube/starter-theme-tokens";

import type { ThemeStyleProps, ThemeStyles } from "./types.js";

export const defaultLightThemeStyles: ThemeStyleProps = platformLightPalette;
export const defaultDarkThemeStyles: ThemeStyleProps = platformDarkPalette;

export const defaultThemeStyles: ThemeStyles = {
  light: defaultLightThemeStyles,
  dark: defaultDarkThemeStyles,
};

/** Tokens that are not colours and must pass through the apply path
 * verbatim (no OKLCH conversion). Re-exported from the tokens package
 * so the kit, the editor, and the RN runtime all agree on the set. */
export const NON_COLOR_KEYS: ReadonlySet<string> = TOKENS_NON_COLOR_KEYS;
