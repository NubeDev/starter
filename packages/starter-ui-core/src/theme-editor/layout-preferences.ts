// User-facing layout preferences that live **alongside** ThemeDocument
// but don't belong inside the 38-token theme model.
//
// These are runtime presentation knobs (density, font scale, motion,
// resolved colour mode, palette enum) that consumers like test-ui-5
// want to expose in a "Settings → Appearance" surface. The theme model
// itself stays pure (tokens + ShellConfig + assets).
//
// Persistence is intentionally client-only — `localStorage` via
// `LayoutPreferenceTransport`. If a future consumer needs server-side
// sync, the transport interface is the swap point, just like
// `ThemeTransport`.

/** Resolved colour mode that the DOM actually renders. */
export type ResolvedMode = "light" | "dark";

/** User-facing preference. `"system"` defers to
 * `prefers-color-scheme`; `resolveMode()` collapses it to a
 * `ResolvedMode`. */
export type ModePreference = "light" | "dark" | "system";

/** Density scale. Drives a single CSS var (`--density-scale`) that
 * consumer stylesheets can multiply against paddings / gaps. */
export type Density = "compact" | "comfortable" | "spacious";

/** Base font-size scale. Drives `--font-size-scale` so the consumer's
 * root font-size can pick it up via `calc()`. */
export type FontSize = "sm" | "md" | "lg";

/** Motion preference. `"reduced"` mirrors `prefers-reduced-motion`. */
export type Motion = "full" | "reduced";

/** Optional consumer-defined palette identifier. ui-core does not own
 * the palette enumeration — it's an arbitrary string the consumer
 * matches in their own CSS via `[data-palette="..."]`. */
export type PaletteId = string;

/** Full layout-prefs shape. All fields are required to keep the type
 * narrow at consumption sites; use `defaultLayoutPreferences` to seed
 * partial state. */
export interface LayoutPreferences {
  mode: ModePreference;
  density: Density;
  fontSize: FontSize;
  motion: Motion;
  /** Optional palette key. Consumers that don't use the palette-enum
   * pattern can leave this `null`. */
  palette: PaletteId | null;
}

export const defaultLayoutPreferences: LayoutPreferences = {
  mode: "system",
  density: "comfortable",
  fontSize: "md",
  motion: "full",
  palette: null,
};

/** Numeric multipliers exposed to consumer CSS as `--density-scale`
 * and `--font-size-scale`. Chosen so `comfortable` / `md` are 1.0 and
 * the steps are perceptible without being jarring. */
export const DENSITY_SCALE: Record<Density, number> = {
  compact: 0.85,
  comfortable: 1,
  spacious: 1.15,
};

export const FONT_SIZE_SCALE: Record<FontSize, number> = {
  sm: 0.875, // ~14px at a 16px root
  md: 1,
  lg: 1.125, // ~18px
};

/** Resolve a `ModePreference` to a concrete `ResolvedMode` by reading
 * `prefers-color-scheme` when the preference is `"system"`.
 *
 * Server-safe: when `window` is undefined or `matchMedia` is missing,
 * `"system"` falls back to `"light"` so SSR doesn't crash. */
export function resolveMode(pref: ModePreference): ResolvedMode {
  if (pref !== "system") return pref;
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return "light";
  }
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

/** Subscribe to OS-level `prefers-color-scheme` changes. Returns an
 * unsubscribe function. The listener fires only when the user's
 * `ModePreference` is `"system"` — callers must filter themselves
 * since this helper is provider-agnostic. */
export function subscribePrefersDark(listener: (isDark: boolean) => void): () => void {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return () => {};
  }
  const mql = window.matchMedia("(prefers-color-scheme: dark)");
  const handler = (e: MediaQueryListEvent) => listener(e.matches);
  mql.addEventListener("change", handler);
  return () => mql.removeEventListener("change", handler);
}

/** Subscribe to OS-level `prefers-reduced-motion`. Same shape as
 * `subscribePrefersDark`. */
export function subscribePrefersReducedMotion(listener: (reduced: boolean) => void): () => void {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return () => {};
  }
  const mql = window.matchMedia("(prefers-reduced-motion: reduce)");
  const handler = (e: MediaQueryListEvent) => listener(e.matches);
  mql.addEventListener("change", handler);
  return () => mql.removeEventListener("change", handler);
}
