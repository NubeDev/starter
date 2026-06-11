import type { Widget, WidgetData } from "@/data/types";
import { EChart } from "@/features/widgets/EChart";
import { buildLineOption } from "@/features/widgets/Line/lineOption";
import { useThemeStore } from "@/theme/store";
import { useDateTime } from "@/datetime";

// Line panel. Pure: renders the ECharts option built from its typed
// props, fetches nothing (F6). Data arrives from `WidgetCard`.
export function Line({ widget, data }: { widget: Widget; data: WidgetData }) {
  // Re-render on dark/light switch so the theme-resolved series colours
  // are rebuilt (see `Area` for the full rationale).
  useThemeStore((s) => s.mode);
  // Region/preference-aware x-axis formatter; the builder applies it only
  // when `fields.xKind === "time"`. `useDateTime` re-renders on region
  // change, so the axis re-formats automatically.
  const { date } = useDateTime();
  return (
    <EChart
      option={buildLineOption(widget, data, { area: false, formatX: date })}
      ariaLabel={widget.title}
    />
  );
}
