import { describe, expect, it } from "vitest";

import { formatValue } from "@/features/widgets/formatValue";
import { rampColor } from "@/features/widgets/rampColor";

// Pure value formatting (unit + decimals + mappings) and ramp colour
// selection (F10).

describe("formatValue", () => {
  it("applies fixed decimals and a trailing unit symbol", () => {
    expect(formatValue(21.456, { unit: "celsius", decimals: 1 }).text).toBe("21.5 °C");
  });

  it("renders a prefix unit (currency) before the number", () => {
    expect(formatValue(12, { unit: "usd" }).text).toBe("$12");
  });

  it("scales percentunit from 0–1 to 0–100", () => {
    expect(formatValue(0.5, { unit: "percentunit", decimals: 0 }).text).toBe("50%");
  });

  it("shows the no-value placeholder for null / NaN", () => {
    expect(formatValue(null, {}).text).toBe("—");
    expect(formatValue(NaN, { noValue: "n/a" }).text).toBe("n/a");
  });

  it("a value mapping replaces the text and can supply a colour", () => {
    const r = formatValue(1, {
      mappings: [{ type: "value", match: "1", text: "On", color: "152 76% 44%" }],
    });
    expect(r.text).toBe("On");
    expect(r.color).toBe("152 76% 44%");
  });

  it("a range mapping matches within bounds", () => {
    const r = formatValue(95, {
      mappings: [{ type: "range", from: 90, to: 100, text: "High" }],
    });
    expect(r.text).toBe("High");
  });

  it("auto precision trims trailing fraction noise without a decimals setting", () => {
    expect(formatValue(3.10004, {}).text).toBe("3.1");
    expect(formatValue(7, {}).text).toBe("7");
  });
});

describe("rampColor", () => {
  const steps = [
    { value: null, color: "green" },
    { value: 70, color: "amber" },
    { value: 90, color: "red" },
  ];

  it("picks the highest step the value meets", () => {
    expect(rampColor(50, steps)).toBe("hsl(green)");
    expect(rampColor(75, steps)).toBe("hsl(amber)");
    expect(rampColor(95, steps)).toBe("hsl(red)");
  });

  it("returns undefined for an empty ramp so the caller keeps its default", () => {
    expect(rampColor(10, [])).toBeUndefined();
  });
});
