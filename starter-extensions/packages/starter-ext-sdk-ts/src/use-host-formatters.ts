// `useHostFormatters()` — convenience hook that returns the pure
// formatter functions bound to the host's resolved prefs.
//
// The formatters themselves are pure functions exported from
// `./formatters.ts` (local mirror of
// `@nube/starter-ui-core/preferences/formatters.ts`; see that file
// for the dep-arrow rationale). The hook is a thin wrapper — it
// reads the prefs via `useHostPrefs()` and curries each formatter
// so the panel's render path never threads `prefs` through call
// sites:
//
// ```tsx
// const { formatDate, formatQuantity } = useHostFormatters();
// return <div>{formatDate(Date.now())} · {formatQuantity(22.4, "temperature", "celsius")}</div>;
// ```
//
// The returned object is memoised on the prefs object identity so a
// formatter consumer that puts the result into `useEffect`'s deps
// list does not re-fire when the host re-renders for unrelated
// reasons.

import * as React from "react";

import {
  formatCurrency,
  formatDate,
  formatNumber,
  formatQuantity,
  formatTime,
} from "./formatters.js";
import type { Quantity, Unit } from "./prefs-types.js";
import { useHostPrefs } from "./use-host-prefs.js";

export interface HostFormatters {
  /** Render a UNIX-millis timestamp as a date in the user's
   *  preferred pattern + timezone. */
  formatDate(timestampMs: number): string;
  /** Render a UNIX-millis timestamp as a time-of-day in the user's
   *  preferred clock + timezone. */
  formatTime(timestampMs: number): string;
  /** Render a number keyed off the user's locale + number format. */
  formatNumber(value: number, options?: Intl.NumberFormatOptions): string;
  /** Render a currency amount. Currency code defaults to
   *  `prefs.currency` when omitted. */
  formatCurrency(
    amount: number,
    currencyCode?: string,
    options?: Intl.NumberFormatOptions,
  ): string;
  /** Convert `value` (in `sourceUnit`) to the user's preferred unit
   *  for `quantity`, then render with the unit symbol. */
  formatQuantity(
    value: number,
    quantity: Quantity,
    sourceUnit: Unit,
    options?: Intl.NumberFormatOptions,
  ): string;
}

/**
 * Return the bound formatter bundle. The object identity is stable
 * for the lifetime of the surrounding host prefs (changes only when
 * the resolved prefs object changes — i.e. after `setPreferences`
 * resolves and the host re-renders).
 */
export function useHostFormatters(): HostFormatters {
  const prefs = useHostPrefs();
  return React.useMemo<HostFormatters>(
    () => ({
      formatDate: (ts) => formatDate(ts, prefs),
      formatTime: (ts) => formatTime(ts, prefs),
      formatNumber: (n, opts) => formatNumber(n, prefs, opts),
      formatCurrency: (amt, code, opts) =>
        formatCurrency(amt, code ?? prefs.currency, prefs, opts),
      formatQuantity: (v, q, src, opts) => formatQuantity(v, q, src, prefs, opts),
    }),
    [prefs],
  );
}
