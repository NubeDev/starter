import { useEffect, useLayoutEffect, useMemo, useRef } from "react"
import uPlot, { type AlignedData, type Options } from "uplot"
import "uplot/dist/uPlot.min.css"

import type { CacheDemoMetricSeries } from "@/lib/api"

import { humanise } from "./shared"

/// Canvas-backed line chart powered by uPlot. Renders `series.points`
/// as three lines (min/max faded band + avg) on a single `<canvas>`,
/// which scales to tens of thousands of points without choking the
/// main thread.
///
/// `onRender` fires once per real render with the wall-clock time
/// uPlot took to draw — used by the demo page to show the renderer
/// comparison.
export function UPlotMetricChart({
  series,
  onRender,
}: {
  series: CacheDemoMetricSeries
  onRender?: (ms: number) => void
}) {
  const wrapRef = useRef<HTMLDivElement | null>(null)
  const plotRef = useRef<uPlot | null>(null)

  // uPlot wants columnar data: [xs, y1, y2, y3]. The x-axis is a
  // unix timestamp (seconds) per bucket, derived from the RFC3339
  // sample timestamp.
  const data = useMemo<AlignedData>(() => {
    const xs = new Float64Array(series.points.length)
    const avg = new Float64Array(series.points.length)
    const min = new Float64Array(series.points.length)
    const max = new Float64Array(series.points.length)
    for (let i = 0; i < series.points.length; i += 1) {
      const p = series.points[i]!
      xs[i] = new Date(p.t).getTime() / 1000
      avg[i] = p.avg
      min[i] = p.min
      max[i] = p.max
    }
    return [xs, avg, min, max] as AlignedData
  }, [series])

  const opts = useMemo<Omit<Options, "width" | "height">>(() => {
    const faded = series.color + "55"
    return {
      // Padded so axis labels don't get clipped against the card.
      padding: [8, 12, 8, 8],
      scales: { x: { time: true } },
      legend: { show: false },
      cursor: {
        drag: { x: true, y: false, setScale: false },
        focus: { prox: 16 },
      },
      axes: [
        {
          stroke: "var(--muted-foreground)",
          grid: { stroke: "var(--border)", width: 1 },
          ticks: { stroke: "var(--border)", width: 1 },
          font: "10px ui-sans-serif, system-ui, sans-serif",
        },
        {
          stroke: "var(--muted-foreground)",
          grid: { stroke: "var(--border)", width: 1 },
          ticks: { stroke: "var(--border)", width: 1 },
          font: "10px ui-sans-serif, system-ui, sans-serif",
          size: 48,
        },
      ],
      series: [
        {}, // x
        {
          label: "avg",
          stroke: series.color,
          width: 1.5,
          points: { show: false },
        },
        {
          label: "min",
          stroke: faded,
          width: 1,
          points: { show: false },
        },
        {
          label: "max",
          stroke: faded,
          width: 1,
          points: { show: false },
        },
      ],
    }
  }, [series.color])

  // Synchronous create/update — measure the wall-clock time uPlot
  // takes so the demo can show it side-by-side with Recharts.
  useLayoutEffect(() => {
    const wrap = wrapRef.current
    if (!wrap) return
    const width = wrap.clientWidth || 600
    const height = wrap.clientHeight || 260

    const t0 = performance.now()
    if (plotRef.current) {
      plotRef.current.setSize({ width, height })
      plotRef.current.setData(data)
    } else {
      plotRef.current = new uPlot(
        { ...opts, width, height } as Options,
        data,
        wrap,
      )
    }
    const ms = performance.now() - t0
    onRender?.(ms)
  }, [data, opts, onRender])

  // Keep the plot in sync with container resizes.
  useEffect(() => {
    const wrap = wrapRef.current
    if (!wrap) return
    const ro = new ResizeObserver(() => {
      if (!plotRef.current || !wrap) return
      plotRef.current.setSize({
        width: wrap.clientWidth,
        height: wrap.clientHeight,
      })
    })
    ro.observe(wrap)
    return () => ro.disconnect()
  }, [])

  // Clean up on unmount.
  useEffect(
    () => () => {
      plotRef.current?.destroy()
      plotRef.current = null
    },
    [],
  )

  return (
    <div className="rounded-md border bg-card p-3">
      <div className="mb-2 flex items-center justify-between">
        <h4 className="text-sm font-medium">
          {humanise(series.name)}{" "}
          <span className="text-xs text-muted-foreground">
            · {series.unit}
          </span>
        </h4>
        <span
          className="inline-block size-2 rounded-full"
          style={{ background: series.color }}
        />
      </div>
      <div ref={wrapRef} className="h-[260px] w-full" />
    </div>
  )
}
