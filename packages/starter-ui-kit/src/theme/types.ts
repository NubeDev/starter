// Public theme types. `light` and `dark` are explicit; `system`
// defers to the OS via `prefers-color-scheme`.

export type Theme = "light" | "dark" | "system";

export interface ThemeContextValue {
  /** Current theme as the user set it (may be `system`). */
  theme: Theme;
  /**
   * Resolved theme — `light` or `dark`. Use this when applying
   * actual styles; `theme` is the user's *preference*.
   */
  resolved: "light" | "dark";
  /** Change the theme. Persists to `localStorage`. */
  setTheme: (theme: Theme) => void;
}
