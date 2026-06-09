import type { Widget, WidgetData } from "@/data/types";
import { EChart } from "@/features/widgets/EChart";
import { buildGaugeOption } from "@/features/widgets/gaugeOption";

// Gauge panel — threshold-aware radial dial. Pure; the arc colour and
// value come from the option builder, which reads only typed props (F6).
export function Gauge({ widget, data }: { widget: Widget; data: WidgetData }) {
  return (
    <EChart option={buildGaugeOption(widget, data)} ariaLabel={widget.title} />
  );
}
