import type { Widget, WidgetData } from "@/data/types";
import { EChart } from "@/features/widgets/EChart";
import { buildLineOption } from "@/features/widgets/lineOption";

// Area panel — a line with a filled region. Shares the option builder
// with `Line` (area is one flag); pure, data via props (F6).
export function Area({ widget, data }: { widget: Widget; data: WidgetData }) {
  return (
    <EChart
      option={buildLineOption(widget, data, { area: true })}
      ariaLabel={widget.title}
    />
  );
}
