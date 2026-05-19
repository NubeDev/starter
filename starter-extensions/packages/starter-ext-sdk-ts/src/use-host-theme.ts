// `useHostTheme()` — the single read API extension authors use to
// consume the host's theme.
//
// Why a dedicated hook on top of `useSlotContext()`:
//
// - Most extensions only need the resolved token map (for a chart
//   palette, a canvas fill, an emit-CSS-in-JS path). The slot
//   context carries it, but reaching through `slot.themeTokens`
//   loses the fallback to live `getComputedStyle` when the host
//   didn't pre-resolve the map (older hosts, server-rendered seeds).
// - Mode-only consumers (logo swap, dark-mode-only asset) want a
//   compact `{ mode }` read without binding to the rest of the
//   slot context.
// - It is the supported extension point: future versions can layer
//   reactive subscriptions (subscribing to the host's theme editor
//   so previewing flows into mounted extensions live) without
//   changing the call shape.
//
// The SDK never imports from `@nube/starter-ui-core` — SCOPE.md's
// TS dep arrow says extension authors depend on
// `starter-ext-sdk-ts` + `starter-ui-kit` + `starter-client-ts`,
// never on the host's brain. The host is the only thing that wires
// `@nube/starter-ui-core/theme-editor` into its
// `ExtensionHostProvider`.

import * as React from "react";

import { useSlotContext } from "./slot-context.js";
import type { HostThemeMode, HostThemeTokens } from "./slot-context.js";

/** Shape returned by [`useHostTheme`]. */
export interface HostTheme {
  /** Active colour mode the host has selected. */
  mode: HostThemeMode;
  /**
   * Read one token value. Prefers the host-supplied map; falls back
   * to `getComputedStyle(document.documentElement)` for tokens not
   * in the map (so extensions running under an older host still see
   * the cascading CSS variables).
   *
   * Returns the empty string when neither source resolves — callers
   * decide whether to default themselves.
   */
  token(key: string): string;
  /**
   * Full token map if the host supplied one, otherwise `null`.
   * Useful when an extension wants to enumerate tokens (chart
   * palette cycling through `chart-1` … `chart-5`).
   */
  tokens: HostThemeTokens | null;
}

/**
 * Read the host theme inside an extension panel. Must be called
 * under `<SlotContextProvider>` (which `BlockShell` provides for
 * you).
 *
 * Example:
 *
 * ```tsx
 * function ChartPanel() {
 *   const theme = useHostTheme();
 *   const palette = [1, 2, 3, 4, 5].map((i) => theme.token(`chart-${i}`));
 *   return <Chart palette={palette} dark={theme.mode === "dark"} />;
 * }
 * ```
 */
export function useHostTheme(): HostTheme {
  const slot = useSlotContext();
  // `getComputedStyle` is fine to call per render in v0.1 — the host
  // mounts each extension behind `BlockShell`, which is itself
  // wrapped in an error boundary; the read is cheap and avoids
  // staleness vs caching. If profiling shows it is hot we move to a
  // `useSyncExternalStore` against a host-emitted notifier.
  return React.useMemo<HostTheme>(
    () => ({
      mode: slot.theme,
      tokens: slot.themeTokens,
      token(key) {
        const fromMap = slot.themeTokens?.[key];
        if (fromMap) return fromMap;
        if (typeof window === "undefined") return "";
        const styles = window.getComputedStyle(document.documentElement);
        return styles.getPropertyValue(`--${key}`).trim();
      },
    }),
    [slot.theme, slot.themeTokens],
  );
}
