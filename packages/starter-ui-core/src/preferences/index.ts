// Public surface of the preferences module. Consumers import via
// `@nube/starter-ui-core/preferences` (see `package.json#exports`).

export type {
  CurrencyCode,
  DateFormat,
  NumberFormat,
  PreferencesPatch,
  Quantity,
  ResolvedPreferences,
  Theme,
  TimeFormat,
  Unit,
  UnitSystem,
  WeekStart,
} from "./types.js";

export {
  ALLOWED_UNITS,
  CANONICAL_UNIT,
  TO_CANONICAL,
  UNIT_QUANTITY,
  UNIT_SYMBOL,
  UnitConversionError,
  convertUnit,
} from "./units.js";
export type { UnitsResponse } from "./units.js";

export {
  formatCurrency,
  formatDate,
  formatNumber,
  formatQuantity,
  formatTime,
  preferredUnitFor,
} from "./formatters.js";

export {
  DEFAULT_WORKSPACE,
  PreferencesContext,
  PreferencesProvider,
  usePreferences,
} from "./provider.js";
export type {
  PreferencesContextValue,
  PreferencesProviderProps,
} from "./provider.js";

export { SettingsPage } from "./SettingsPage.js";
export type { SettingsPageProps, ToastFn } from "./SettingsPage.js";
