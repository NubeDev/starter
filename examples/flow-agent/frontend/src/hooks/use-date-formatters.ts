// Preferences-aware date/time formatters scoped to flow-agent's
// existing call sites. Centralised here so each page can call a
// single hook (`useDateFormatters()`) instead of pulling the
// formatters + prefs + null-check boilerplate at every leaf.
//
// `prefs` may be `null` during the initial fetch — every helper falls
// back to the browser's `Date.prototype.toLocale*` API in that window
// so the UI continues rendering instead of throwing.

import { useCallback } from "react";
import {
  formatDate as fmtDate,
  formatDateTime as fmtDateTime,
  formatTime as fmtTime,
  usePreferences,
} from "@nube/starter-ui-core/preferences";

export interface DateFormatters {
  /** Date only ("6/15/24" in en-US, "15/06/2024" in es-ES). */
  date: (ts: number | string | Date) => string;
  /** Time only ("14:30" in 24h, "2:30 PM" in 12h). */
  time: (ts: number | string | Date) => string;
  /** Combined date + time (preference-aware replacement for
   * `Date.prototype.toLocaleString()`). */
  dateTime: (ts: number | string | Date) => string;
}

function toMillis(input: number | string | Date): number {
  if (typeof input === "number") return input;
  if (input instanceof Date) return input.getTime();
  return new Date(input).getTime();
}

/** Returns a small bag of formatters keyed off the resolved
 * preferences (locale, timezone, date_format, time_format). Falls
 * back to the browser default while preferences are loading. */
export function useDateFormatters(): DateFormatters {
  const { preferences } = usePreferences();

  const date = useCallback(
    (input: number | string | Date) => {
      const ms = toMillis(input);
      return preferences ? fmtDate(ms, preferences) : new Date(ms).toLocaleDateString();
    },
    [preferences],
  );

  const time = useCallback(
    (input: number | string | Date) => {
      const ms = toMillis(input);
      return preferences ? fmtTime(ms, preferences) : new Date(ms).toLocaleTimeString();
    },
    [preferences],
  );

  const dateTime = useCallback(
    (input: number | string | Date) => {
      const ms = toMillis(input);
      return preferences ? fmtDateTime(ms, preferences) : new Date(ms).toLocaleString();
    },
    [preferences],
  );

  return { date, time, dateTime };
}
