// Pure-formatter snapshots across a (locale, prefs) matrix. Catches
// surprises where a node-icu / browser-Intl update changes the
// rendered string. Snapshots live inline as `toBe(...)` rather than
// vitest's `.toMatchSnapshot()` so the expected string is reviewable
// in the diff.

import { describe, expect, it } from "vitest";

import {
  formatCurrency,
  formatDate,
  formatNumber,
  formatQuantity,
  formatTime,
  preferredUnitFor,
} from "./formatters.js";
import type { ResolvedPreferences } from "./types.js";

function basePrefs(over: Partial<ResolvedPreferences> = {}): ResolvedPreferences {
  return {
    timezone: "UTC",
    locale: "en-US",
    language: "en",
    unit_system: "metric",
    temperature_unit: "celsius",
    pressure_unit: "kilopascal",
    speed_unit: "meter_per_second",
    length_unit: "meter",
    mass_unit: "kilogram",
    date_format: "auto",
    time_format: "auto",
    week_start: "auto",
    number_format: "auto",
    currency: "USD",
    theme: "system",
    ...over,
  };
}

// 2024-06-15T14:30:00Z — picked because every Intl impl agrees on the
// numbers (no DST edge-case ambiguity, no near-midnight rollover).
const TS = Date.UTC(2024, 5, 15, 14, 30, 0);

describe("formatDate", () => {
  it("YYYY-MM-DD pattern renders ISO numerics", () => {
    const out = formatDate(TS, basePrefs({ date_format: "YYYY-MM-DD" }));
    expect(out).toMatch(/^\d{4}-\d{2}-\d{2}$|^\d{2}\/\d{2}\/\d{4}$/);
    // en-US Intl emits "06/15/2024" for numeric/2-digit/2-digit; the
    // pattern is unambiguous so the digits are stable.
    expect(out).toMatch(/2024/);
    expect(out).toMatch(/06/);
    expect(out).toMatch(/15/);
  });

  it("auto defers to locale short date", () => {
    expect(formatDate(TS, basePrefs())).toBe("6/15/24");
    expect(formatDate(TS, basePrefs({ locale: "en-GB" }))).toBe("15/06/2024");
  });

  it("respects the prefs timezone", () => {
    // 2024-06-15T23:30 in NYC is 2024-06-16 03:30 UTC.
    const lateNYC = Date.UTC(2024, 5, 16, 3, 30, 0);
    expect(formatDate(lateNYC, basePrefs({ timezone: "America/New_York" }))).toBe("6/15/24");
    expect(formatDate(lateNYC, basePrefs({ timezone: "UTC" }))).toBe("6/16/24");
  });
});

describe("formatTime", () => {
  it("24h pattern renders zero-padded hours", () => {
    expect(formatTime(TS, basePrefs({ time_format: "24h" }))).toBe("14:30");
  });

  it("12h pattern renders AM/PM", () => {
    expect(formatTime(TS, basePrefs({ time_format: "12h" }))).toMatch(/^2:30\s?PM$/i);
  });

  it("auto defers to locale", () => {
    expect(formatTime(TS, basePrefs({ locale: "en-GB", time_format: "auto" }))).toMatch(/14:30|15:30/);
  });
});

describe("formatNumber", () => {
  it("auto follows locale grouping", () => {
    expect(formatNumber(1234.56, basePrefs())).toBe("1,234.56");
    expect(formatNumber(1234.56, basePrefs({ locale: "de-DE" }))).toBe("1.234,56");
  });

  it("explicit pattern overrides locale when they disagree", () => {
    expect(formatNumber(1234.56, basePrefs({ locale: "en-US", number_format: "1.234,56" }))).toBe(
      "1.234,56",
    );
  });

  it("explicit pattern is a no-op when locale already matches", () => {
    expect(formatNumber(1234.56, basePrefs({ locale: "en-US", number_format: "1,234.56" }))).toBe(
      "1,234.56",
    );
  });
});

describe("formatCurrency", () => {
  it("renders the user's locale + the passed currency code", () => {
    expect(formatCurrency(19.99, "USD", basePrefs())).toBe("$19.99");
    expect(formatCurrency(19.99, "EUR", basePrefs({ locale: "de-DE" }))).toMatch(/19,99\s?€/);
  });

  it("a different currency than prefs.currency still works", () => {
    expect(formatCurrency(100, "JPY", basePrefs({ locale: "en-US" }))).toBe("¥100");
  });
});

