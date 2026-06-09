import type { Widget, WidgetData } from "@/data/types";
import { EChart } from "@/features/widgets/EChart";
import { buildBarOption } from "@/features/widgets/barOption";
import { useThemeStore } from "@/theme/store";
import { useDateTime } from "@/datetime";

// Bar panel. Pure: renders the ECharts option built from its typed props,
// fetches nothing (F6). Data arrives from `WidgetCard`.
export function Bar({ widget, data }: { widget: Widget; data: WidgetData }) {
  // Re-render on dark/light switch so the theme-resolved series colours
  // are rebuilt (see `Area` for the full rationale).
  useThemeStore((s) => s.mode);
  const { date } = useDateTime();
  return (
    <EChart
      option={buildBarOption(widget, data, { formatX: date })}
      ariaLabel={widget.title}
    />
  );
}
