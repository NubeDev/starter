// `uplot-chart.tsx` — thin React wrapper around µPlot.
//
// µPlot is a tiny, dependency-free, canvas-based time-series chart
// (~50KB). Exactly the right tool for rendering multi-series BMS
// data without pulling in d3 or recharts. The wrapper handles:
//
//  - mount/unmount lifecycle (creates the chart in a ref'd div),
//  - resize-on-container-resize via ResizeObserver,
//  - re-render when the data array identity changes,
//  - dark-mode-aware default axis/grid colours read off the
//    `useHostTheme()` token (passed in by the caller).

import * as React from "react";
import uPlot, { type AlignedData, type Options } from "uplot";
import "uplot/dist/uPlot.min.css";

export interface UplotChartProps {
  /** Data in µPlot's "aligned" format: `[xs, ys1, ys2, ...]`. */
  data: AlignedData;
  /** Chart options excluding `width`; width is derived from the
   *  container. Height is read from `opts.height` (default 240). */
  opts: Omit<Options, "width">;
  /** Force re-build when the series schema (legend/colours) changes;
   *  pass a stable key (e.g. the number of series). Data changes
   *  alone are diffed via `setData()` without rebuild. */
  schemaKey: string;
  className?: string;
}

export function UplotChart({
  data,
  opts,
  schemaKey,
  className,
}: UplotChartProps): React.ReactElement {
  const hostRef = React.useRef<HTMLDivElement | null>(null);
  const chartRef = React.useRef<uPlot | null>(null);
  const dataRef = React.useRef<AlignedData>(data);
  dataRef.current = data;

  // (Re)build the chart whenever the series schema changes, the
  // container mounts, or the option shape changes meaningfully.
  React.useEffect(() => {
    const host = hostRef.current;
    if (!host) return;
    const width = host.clientWidth || 600;
    const height = (opts as { height?: number }).height ?? 240;
    const chart = new uPlot({ ...opts, width, height } as Options, dataRef.current, host);
    chartRef.current = chart;

    const ro = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;
      const w = Math.max(120, Math.round(entry.contentRect.width));
      const h = (opts as { height?: number }).height ?? 240;
      chart.setSize({ width: w, height: h });
    });
    ro.observe(host);

    return () => {
      ro.disconnect();
      chart.destroy();
      chartRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [schemaKey]);

  // Hot-swap data without rebuilding.
  React.useEffect(() => {
    const chart = chartRef.current;
    if (chart) chart.setData(data);
  }, [data]);

  return <div ref={hostRef} className={className} />;
}
