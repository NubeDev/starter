import type { Widget, WidgetData } from "@/data/types";
import { EChart } from "@/features/widgets/EChart";
import { buildHeatmapOption } from "@/features/widgets/heatmapOption";
import { useThemeStore } from "@/theme/store";

// Heatmap panel. Pure: renders the ECharts option built from its typed
// props, fetches nothing (F6). Data arrives from `WidgetCard`.
export function Heatmap({ widget, data }: { widget: Widget; data: WidgetData }) {
  // Re-render on dark/light switch so the theme-resolved colour ramp is
  // rebuilt (see `Area` for the rationale).
  useThemeStore((s) => s.mode);
  return (
    <EChart option={buildHeatmapOption(widget, data)} ariaLabel={widget.title} />
  );
}
