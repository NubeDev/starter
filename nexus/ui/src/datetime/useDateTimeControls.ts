import { useCallback, useContext, useMemo } from "react";

import {
  PreferencesContext,
  type DateFormat,
  type PreferencesPatch,
  type TimeFormat,
} from "@nube/starter-ui-core/preferences";

import { useDateTimeSettings, type DateTimeSettings } from "@/datetime/store";
import { REGION_PREFS, type Region } from "@/datetime/regions";

/**
 * The single hook the settings UI binds to for *editing* date/time
 * display — the write-side companion to `useDateTime()` (the read side).
 *
 * It hides *where the setting lives*:
 *
 *   • **Backend mode** — when a `<PreferencesProvider>` has resolved the
 *     caller's preferences (a live session), edits PATCH
 *     `/api/v1/me/preferences`, so they persist server-side and apply to
 *     every device + consumer (the WS-11 goal). Values read back from the
 *     resolved prefs.
 *   • **Local mode** — with no backend prefs (unauthed, or the probe
 *     in flight), edits fall through to the per-device `datetime/store.ts`
 *     (localStorage), exactly as before.
 *
 * Both modes feed the same `useDateTime()` resolution order, so the live
 * preview and the whole app stay consistent whichever backs the edit.
 * The menu's JSX is identical either way — only the source swaps.
 */
export interface DateTimeControls {
  dateFormat: DateFormat;
  timeFormat: TimeFormat;
  /** Patch one or more date/time display settings. */
  set: (patch: { dateFormat?: DateFormat; timeFormat?: TimeFormat }) => void;
  /** Revert date/time display to automatic (inherit, in backend mode). */
  reset: () => void;
  /** Quick-set a region's date/time conventions. */
  applyRegion: (region: Region) => void;
  /** True when edits persist to the backend (a session is present). */
  backed: boolean;
}

export function useDateTimeControls(): DateTimeControls {
  const prefsCtx = useContext(PreferencesContext);
  const backendPrefs = prefsCtx?.preferences ?? null;
  const setPreferences = prefsCtx?.setPreferences;
  const backed = backendPrefs !== null && setPreferences !== undefined;

  // Local store (always available; the fallback backing).
  const local = useDateTimeSettings();

  const set = useCallback(
    (patch: { dateFormat?: DateFormat; timeFormat?: TimeFormat }) => {
      if (backed && setPreferences) {
        const body: PreferencesPatch = {};
        if (patch.dateFormat !== undefined) body.date_format = patch.dateFormat;
        if (patch.timeFormat !== undefined) body.time_format = patch.timeFormat;
        void setPreferences(body);
      } else {
        local.set(patch as Partial<DateTimeSettings>);
      }
    },
    [backed, setPreferences, local],
  );

  const reset = useCallback(() => {
    if (backed && setPreferences) {
      // `null` reverts each field to inherit (org → system default).
      void setPreferences({
        date_format: null,
        time_format: null,
        timezone: null,
      });
    } else {
      local.reset();
    }
  }, [backed, setPreferences, local]);

  const applyRegion = useCallback(
    (region: Region) => {
      if (backed && setPreferences) {
        const r = REGION_PREFS[region];
        // Persist the region's full display slice — the backend owns
        // these fields, so a region quick-set is durable and complete.
        void setPreferences({
          date_format: r.date_format,
          time_format: r.time_format,
          timezone: r.timezone,
          locale: r.locale,
          number_format: r.number_format,
          week_start: r.week_start,
        });
      } else {
        local.applyRegion(region);
      }
    },
    [backed, setPreferences, local],
  );

  return useMemo(
    () => ({
      dateFormat: backendPrefs?.date_format ?? local.dateFormat,
      timeFormat: backendPrefs?.time_format ?? local.timeFormat,
      set,
      reset,
      applyRegion,
      backed,
    }),
    [
      backendPrefs?.date_format,
      backendPrefs?.time_format,
      local.dateFormat,
      local.timeFormat,
      set,
      reset,
      applyRegion,
      backed,
    ],
  );
}
