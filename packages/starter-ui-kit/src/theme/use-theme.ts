// `useTheme()` — read + set the current theme from any component.

import { useContext } from "react";

import { ThemeContext } from "./theme-provider.js";
import type { ThemeContextValue } from "./types.js";

export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) {
    throw new Error("useTheme must be used inside <ThemeProvider>");
  }
  return ctx;
}
