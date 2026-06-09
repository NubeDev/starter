import type { Widget, WidgetData } from "@/data/types";
import { EChart } from "@/features/widgets/EChart";
import { buildLineOption } from "@/features/widgets/lineOption";

// Line panel. Pure: renders the ECharts option built from its typed
// props, fetches nothing (F6). Data arrives from `WidgetCard`.
export function Line({ widget, data }: { widget: Widget; data: WidgetData }) {
  return (
    <EChart
      option={buildLineOption(widget, data, { area: false })}
      ariaLabel={widget.title}
    />
  );
}
