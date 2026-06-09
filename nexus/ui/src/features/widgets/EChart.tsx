import { useEffect, useRef } from "react";
import * as echarts from "echarts";
import type { EChartsOption } from "echarts";

// The one place ECharts is mounted. Every panel builds an `EChartsOption`
// from its typed props and hands it here; this component owns the
// instance lifecycle (init, resize, dispose) and nothing else. Keeping
// the chart library behind a single wrapper is what lets a panel stay
// pure (F6) and what would make swapping the engine a one-file change.
export function EChart({
  option,
  className,
  ariaLabel,
}: {
  option: EChartsOption;
  className?: string;
  ariaLabel?: string;
}) {
  const elRef = useRef<HTMLDivElement | null>(null);
  const chartRef = useRef<echarts.ECharts | null>(null);

  useEffect(() => {
    const el = elRef.current;
    if (!el) return;
    const chart = echarts.init(el, undefined, { renderer: "canvas" });
    chartRef.current = chart;
    // ResizeObserver keeps the chart sized to its grid cell as panels are
    // dragged/resized on the canvas, without a window-level listener.
    const ro = new ResizeObserver(() => chart.resize());
    ro.observe(el);
    return () => {
      ro.disconnect();
      chart.dispose();
      chartRef.current = null;
    };
  }, []);

  useEffect(() => {
    // `notMerge: true` so a config change fully replaces the prior option
    // rather than layering onto stale series (e.g. when a panel's field
    // mapping changes).
    chartRef.current?.setOption(option, { notMerge: true });
  }, [option]);

  return (
    <div
      ref={elRef}
      role="img"
      aria-label={ariaLabel}
      className={className ?? "h-full w-full"}
    />
  );
}
