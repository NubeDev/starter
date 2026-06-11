import { describe, expect, it } from "vitest";
import { render } from "@testing-library/react";

import type { Widget, WidgetData } from "@/data/types";
import { Stat } from "@/features/widgets/Stat";

// The stat tile must run its value through `formatValue` so decimals, unit
// symbols, and value mappings apply — they used to be bypassed (the raw number
// went straight to MetricCard). These lock that routing in.
function widget(fieldConfig?: Widget["config"]["fieldConfig"]): Widget {
  return {
    id: "s",
    type: "stat",
    title: "S",
    layout: { x: 0, y: 0, w: 3, h: 3 },
    config: {
      query: { datasourceId: "d", sql: "select value from t" },
      fields: { series: [{ value: "value" }] },
      fieldConfig,
    },
  };
}
const rows = (v: number | null): WidgetData => ({ points: [{ value: v }] });

describe("Stat formatting (routes through formatValue)", () => {
  it("applies fixed decimals", () => {
    const { getByText } = render(
      <Stat widget={widget({ defaults: { decimals: 3 } })} data={rows(20.2531)} />,
    );
    expect(getByText("20.253")).toBeInTheDocument();
  });

  it("appends the unit symbol from the registry", () => {
    const { getByText } = render(
      <Stat widget={widget({ defaults: { unit: "kilowatthour", decimals: 1 } })} data={rows(20.25)} />,
    );
    expect(getByText("20.3 kWh")).toBeInTheDocument();
  });

  it("applies a value mapping (text replaces the number)", () => {
    const { getByText } = render(
      <Stat
        widget={widget({ defaults: { mappings: [{ type: "value", match: "1", text: "On" }] } })}
        data={rows(1)}
      />,
    );
    expect(getByText("On")).toBeInTheDocument();
  });

  it("shows the configured no-value text when there is no reading", () => {
    const { getByText } = render(
      <Stat widget={widget({ defaults: { noValue: "n/a" } })} data={rows(null)} />,
    );
    expect(getByText("n/a")).toBeInTheDocument();
  });
});