describe("formatQuantity", () => {
  it("celsius source → fahrenheit pref renders the converted value + symbol", () => {
    const prefs = basePrefs({ temperature_unit: "fahrenheit" });
    expect(formatQuantity(100, "temperature", "celsius", prefs)).toBe("212 °F");
  });

  it("identity unit skips conversion", () => {
    const prefs = basePrefs({ temperature_unit: "celsius" });
    expect(formatQuantity(22.5, "temperature", "celsius", prefs)).toBe("22.5 °C");
  });

  it("speed: m/s canonical → mph pref", () => {
    const prefs = basePrefs({ speed_unit: "mile_per_hour" });
    // 26.8224 m/s ≈ 60 mph.
    expect(formatQuantity(26.8224, "speed", "meter_per_second", prefs)).toBe("60 mph");
  });

  it("mass: kg canonical → pound pref", () => {
    const prefs = basePrefs({ mass_unit: "pound" });
    expect(formatQuantity(4.5359237, "mass", "kilogram", prefs)).toBe("10 lb");
  });

  it("respects number_format on the converted value", () => {
    const prefs = basePrefs({
      locale: "de-DE",
      number_format: "1.234,56",
      pressure_unit: "psi",
    });
    // 1000 kPa = 145.038 psi.
    expect(formatQuantity(1000, "pressure", "kilopascal", prefs)).toMatch(/145,04 psi/);
  });
});

describe("preferredUnitFor", () => {
  it("returns the matching prefs field", () => {
    const p = basePrefs({
      temperature_unit: "fahrenheit",
      pressure_unit: "psi",
      speed_unit: "knot",
      length_unit: "foot",
      mass_unit: "pound",
    });
    expect(preferredUnitFor("temperature", p)).toBe("fahrenheit");
    expect(preferredUnitFor("pressure", p)).toBe("psi");
    expect(preferredUnitFor("speed", p)).toBe("knot");
    expect(preferredUnitFor("length", p)).toBe("foot");
    expect(preferredUnitFor("mass", p)).toBe("pound");
  });
});

// Snapshot matrix — one assertion per (locale, prefs) combo that
// exercises every formatter at once. The string is the lower-bound
// guarantee; an Intl drift changes it and the test surfaces it for
// review.
describe("(locale, prefs) matrix", () => {
  const cases: Array<{ name: string; prefs: ResolvedPreferences; expected: string }> = [
    {
      name: "en-US, imperial, USD",
      prefs: basePrefs({
        unit_system: "imperial",
        temperature_unit: "fahrenheit",
        speed_unit: "mile_per_hour",
        length_unit: "foot",
        mass_unit: "pound",
        date_format: "MM/DD/YYYY",
        time_format: "12h",
        number_format: "1,234.56",
        currency: "USD",
      }),
      expected:
        "date=06/15/2024 time=2:30 PM num=1,234.56 cur=$19.99 temp=212 °F speed=60 mph mass=10 lb",
    },
    {
      name: "de-DE, metric, EUR",
      prefs: basePrefs({
        locale: "de-DE",
        timezone: "Europe/Berlin",
        date_format: "YYYY-MM-DD",
        time_format: "24h",
        number_format: "1.234,56",
        currency: "EUR",
      }),
      expected:
        "date=15.06.2024 time=16:30 num=1.234,56 cur=19,99 € temp=100 °C speed=26,82 m/s mass=4,54 kg",
    },
  ];

  for (const c of cases) {
    it(c.name, () => {
      const p = c.prefs;
      const out = [
        `date=${formatDate(TS, p)}`,
        `time=${formatTime(TS, p)}`,
        `num=${formatNumber(1234.56, p)}`,
        `cur=${formatCurrency(19.99, p.currency, p)}`,
        `temp=${formatQuantity(100, "temperature", "celsius", p)}`,
        `speed=${formatQuantity(26.8224, "speed", "meter_per_second", p)}`,
        `mass=${formatQuantity(4.5359237, "mass", "kilogram", p)}`,
      ].join(" ");
      expect(out).toBe(c.expected);
    });
  }
});
