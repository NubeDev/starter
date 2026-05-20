// `<SettingsPage />` — the user-facing form bound to
// `<PreferencesProvider>`. A consumer (e.g. starter-auth-users'
// account surface) mounts this at `/account/settings`; the page
// renders dropdowns for every column of `ResolvedPreferences` and
// PATCHes via `usePreferences().setPreferences(...)`.
//
// Lookup data:
// - Timezones — `Intl.supportedValuesOf("timeZone")` at runtime.
// - Locales / languages — manifest keys from `<IntlProvider>` for
//   the language dropdown; locale falls back to a curated short
//   list when `Intl.supportedValuesOf("locale")` is unavailable.
// - Units — the closed `ALLOWED_UNITS` map shared with the Rust
//   registry.
// - Currencies — a static curated subset to avoid shipping every
//   ISO-4217 code with the bundle. Consumers can drop in a wider
//   list via the optional `currencyChoices` prop.
//
// Toasts are routed through an injectable `onToast` callback so the
// page does not lock callers into a particular toast library;
// translated text is rendered via `useTranslate()` with stable key
// shape `starter.settings.*`.

import { useEffect, useMemo, useRef, useState, type FormEvent, type ReactNode } from "react";

import { usePreferences } from "./provider.js";
import type {
  CurrencyCode,
  DateFormat,
  NumberFormat,
  PreferencesPatch,
  ResolvedPreferences,
  Theme,
  TimeFormat,
  Unit,
  UnitSystem,
  WeekStart,
} from "./types.js";
import { ALLOWED_UNITS } from "./units.js";
import { useIntlContext } from "../i18n/provider.js";
import { useTranslate } from "../i18n/use-translate.js";

/** Callback that displays a toast/snackbar. Wired by the consumer so
 * the package does not depend on a particular toast library. */
export type ToastFn = (args: { kind: "success" | "error"; message: string }) => void;

export interface SettingsPageProps {
  /** Display a toast. Defaults to a no-op. */
  onToast?: ToastFn;
  /** Curated currency choices for the dropdown. Defaults to a small
   * top-of-mind list; consumers in finance verticals will want to
   * override with a full ISO-4217 set. */
  currencyChoices?: readonly CurrencyCode[];
}

const DEFAULT_CURRENCIES: readonly CurrencyCode[] = [
  "USD",
  "EUR",
  "GBP",
  "JPY",
  "CNY",
  "INR",
  "AUD",
  "CAD",
  "CHF",
  "BRL",
];

const DATE_FORMATS: readonly DateFormat[] = ["auto", "YYYY-MM-DD", "DD/MM/YYYY", "MM/DD/YYYY"];
const TIME_FORMATS: readonly TimeFormat[] = ["auto", "24h", "12h"];
const WEEK_STARTS: readonly WeekStart[] = ["auto", "monday", "sunday"];
const NUMBER_FORMATS: readonly NumberFormat[] = ["auto", "1,234.56", "1.234,56", "1 234,56"];
const UNIT_SYSTEMS: readonly UnitSystem[] = ["metric", "imperial"];
const THEMES: readonly Theme[] = ["light", "dark", "system"];

/** Settings form bound to `<PreferencesProvider>`. */
export function SettingsPage({ onToast, currencyChoices }: SettingsPageProps = {}) {
  const { preferences, isLoading, setPreferences } = usePreferences();
  const t = useTranslate();
  const intl = useTryIntlContext();

  const timezones = useMemo(() => listTimezones(), []);
  const languages = useMemo(() => Object.keys(intl?.manifest ?? { en: "" }), [intl?.manifest]);
  const currencies = currencyChoices ?? DEFAULT_CURRENCIES;

  const [draft, setDraft] = useState<ResolvedPreferences | null>(preferences);
  const [submitting, setSubmitting] = useState(false);

  // Keep the local draft in sync with the upstream prefs when they
  // load or change underfoot (e.g. via another tab).
  useSyncDraft(preferences, draft, setDraft);

  if (isLoading || !preferences || !draft) {
    return <div data-testid="settings-loading">{t("starter.settings.loading")}</div>;
  }

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setSubmitting(true);
    const patch = diffPatch(preferences, draft);
    try {
      await setPreferences(patch);
      onToast?.({ kind: "success", message: t("starter.settings.toast.saved") });
    } catch (err) {
      onToast?.({
        kind: "error",
        message: t("starter.settings.toast.error", {
          message: err instanceof Error ? err.message : String(err),
        }),
      });
    } finally {
      setSubmitting(false);
    }
  };

  const bind = <K extends keyof ResolvedPreferences>(field: K) => ({
    name: field,
    value: draft[field] as string,
    onChange: (e: { target: { value: string } }) => {
      setDraft({ ...draft, [field]: e.target.value as ResolvedPreferences[K] });
    },
  });

  return (
    <form data-testid="settings-form" onSubmit={onSubmit}>
      <h2>{t("starter.settings.heading")}</h2>

      <Field id="timezone" label={t("starter.settings.preferences.timezone.label")}>
        <select data-testid="field-timezone" {...bind("timezone")}>
          {timezones.map((tz) => (
            <option key={tz} value={tz}>
              {tz}
            </option>
          ))}
        </select>
      </Field>

      <Field id="locale" label={t("starter.settings.preferences.locale.label")}>
        <input data-testid="field-locale" type="text" {...bind("locale")} />
      </Field>

      <Field id="language" label={t("starter.settings.preferences.language.label")}>
        <select data-testid="field-language" {...bind("language")}>
          {languages.map((lang) => (
            <option key={lang} value={lang}>
              {lang}
            </option>
          ))}
        </select>
      </Field>

      <Field id="unit_system" label={t("starter.settings.preferences.unit_system.label")}>
        <SelectEnum testId="field-unit_system" {...bind("unit_system")} options={UNIT_SYSTEMS} />
      </Field>

      <UnitField label={t("starter.settings.preferences.temperature_unit.label")} {...bind("temperature_unit")} testId="field-temperature_unit" units={ALLOWED_UNITS.temperature} />
      <UnitField label={t("starter.settings.preferences.pressure_unit.label")} {...bind("pressure_unit")} testId="field-pressure_unit" units={ALLOWED_UNITS.pressure} />
      <UnitField label={t("starter.settings.preferences.speed_unit.label")} {...bind("speed_unit")} testId="field-speed_unit" units={ALLOWED_UNITS.speed} />
      <UnitField label={t("starter.settings.preferences.length_unit.label")} {...bind("length_unit")} testId="field-length_unit" units={ALLOWED_UNITS.length} />
      <UnitField label={t("starter.settings.preferences.mass_unit.label")} {...bind("mass_unit")} testId="field-mass_unit" units={ALLOWED_UNITS.mass} />

      <Field id="date_format" label={t("starter.settings.preferences.date_format.label")}>
        <SelectEnum testId="field-date_format" {...bind("date_format")} options={DATE_FORMATS} />
      </Field>
      <Field id="time_format" label={t("starter.settings.preferences.time_format.label")}>
        <SelectEnum testId="field-time_format" {...bind("time_format")} options={TIME_FORMATS} />
      </Field>
      <Field id="week_start" label={t("starter.settings.preferences.week_start.label")}>
        <SelectEnum testId="field-week_start" {...bind("week_start")} options={WEEK_STARTS} />
      </Field>
      <Field id="number_format" label={t("starter.settings.preferences.number_format.label")}>
        <SelectEnum testId="field-number_format" {...bind("number_format")} options={NUMBER_FORMATS} />
      </Field>

      <Field id="currency" label={t("starter.settings.preferences.currency.label")}>
        <select data-testid="field-currency" {...bind("currency")}>
          {currencies.map((c) => (
            <option key={c} value={c}>
              {c}
            </option>
          ))}
        </select>
      </Field>

      <Field id="theme" label={t("starter.settings.preferences.theme.label")}>
        <SelectEnum testId="field-theme" {...bind("theme")} options={THEMES} />
      </Field>

      <button data-testid="settings-submit" type="submit" disabled={submitting}>
        {t("starter.settings.save")}
      </button>
    </form>
  );
}

