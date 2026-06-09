// Public surface of the centralised date/time module.
//
// Components: `useDateTime()` for bound formatters (the settings UI lives
// in `app/SettingsMenu`). Off-tree (chart option builders, tests): the
// `formatWith*` pure functions.
//
// Display conventions come from the user's local date/time settings
// (date format, clock, time zone — each defaulting to "Automatic"/device
// locale). Mounting `<PreferencesProvider>` upgrades the whole app to
// real org/user preferences with no call-site changes. Region presets
// (US / EU / China) remain as a quick-set shortcut.

export {
  type Region,
  type RegionPreferences,
  REGION_PREFS,
  REGION_LABELS,
} from "@/datetime/regions";

export {
  type DateInput,
  type DateTimeFormatters,
  toEpochMs,
  prefsForRegion,
  prefsForSettings,
  mergePrefs,
  makeFormatters,
  deviceLocale,
  deviceTimeZone,
  formatDateWith,
  formatTimeWith,
  formatDateTimeWith,
  formatRangeWith,
} from "@/datetime/datetime";

export {
  useDateTimeSettings,
  AUTO_SETTINGS,
  type DateTimeSettings as DateTimeSettingsValue,
  type TimeZoneSetting,
} from "@/datetime/store";
export { useDateTime } from "@/datetime/useDateTime";
