import { useEffect, useMemo, useRef, useState } from 'react'
import uPlot from 'uplot'
import 'uplot/dist/uPlot.min.css'
import {
  Activity,
  ArrowLeft,
  Gauge,
  LineChart,
  type LucideIcon,
  Maximize2,
  Zap,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

type RangeKey = '1H' | '24H' | '7D' | '30D'

const RANGE_POINTS: Record<RangeKey, number> = {
  '1H': 3_600,
  '24H': 28_800,
  '7D': 50_400,
  '30D': 86_400,
}

const RANGE_SECONDS: Record<RangeKey, number> = {
  '1H': 60 * 60,
  '24H': 24 * 60 * 60,
  '7D': 7 * 24 * 60 * 60,
  '30D': 30 * 24 * 60 * 60,
}

const START_TS = Math.floor(new Date('2026-05-26T00:00:00Z').getTime() / 1000)

function makeTelemetry(range: RangeKey): uPlot.AlignedData {
  const count = RANGE_POINTS[range]
  const seconds = RANGE_SECONDS[range]
  const step = seconds / (count - 1)
  const x = new Float64Array(count)
  const solar = new Float32Array(count)
  const demand = new Float32Array(count)
  const battery = new Float32Array(count)

  for (let i = 0; i < count; i += 1) {
    const t = i / Math.max(1, count - 1)
    const day = (t * seconds) / 86_400
    const intraday = day % 1
    const sunCurve = Math.max(0, Math.sin((intraday - 0.22) * Math.PI))
    const weekdayPulse = Math.sin(day * Math.PI * 2)
    const micro = Math.sin(i * 0.021) * 0.7 + Math.sin(i * 0.007) * 1.1
    const loadPulse =
      Math.exp(-Math.pow((intraday - 0.34) * 8, 2)) * 10 +
      Math.exp(-Math.pow((intraday - 0.78) * 9, 2)) * 15

    x[i] = START_TS - seconds + i * step
    solar[i] = Math.max(0, sunCurve * (56 + weekdayPulse * 5) + micro)
    demand[i] = 22 + loadPulse + Math.sin(i * 0.013) * 2.8 + Math.sin(day * 5.2) * 3
    battery[i] = 62 + Math.sin(day * 2.4 - 0.6) * 21 + sunCurve * 11 - loadPulse * 0.38
  }

  return [x, solar, demand, battery]
}

function numberValue(value: number | null | undefined, suffix = '') {
  if (value == null || Number.isNaN(value)) return '—'
  return `${value.toLocaleString(undefined, { maximumFractionDigits: 1 })}${suffix}`
}

function maxValue(values: ArrayLike<number>) {
  let max = Number.NEGATIVE_INFINITY
  for (let i = 0; i < values.length; i += 1) {
    max = Math.max(max, values[i])
  }
  return max
}

function UplotTelemetryChart({
  data,
  range,
  compact = false,
}: {
  data: uPlot.AlignedData
  range: RangeKey
  compact?: boolean
}) {
  const hostRef = useRef<HTMLDivElement>(null)
  const plotRef = useRef<uPlot | null>(null)

  useEffect(() => {
    const host = hostRef.current
    if (!host) return

    const makeGradient = (top: string, bottom: string) => (self: uPlot) => {
      const gradient = self.ctx.createLinearGradient(0, self.bbox.top, 0, self.bbox.top + self.bbox.height)
      gradient.addColorStop(0, top)
      gradient.addColorStop(1, bottom)
      return gradient
    }

    const opts: uPlot.Options = {
      width: Math.max(320, host.clientWidth),
      height: compact ? 196 : 430,
      class: 'beautiful-uplot',
      padding: [16, 16, 8, 8],
      cursor: {
        drag: { x: true, y: false },
        points: { size: compact ? 6 : 8, width: 2 },
        focus: { prox: 32 },
      },
      legend: { show: false },
      scales: {
        x: { time: true },
        kwh: {
          range: (_self, min, max) => [Math.max(0, min - 6), max + 10],
        },
        pct: { range: [0, 100] },
      },
      axes: [
        {
          stroke: 'rgba(167, 208, 189, 0.58)',
          grid: { show: true, stroke: 'rgba(167, 208, 189, 0.08)', width: 1 },
          ticks: { show: true, stroke: 'rgba(167, 208, 189, 0.13)', width: 1, size: 6 },
          size: compact ? 28 : 36,
        },
        {
          scale: 'kwh',
          stroke: 'rgba(167, 208, 189, 0.62)',
          grid: { show: true, stroke: 'rgba(167, 208, 189, 0.07)', width: 1 },
          ticks: { show: true, stroke: 'rgba(167, 208, 189, 0.13)', width: 1, size: 6 },
          size: compact ? 42 : 54,
          values: (_self, vals) => vals.map((v) => `${v}`),
        },
        {
          scale: 'pct',
          side: 1,
          stroke: 'rgba(253, 230, 138, 0.62)',
          grid: { show: false },
          ticks: { show: true, stroke: 'rgba(253, 230, 138, 0.16)', width: 1, size: 6 },
          size: compact ? 34 : 46,
          values: (_self, vals) => vals.map((v) => `${v}%`),
        },
      ],
      series: [
        {
          label: 'Time',
          value: (_self, raw) =>
            new Date(raw * 1000).toLocaleString(undefined, {
              month: 'short',
              day: 'numeric',
              hour: 'numeric',
              minute: '2-digit',
            }),
        },
        {
          label: 'Solar',
          scale: 'kwh',
          stroke: '#4ade80',
          fill: makeGradient('rgba(74, 222, 128, 0.24)', 'rgba(74, 222, 128, 0)'),
          width: compact ? 1.4 : 2,
          points: { show: false },
          value: (_self, raw) => numberValue(raw, ' kWh'),
        },
        {
          label: 'Demand',
          scale: 'kwh',
          stroke: '#67e8f9',
          fill: makeGradient('rgba(103, 232, 249, 0.16)', 'rgba(103, 232, 249, 0)'),
          width: compact ? 1.2 : 1.8,
          points: { show: false },
          value: (_self, raw) => numberValue(raw, ' kWh'),
        },
        {
          label: 'Battery',
          scale: 'pct',
          stroke: '#fde68a',
          width: compact ? 1.2 : 1.8,
          dash: [8, 5],
          points: { show: false },
          value: (_self, raw) => numberValue(raw, '%'),
        },
      ],
    }

    const plot = new uPlot(opts, data, host)
    plotRef.current = plot

    const resizeObserver = new ResizeObserver(([entry]) => {
      const width = Math.max(320, Math.floor(entry.contentRect.width))
      plot.setSize({ width, height: compact ? 196 : 430 })
    })
    resizeObserver.observe(host)

    return () => {
      resizeObserver.disconnect()
      plot.destroy()
      plotRef.current = null
    }
  }, [compact])

  useEffect(() => {
    plotRef.current?.setData(data, true)
  }, [data, range])

  return <div ref={hostRef} className="min-h-[196px] w-full" />
}

function Metric({
  icon: Icon,
  label,
  value,
  sub,
  accent,
}: {
  icon: LucideIcon
  label: string
  value: string
  sub: string
  accent: 'leaf' | 'aqua' | 'sun'
}) {
  const color =
    accent === 'aqua'
      ? 'var(--color-aqua)'
      : accent === 'sun'
        ? 'var(--color-sun)'
        : 'var(--color-leaf)'

  return (
    <div className="glass rounded-2xl p-4">
      <div className="mb-4 flex items-center justify-between">
        <span className="text-xs font-medium uppercase tracking-[0.16em] text-[color:var(--color-subtle)]">
          {label}
        </span>
        <Icon className="h-4 w-4" style={{ color }} />
      </div>
      <div className="tabular text-2xl font-semibold tracking-tight text-white">{value}</div>
      <div className="mt-1 text-xs text-[color:var(--color-muted)]">{sub}</div>
    </div>
  )
}

export function UplotDemoPage() {
  const [range, setRange] = useState<RangeKey>('7D')
  const data = useMemo(() => makeTelemetry(range), [range])
  const solar = data[1] as Float32Array
  const demand = data[2] as Float32Array
  const battery = data[3] as Float32Array
  const last = solar.length - 1
  const peakSolar = maxValue(solar)
  const avgDemand = demand.reduce((sum, value) => sum + value, 0) / demand.length

  return (
    <main className="min-h-screen overflow-hidden bg-[color:var(--color-bg)] text-white">
      <div className="mx-auto flex min-h-screen max-w-7xl flex-col px-4 py-5 sm:px-6 lg:px-8">
        <header className="flex flex-wrap items-center justify-between gap-4 border-b border-white/5 pb-5">
          <a
            href="/"
            className="inline-flex items-center gap-2 text-sm font-medium text-[color:var(--color-muted)] transition-colors hover:text-white"
          >
            <ArrowLeft className="h-4 w-4" />
            Verdant dashboard
          </a>
          <div className="flex items-center gap-2 rounded-full border border-white/10 bg-white/[0.04] p-1">
            {(Object.keys(RANGE_POINTS) as RangeKey[]).map((key) => (
              <button
                key={key}
                onClick={() => setRange(key)}
                className={cn(
                  'h-8 rounded-full px-3 text-xs font-semibold transition-colors',
                  range === key
                    ? 'bg-white text-zinc-950'
                    : 'text-[color:var(--color-muted)] hover:bg-white/[0.06] hover:text-white',
                )}
              >
                {key}
              </button>
            ))}
          </div>
        </header>

        <section className="grid flex-1 items-center gap-8 py-10 lg:grid-cols-[minmax(0,1fr)_320px]">
          <div>
            <div className="mb-8 max-w-3xl">
              <div className="mb-4 inline-flex items-center gap-2 rounded-full border border-[color:var(--color-leaf)]/20 bg-[color:var(--color-leaf)]/10 px-3 py-1 text-xs font-semibold uppercase tracking-[0.16em] text-[color:var(--color-leaf)]">
                <LineChart className="h-3.5 w-3.5" />
                uPlot high density demo
              </div>
              <h1 className="text-5xl font-medium leading-[1.02] tracking-[-0.04em] text-white sm:text-6xl">
                Fast charts can still look like they belong here.
              </h1>
              <p className="mt-5 max-w-2xl text-base leading-relaxed text-[color:var(--color-muted)]">
                This page renders {data[0].length.toLocaleString()} aligned points with uPlot, using a styled canvas chart, quiet axes, custom range controls, and dashboard metrics around it.
              </p>
            </div>

            <div className="glass relative overflow-hidden rounded-[2rem] p-4 sm:p-5">
              <div className="mb-4 flex flex-wrap items-start justify-between gap-4 px-1">
                <div>
                  <div className="text-xs font-medium uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
                    Home energy stream
                  </div>
                  <div className="mt-1 flex flex-wrap items-baseline gap-x-3 gap-y-1">
                    <span className="tabular text-3xl font-semibold tracking-tight text-white">
                      {numberValue(solar[last], ' kWh')}
                    </span>
                    <span className="text-sm text-[color:var(--color-leaf)]">live solar input</span>
                  </div>
                </div>
                <div className="grid grid-cols-3 gap-3 text-xs">
                  {[
                    ['Solar', 'var(--color-leaf)'],
                    ['Demand', 'var(--color-aqua)'],
                    ['Battery', 'var(--color-sun)'],
                  ].map(([label, color]) => (
                    <div key={label} className="flex items-center gap-2 text-[color:var(--color-muted)]">
                      <span className="h-2 w-2 rounded-full" style={{ background: color }} />
                      {label}
                    </div>
                  ))}
                </div>
              </div>
              <UplotTelemetryChart data={data} range={range} />
            </div>
          </div>

          <aside className="grid gap-4">
            <Metric
              icon={Zap}
              label="Peak solar"
              value={numberValue(peakSolar, ' kWh')}
              sub="Canvas-drawn line and area fill"
              accent="leaf"
            />
            <Metric
              icon={Activity}
              label="Avg demand"
              value={numberValue(avgDemand, ' kWh')}
              sub="No SVG point storm"
              accent="aqua"
            />
            <Metric
              icon={Gauge}
              label="Battery"
              value={numberValue(battery[last], '%')}
              sub="Separate right-side scale"
              accent="sun"
            />
            <div className="glass rounded-2xl p-4">
              <div className="mb-3 flex items-center justify-between">
                <div className="text-xs font-medium uppercase tracking-[0.16em] text-[color:var(--color-subtle)]">
                  Mini stream
                </div>
                <Maximize2 className="h-4 w-4 text-[color:var(--color-muted)]" />
              </div>
              <UplotTelemetryChart data={data} range={range} compact />
            </div>
            <Button variant="leaf" className="w-full" onClick={() => setRange('30D')}>
              Stress 86,400 points
              <LineChart className="h-4 w-4" />
            </Button>
          </aside>
        </section>
      </div>
    </main>
  )
}
