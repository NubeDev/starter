import type { Widget, WidgetData } from "@/data/types";
import { EChart } from "@/features/widgets/EChart";
import { buildGaugeOption } from "@/features/widgets/Gauge/gaugeOption";
import { useThemeStore } from "@/theme/store";

// Gauge panel — threshold-aware radial dial. Pure; the arc colour and
// value come from the option builder, which reads only typed props (F6).
export function Gauge({ widget, data }: { widget: Widget; data: WidgetData }) {
  // Re-render on dark/light switch so the theme-resolved arc/track
  // colours are rebuilt (see `Area` for the full rationale).
  useThemeStore((s) => s.mode);
  return (
    <EChart option={buildGaugeOption(widget, data)} ariaLabel={widget.title} />
  );
}