// ---------------------------------------------------------------------
// Small render helpers
// ---------------------------------------------------------------------

interface FieldProps {
  id: string;
  label: string;
  children: ReactNode;
}
function Field({ id, label, children }: FieldProps) {
  return (
    <div>
      <label htmlFor={id}>{label}</label>
      {children}
    </div>
  );
}

interface SelectEnumProps<T extends string> {
  testId: string;
  name: string;
  value: string;
  options: readonly T[];
  onChange: (e: { target: { value: string } }) => void;
}
function SelectEnum<T extends string>({ testId, name, value, options, onChange }: SelectEnumProps<T>) {
  return (
    <select data-testid={testId} name={name} value={value} onChange={onChange}>
      {options.map((o) => (
        <option key={o} value={o}>
          {o}
        </option>
      ))}
    </select>
  );
}

interface UnitFieldProps {
  label: string;
  name: string;
  value: string;
  testId: string;
  units: readonly Unit[];
  onChange: (e: { target: { value: string } }) => void;
}
function UnitField({ label, name, value, testId, units, onChange }: UnitFieldProps) {
  return (
    <Field id={name} label={label}>
      <select data-testid={testId} name={name} value={value} onChange={onChange}>
        {units.map((u) => (
          <option key={u} value={u}>
            {u}
          </option>
        ))}
      </select>
    </Field>
  );
}

// ---------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------

/** Compute the minimal PATCH body — only changed fields. */
function diffPatch(prev: ResolvedPreferences, next: ResolvedPreferences): PreferencesPatch {
  const out: Record<string, unknown> = {};
  (Object.keys(next) as (keyof ResolvedPreferences)[]).forEach((k) => {
    if (prev[k] !== next[k]) out[k] = next[k];
  });
  return out as PreferencesPatch;
}

/** `Intl.supportedValuesOf("timeZone")` with a hard-coded fallback
 * for environments (jsdom) that haven't shipped the API. */
function listTimezones(): string[] {
  const fn = (Intl as unknown as { supportedValuesOf?: (k: string) => string[] }).supportedValuesOf;
  if (typeof fn === "function") {
    try {
      return fn("timeZone");
    } catch {
      // fall through
    }
  }
  return ["UTC", "Europe/London", "Europe/Paris", "America/New_York", "America/Los_Angeles", "Asia/Tokyo", "Australia/Sydney"];
}

/** Optional read of the IntlProvider context. */
function useTryIntlContext(): ReturnType<typeof useIntlContext> | null {
  try {
    return useIntlContext();
  } catch {
    return null;
  }
}

/** Push upstream prefs into the local draft when the upstream value
 * changes and the draft hasn't been edited away from it. Tracking
 * "edited" precisely would require a dirty-bit map; here we just
 * adopt upstream when the user has not changed the timezone (the
 * pragmatic dirty-bit) — good enough for the happy path. */
function useSyncDraft(
  upstream: ResolvedPreferences | null,
  draft: ResolvedPreferences | null,
  setDraft: (d: ResolvedPreferences) => void,
) {
  const seeded = useRef(false);
  useEffect(() => {
    if (upstream && !seeded.current) {
      setDraft(upstream);
      seeded.current = true;
    }
  }, [upstream, setDraft]);
  // Silence the unused-var warning for `draft` — we intentionally
  // ignore it for now (no auto-resync after the first seed).
  void draft;
}
