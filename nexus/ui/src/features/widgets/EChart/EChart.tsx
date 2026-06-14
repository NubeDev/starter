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
  // The series count of the option last applied. A periodic data refresh
  // keeps this constant (only the points change); a structural change (field
  // mapping, widget deletion) changes it. We use it to decide whether to do a
  // cheap in-place data update or a full teardown — see the update effect.
  const prevSeriesCountRef = useRef<number | null>(null);

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
    const chart = chartRef.current;
    if (!chart) return;

    const seriesCount = Array.isArray(option.series)
      ? option.series.length
      : option.series
        ? 1
        : 0;
    const prevCount = prevSeriesCountRef.current;
    prevSeriesCountRef.current = seriesCount;

    // Structural change — the series *count* differs (a panel re-renders with
    // a different field mapping, a widget is deleted, the option is rebuilt on
    // a date-format change). Here we must tear down: without `clear()` ECharts
    // animates the new series against a missing old one and throws in
    // `interpolate1DArray` (reading `.length` of `undefined`); `notMerge` then
    // fully replaces the option rather than layering onto stale series.
    if (prevCount === null || prevCount !== seriesCount) {
      chart.clear();
      chart.setOption(option, { notMerge: true });
      return;
    }

    // Same series count — this is the common case of a periodic refresh where
    // only the data points changed. Merge the new option in place: ECharts
    // diffs the data and transitions smoothly instead of replaying the entry
    // animation (the line redrawing itself, bars growing from zero) that a
    // `clear()` + `notMerge` rebuild causes every tick.
    chart.setOption(option);
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
