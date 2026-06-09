// Regional date/time presets — the developer-facing "easy knob".
//
// The platform already ships a full per-principal preferences system
// (`@nube/starter-ui-core/preferences`: locale, timezone, date/time/
// number format, week start, …) plus pure `formatDate/formatTime/
// formatDateTime` formatters keyed off `ResolvedPreferences`. That is
// the source of truth once an org/user has saved preferences.
//
// This file is the *bootstrap* layer for the common case "just show
// dates the way the US / EU / China expect" before (or instead of) a
// backend preference store. Each region is a partial `ResolvedPreferences`
// — the display-relevant subset. The active region resolves through
// `datetime/store.ts`, and `useDateTime()` prefers a mounted
// `PreferencesProvider` over the region when one exists, so wiring real
// org/user prefs later is a drop-in with no call-site changes.

import type { ResolvedPreferences } from "@nube/starter-ui-core/preferences";

/** The three supported display regions. Extend this union (and
 * `REGION_PREFS` / `REGION_LABELS`) to add more. */
export type Region = "usa" | "eu" | "china";

/** The display-relevant slice of `ResolvedPreferences` a region pins.
 * The rest of `ResolvedPreferences` (units, currency, theme, language)
 * is left to the platform preferences layer / its defaults. */
export type RegionPreferences = Pick<
  ResolvedPreferences,
  | "locale"
  | "timezone"
  | "date_format"
  | "time_format"
  | "number_format"
  | "week_start"
>;

/**
 * Per-region display conventions.
 *
 *                USA              EU               China
 *   date         MM/DD/YYYY       DD/MM/YYYY       YYYY-MM-DD
 *   time         12-hour          24-hour          24-hour
 *   number       1,234.56         1.234,56         1,234.56
 *   week starts  Sunday           Monday           Monday
 *
 * `timezone` is a sensible regional default only — a real deployment
 * should override it from the org/user preference once known.
 */
export const REGION_PREFS: Readonly<Record<Region, RegionPreferences>> = {
  usa: {
    locale: "en-US",
    timezone: "America/New_York",
    date_format: "MM/DD/YYYY",
    time_format: "12h",
    number_format: "1,234.56",
    week_start: "sunday",
  },
  eu: {
    locale: "en-GB",
    timezone: "Europe/Paris",
    date_format: "DD/MM/YYYY",
    time_format: "24h",
    number_format: "1.234,56",
    week_start: "monday",
  },
  china: {
    locale: "zh-CN",
    timezone: "Asia/Shanghai",
    date_format: "YYYY-MM-DD",
    time_format: "24h",
    number_format: "1,234.56",
    week_start: "monday",
  },
};

/** Short labels for the quick-set region buttons. */
export const REGION_LABELS: Readonly<Record<Region, string>> = {
  usa: "US",
  eu: "EU",
  china: "China",
};
