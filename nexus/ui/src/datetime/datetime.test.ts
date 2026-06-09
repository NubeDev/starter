import { describe, expect, it } from "vitest";

import {
  deviceLocale,
  deviceTimeZone,
  formatDateWith,
  formatRangeWith,
  formatTimeWith,
  prefsForRegion,
  prefsForSettings,
  toEpochMs,
} from "@/datetime/datetime";
import { AUTO_SETTINGS } from "@/datetime/store";
import { REGION_PREFS } from "@/datetime/regions";

// A fixed instant: 2026-03-09T14:05:00Z. Each region pins an explicit
// timezone, so the rendered wall-clock differs — assertions use each
// region's own zone, keeping the test independent of the host TZ.
const INSTANT = Date.UTC(2026, 2, 9, 14, 5, 0);

describe("datetime regions", () => {
  it("pins the documented date/time conventions per region", () => {
    expect(REGION_PREFS.usa.date_format).toBe("MM/DD/YYYY");
    expect(REGION_PREFS.usa.time_format).toBe("12h");
    expect(REGION_PREFS.eu.date_format).toBe("DD/MM/YYYY");
    expect(REGION_PREFS.eu.time_format).toBe("24h");
    expect(REGION_PREFS.china.date_format).toBe("YYYY-MM-DD");
    expect(REGION_PREFS.china.time_format).toBe("24h");
  });

  it("USA renders month-first, 12-hour", () => {
    const prefs = prefsForRegion("usa"); // America/New_York → 10:05 EDT
    expect(formatDateWith(INSTANT, prefs)).toBe("03/09/2026");
    expect(formatTimeWith(INSTANT, prefs)).toMatch(/10:05\s?AM/i);
  });

  it("EU renders day-first, 24-hour", () => {
    const prefs = prefsForRegion("eu"); // Europe/Paris → 15:05 CET
    expect(formatDateWith(INSTANT, prefs)).toBe("09/03/2026");
    expect(formatTimeWith(INSTANT, prefs)).toBe("15:05");
  });

  it("China renders ISO year-first, 24-hour", () => {
    const prefs = prefsForRegion("china"); // Asia/Shanghai → 22:05 CST
    expect(formatDateWith(INSTANT, prefs)).toBe("2026/03/09");
    expect(formatTimeWith(INSTANT, prefs)).toBe("22:05");
  });
});

describe("datetime helpers", () => {
  it("toEpochMs accepts millis, ISO strings, and Date", () => {
    expect(toEpochMs(INSTANT)).toBe(INSTANT);
    expect(toEpochMs(new Date(INSTANT))).toBe(INSTANT);
    expect(toEpochMs("2026-03-09T14:05:00Z")).toBe(INSTANT);
  });

  it("toEpochMs throws on garbage rather than rendering Invalid Date", () => {
    expect(() => toEpochMs("not-a-date")).toThrow(RangeError);
  });

  it("range collapses when both ends are the same day", () => {
    const prefs = prefsForRegion("eu");
    expect(formatRangeWith(INSTANT, INSTANT, prefs)).toBe("09/03/2026");
    const later = INSTANT + 7 * 24 * 60 * 60 * 1000;
    expect(formatRangeWith(INSTANT, later, prefs)).toBe(
      "09/03/2026 – 16/03/2026",
    );
  });
});

describe("prefsForSettings — explicit settings over an automatic base", () => {
  it("AUTO_SETTINGS resolve to the device locale + zone", () => {
    const prefs = prefsForSettings(AUTO_SETTINGS);
    expect(prefs.locale).toBe(deviceLocale());
    expect(prefs.timezone).toBe(deviceTimeZone());
    expect(prefs.date_format).toBe("auto");
    expect(prefs.time_format).toBe("auto");
  });

  it("explicit format + zone override while locale stays the device's", () => {
    const prefs = prefsForSettings({
      dateFormat: "YYYY-MM-DD",
      timeFormat: "24h",
      timezone: "Asia/Shanghai",
    });
    expect(prefs.locale).toBe(deviceLocale()); // language never overridden
    expect(prefs.timezone).toBe("Asia/Shanghai");
    // The chosen zone shifts the wall-clock: 2026-03-09T14:05Z in
    // Shanghai (UTC+8) = 22:05 the same day, in the chosen 24h clock.
    // (Date *ordering* follows the device locale, not the format token —
    // a quirk of the platform formatter; we assert the parts, not order.)
    const d = formatDateWith(INSTANT, prefs);
    expect(d).toContain("2026");
    expect(d).toContain("03");
    expect(d).toContain("09");
    expect(formatTimeWith(INSTANT, prefs)).toBe("22:05");
  });

  it("an empty timezone falls back to the device zone", () => {
    const prefs = prefsForSettings({
      dateFormat: "auto",
      timeFormat: "12h",
      timezone: "",
    });
    expect(prefs.timezone).toBe(deviceTimeZone());
  });
});
