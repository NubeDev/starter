import type { Widget, WidgetData } from "@/data/types";
import { EChart } from "@/features/widgets/EChart";
import { buildLineOption } from "@/features/widgets/lineOption";
import { useThemeStore } from "@/theme/store";
import { useDateTime } from "@/datetime";

// Area panel — a line with a filled region. Shares the option builder
// with `Line` (area is one flag); pure, data via props (F6).
export function Area({ widget, data }: { widget: Widget; data: WidgetData }) {
  // Subscribe to the colour mode so the option (whose series colours are
  // resolved from the theme tokens) is rebuilt when the user switches
  // dark/light — the store invalidates the colour cache before this runs.
  useThemeStore((s) => s.mode);
  // Region-aware time-axis formatter; applied only when xKind === "time"
  // (see `Line` for the full rationale).
  const { date } = useDateTime();
  return (
    <EChart
      option={buildLineOption(widget, data, { area: true, formatX: date })}
      ariaLabel={widget.title}
    />
  );
}
