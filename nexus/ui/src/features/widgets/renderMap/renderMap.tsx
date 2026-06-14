import type { ReactElement } from "react";

import type { Widget, WidgetData, WidgetType } from "@/data/types";
import { Area } from "@/features/widgets/Area";
import { Bar } from "@/features/widgets/Bar";
import { DeviceTable } from "@/features/widgets/DeviceTable";
import { Gauge } from "@/features/widgets/Gauge";
import { Heatmap } from "@/features/widgets/Heatmap";
import { Line } from "@/features/widgets/Line";
import { Pie } from "@/features/widgets/Pie";
import { Scatter } from "@/features/widgets/Scatter";
import { Stat } from "@/features/widgets/Stat";
import { Status } from "@/features/widgets/Status";

// The JSX layer of the widget registry: every panel type → its renderer.
// Kept separate from `catalog.ts` (data-only) so the API boundary and the
// grid layout can read per-type metadata without pulling React/ECharts
// into their layer. Keyed by `WidgetType` so adding a type to the union
// is a compile error here until a renderer is supplied — the exhaustive
// switch this replaced gave the same guarantee, but a map also lets the
// canvas iterate renderers and carries no per-call branching cost.
type Renderer = (props: { widget: Widget; data: WidgetData }) => ReactElement;

export const WIDGET_RENDERERS: Record<WidgetType, Renderer> = {
  line: (p) => <Line {...p} />,
  area: (p) => <Area {...p} />,
  bar: (p) => <Bar {...p} />,
  scatter: (p) => <Scatter {...p} />,
  heatmap: (p) => <Heatmap {...p} />,
  pie: (p) => <Pie {...p} />,
  gauge: (p) => <Gauge {...p} />,
  stat: (p) => <Stat {...p} />,
  status: (p) => <Status {...p} />,
  table: (p) => <DeviceTable {...p} />,
};
