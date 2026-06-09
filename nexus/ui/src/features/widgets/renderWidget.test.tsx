import { describe, expect, it } from "vitest";
import { render } from "@testing-library/react";

import type { Widget, WidgetData } from "@/data/types";
import { RenderWidget } from "@/features/widgets/renderWidget";

// Every widget type resolves to a renderer that mounts without throwing
// when handed typed props. ECharts panels are smoke-mounted (jsdom can't
// paint canvas, but the React subtree must build); DOM panels assert
// their content. Typed props are the contract under test (F10).
const base = (type: Widget["type"]): Widget => ({
  id: `w_${type}`,
  type,
  title: `${type} panel`,
  layout: { x: 0, y: 0, w: 3, h: 3 },
  config: {
    query: { datasourceId: "ds", sql: "select a, b from t" },
    fields: { x: "a", series: [{ value: "b", label: "B" }] },
  },
});

const data: WidgetData = { points: [{ a: "n1", b: 12 }, { a: "n2", b: 8 }] };

describe("RenderWidget", () => {
  for (const type of ["line", "area", "gauge", "stat"] as const) {
    it(`mounts a ${type} panel from typed props`, () => {
      const { container } = render(
        <RenderWidget widget={base(type)} data={data} />,
      );
      expect(container.firstChild).not.toBeNull();
    });
  }

  it("renders a status list with one row per point", () => {
    const widget = base("status");
    const { getByText } = render(
      <RenderWidget
        widget={widget}
        data={{ points: [{ a: "pump", b: "online" }] }}
      />,
    );
    expect(getByText("pump")).toBeInTheDocument();
    expect(getByText("online")).toBeInTheDocument();
  });

  it("renders a device table with mapped columns", () => {
    const { getByText } = render(
      <RenderWidget widget={base("table")} data={data} />,
    );
    expect(getByText("B")).toBeInTheDocument();
    expect(getByText("12")).toBeInTheDocument();
  });
});
