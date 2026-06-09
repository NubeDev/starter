import type { Widget, WidgetData } from "@/data/types";
import { EChart } from "@/features/widgets/EChart";
import { buildPieOption } from "@/features/widgets/pieOption";
import { useThemeStore } from "@/theme/store";

// Pie panel. Pure: renders the ECharts option built from its typed props,
// fetches nothing (F6). Data arrives from `WidgetCard`.
export function Pie({ widget, data }: { widget: Widget; data: WidgetData }) {
  // Re-render on dark/light switch so the theme-resolved slice colours
  // are rebuilt (see `Area` for the rationale).
  useThemeStore((s) => s.mode);
  return (
    <EChart option={buildPieOption(widget, data)} ariaLabel={widget.title} />
  );
}
