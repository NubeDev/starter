import { useContext, useMemo } from "react";

import { PreferencesContext } from "@nube/starter-ui-core/preferences";

import { useDateTimeSettings } from "@/datetime/store";
import {
  makeFormatters,
  prefsForSettings,
  type DateTimeFormatters,
} from "@/datetime/datetime";

/**
 * The one hook components use to format dates/times.
 *
 *   const { date, time, dateTime, range } = useDateTime();
 *   <span>{dateTime(row.recordedAt)}</span>
 *
 * Resolution order (the "plug in org/user prefs later" seam):
 *   1. If a `<PreferencesProvider>` is mounted *and* has resolved
 *      preferences, those win — real org/user settings drive display.
 *   2. Otherwise the user's local `DateTimeSettings` (date format, clock,
 *      time zone) do, each defaulting to "Automatic" (the device locale).
 *
 * Reads `PreferencesContext` directly (not `usePreferences()`, which
 * throws without a provider) so the hook is safe to call whether or not
 * the preferences backend is wired — no call-site changes when it is.
 */
export function useDateTime(): DateTimeFormatters {
  // `undefined` when no provider is mounted; `.preferences` is `null`
  // while the initial prefs probe is in flight.
  const prefsCtx = useContext(PreferencesContext);
  const dateFormat = useDateTimeSettings((s) => s.dateFormat);
  const timeFormat = useDateTimeSettings((s) => s.timeFormat);
  const timezone = useDateTimeSettings((s) => s.timezone);

  return useMemo(() => {
    const orgUserPrefs = prefsCtx?.preferences ?? null;
    const prefs =
      orgUserPrefs ?? prefsForSettings({ dateFormat, timeFormat, timezone });
    return makeFormatters(prefs);
  }, [prefsCtx?.preferences, dateFormat, timeFormat, timezone]);
}
