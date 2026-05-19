// `useSlotContext` — the per-slot context an extension panel sees.
//
// Different host slots (`sidebar`, `header`, `editor-footer`) carry
// different affordances: a `sidebar` panel can be wider than a
// `header` widget, theming may differ, feature flags scope per-slot.
// Rather than passing this through props (the extension's exposed
// component shape is opaque to the host), the host writes it onto a
// React context that the extension reads via this hook.
//
// `slotId` is always populated. `theme` and `flags` are best-effort
// reads of host state; missing entries default to safe values. The
// host shell mounts a `SlotContextProvider` around every exposed
// extension component before it renders.

import * as React from "react";

/**
 * Resolved colour mode for the host UI. Free-form string so consumers
 * can introduce additional modes (`"high-contrast"`, `"sepia"`, …)
 * without breaking the type, but `"light"` and `"dark"` are the two
 * the kernel recognises everywhere.
 */
export type HostThemeMode = "light" | "dark" | (string & {});

/**
 * Token map for the currently active mode. Keys are the CSS custom
 * property name *without* the leading `--` (e.g. `"primary"`,
 * `"radius"`). Values are author-typed strings — usually `oklch(...)`
 * for colours, but any CSS value the host stamped on `:root` is
 * valid.
 *
 * The host's theme is also written to `document.documentElement` as
 * `--<key>` CSS variables; any extension rendering shadcn primitives
 * from `@nube/starter-ui-kit` inherits the look for free via the
 * cascade. This map exists so extensions that need the values
 * programmatically (charts, canvas, CSS-in-JS) do not have to read
 * `getComputedStyle` themselves.
 */
export type HostThemeTokens = Readonly<Record<string, string>>;

/** Per-slot context an extension panel reads via `useSlotContext()`. */
export interface SlotContext {
  /** Host slot id the panel was mounted into (e.g. `"sidebar"`). */
  slotId: string;
  /**
   * Reverse-DNS id of the extension this panel belongs to. Useful for
   * scoping logs, telemetry tags, and per-extension state keys.
   */
  extensionId: string;
  /**
   * Active colour mode. The same value the host writes onto
   * `document.documentElement` (`.dark` class toggled when `"dark"`).
   * Extensions that switch their own assets (e.g. a logo PNG) on
   * mode read this directly instead of querying media or class names.
   */
  theme: HostThemeMode;
  /**
   * Resolved token map for `theme`. `null` when the host did not
   * supply one — call sites should fall back to `getComputedStyle`
   * on `document.documentElement`, which `useHostTheme()` does
   * automatically.
   */
  themeTokens: HostThemeTokens | null;
  /**
   * Feature flags scoped to this slot. The host populates these from
   * its own flag store; an extension reads them by name and does not
   * assume any particular flag exists.
   */
  flags: Readonly<Record<string, boolean>>;
}

const Context = React.createContext<SlotContext | null>(null);

export interface SlotContextProviderProps {
  value: SlotContext;
  children: React.ReactNode;
}

/**
 * Host-side: writes the per-slot context onto the React tree. The
 * federation runtime in `@nube/starter-ext-ui` mounts one of these
 * around every exposed extension component.
 */
export function SlotContextProvider(
  props: SlotContextProviderProps,
): React.ReactElement {
  return <Context.Provider value={props.value}>{props.children}</Context.Provider>;
}

/**
 * Read the slot context. Throws when called outside a provider — the
 * extension was rendered without the host wiring, which is a host
 * bug, not an extension bug. Throwing surfaces the wiring error in
 * the host's error boundary.
 */
export function useSlotContext(): SlotContext {
  const ctx = React.useContext(Context);
  if (!ctx) {
    throw new Error(
      "useSlotContext() called outside <SlotContextProvider>. " +
        "The host's federation runtime must wrap exposed components in SlotContextProvider.",
    );
  }
  return ctx;
}
