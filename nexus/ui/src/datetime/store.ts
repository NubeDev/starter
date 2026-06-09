import { create } from "zustand";

import type { DateFormat, TimeFormat } from "@nube/starter-ui-core/preferences";

import { REGION_PREFS, type Region } from "@/datetime/regions";

// Local date/time *display settings* — the industry-standard knobs a
// user controls directly: date format, clock, and time zone. Each
// defaults to "auto"/"" meaning "follow the device locale", which is the
// expected first-visit behaviour (a US visitor sees US format, an EU
// visitor EU format, with zero setup).
//
// This is the fallback source of formatting when no `<PreferencesProvider>`
// (org/user prefs) is mounted — see `datetime/useDateTime.ts` for the
// resolution order. Persisted to localStorage; `create` is the
// workspace's single `zustand` federation singleton (like the theme + ui
// stores), so extensions share one store runtime.

const STORAGE_KEY = "nexus.datetime.settings";

/** Empty timezone string = "Automatic (device)". A non-empty value is an
 * IANA zone id (e.g. "Europe/Paris"). */
export type TimeZoneSetting = string;

/** The user's local display settings. Mirrors the `ResolvedPreferences`
 * date/time fields 1:1 so they map straight onto org/user prefs later. */
export interface DateTimeSettings {
  dateFormat: DateFormat;
  timeFormat: TimeFormat;
  timezone: TimeZoneSetting;
}

/** First-visit defaults: everything automatic (follow the browser). */
export const AUTO_SETTINGS: DateTimeSettings = {
  dateFormat: "auto",
  timeFormat: "auto",
  timezone: "",
};

function readStored(): DateTimeSettings {
  if (typeof window === "undefined") return AUTO_SETTINGS;
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return AUTO_SETTINGS;
    const parsed = JSON.parse(raw) as Partial<DateTimeSettings>;
    return { ...AUTO_SETTINGS, ...parsed };
  } catch {
    return AUTO_SETTINGS;
  }
}

function persist(s: DateTimeSettings): void {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
}

interface DateTimeSettingsState extends DateTimeSettings {
  /** Patch one or more settings. */
  set: (patch: Partial<DateTimeSettings>) => void;
  /** Reset every setting back to Automatic. */
  reset: () => void;
  /** Quick-set: apply a region preset's date/time/zone conventions. A
   *  shortcut over the three selects, not a persisted "mode" of its own. */
  applyRegion: (region: Region) => void;
}

export const useDateTimeSettings = create<DateTimeSettingsState>((set) => ({
  ...readStored(),

  set: (patch) =>
    set((prev) => {
      const next = { ...prev, ...patch };
      persist(next);
      return next;
    }),

  reset: () => {
    persist(AUTO_SETTINGS);
    return set(AUTO_SETTINGS);
  },

  applyRegion: (region) =>
    set((prev) => {
      const r = REGION_PREFS[region];
      const next: DateTimeSettings = {
        ...prev,
        dateFormat: r.date_format,
        timeFormat: r.time_format,
        timezone: r.timezone,
      };
      persist(next);
      return next;
    }),
}));
