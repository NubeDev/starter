import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react"
import { useSearchParams } from "react-router-dom"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import {
  IconBolt,
  IconDatabaseOff,
  IconPlayerPlay,
  IconRefresh,
} from "@tabler/icons-react"
import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Line,
  LineChart,
  Legend,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts"

import { UPlotMetricChart } from "./cache-demo/UPlotMetricChart"
import { humanise } from "./cache-demo/shared"

import { PageHero } from "@/components/page-hero"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  api,
  type CacheDemoBucket,
  type CacheDemoMetricSeries,
  type CacheDemoSeries,
  type CacheDemoStats,
} from "@/lib/api"

const BUCKETS: CacheDemoBucket[] = ["1m", "5m", "15m", "30m", "60m"]
const POINTS_OPTIONS = [1_000, 5_000, 10_000, 25_000, 50_000]
const DEFAULT_BUCKET: CacheDemoBucket = "5m"
const DEFAULT_POINTS = 5_000
const BENCH_FETCHES = 10
const HISTORY_LIMIT = 50

/// One recorded load: round-trip wall time, whether the server
/// served it from cache, server-side cold-gen cost, and response
/// size. Used by the benchmark widget and the rolling history chart.
type LoadSample = {
  i: number
  ms: number
  from_cache: boolean
  gen_ms: number
  bytes: number
  kind: "natural" | "bench"
}

/// Which chart library renders the time-series grid. Both are wired
/// up so the demo can show the SVG-vs-canvas perf gap honestly.
type Renderer = "recharts" | "uplot"
const RENDERERS: Renderer[] = ["uplot", "recharts"]
const RENDERER_STORAGE_KEY = "cache-demo:renderer:v1"

/// Initial renderer seed, used only when the URL has no `renderer`
/// param. Reads localStorage so the operator's last choice survives
/// a fresh navigation to `/cache-demo` (no query string).
function initialRenderer(): Renderer {
  if (typeof window === "undefined") return "uplot"
  const stored = window.localStorage.getItem(RENDERER_STORAGE_KEY)
  return stored === "recharts" ? "recharts" : "uplot"
}

/// Cache demo page (`/cache-demo?bucket=5m&points=5000`).
///
/// Generates 4 deterministic metric series on the backend (real CPU
/// work — synth + moving-average + bucket aggregation + JSON
/// serialisation). `starter-cache` (moka) caches the full response;
/// subsequent loads with the same `{bucket, points}` key are warm.
///
/// Selections are mirrored to URL search params so the page is
/// deep-linkable and reload-stable.
export function CacheDemo() {
  const qc = useQueryClient()
  const [searchParams, setSearchParams] = useSearchParams()
  const bucket = parseBucket(searchParams.get("bucket"))
  const points = parsePoints(searchParams.get("points"))
  const renderer = parseRenderer(searchParams.get("renderer"), initialRenderer())
  const [clientFetchMs, setClientFetchMs] = useState<number | null>(null)
  const [history, setHistory] = useState<LoadSample[]>(() => loadHistory())
  const [benchRunning, setBenchRunning] = useState(false)
  const sampleIdRef = useRef(
    history.reduce((max, s) => Math.max(max, s.i), 0),
  )

  const setRenderer = (v: Renderer) => {
    const next = new URLSearchParams(searchParams)
    next.set("renderer", v)
    setSearchParams(next, { replace: true })
    try {
      window.localStorage.setItem(RENDERER_STORAGE_KEY, v)
    } catch {
      /* ignore */
    }
  }

  // Render-time map keyed by metric series name. The chart components
  // call `onChartRender(name, ms)` from a useLayoutEffect → rAF so
  // we capture commit + paint for both renderers consistently.
  const [renderTimes, setRenderTimes] = useState<Record<string, number>>({})
  const onChartRender = (chart: string, ms: number) => {
    setRenderTimes((prev) =>
      prev[chart] === ms ? prev : { ...prev, [chart]: ms },
    )
  }
  // Reset the render-time map when the renderer or data shape
  // changes so stale numbers don't linger.
  useEffect(() => {
    setRenderTimes({})
  }, [renderer, bucket, points])

  // Persist history across browser reloads so the chart shows the
  // accumulated load timeline, not just the current session.
  useEffect(() => {
    saveHistory(history)
  }, [history])

  const pushSample = (s: Omit<LoadSample, "i">) => {
    sampleIdRef.current += 1
    setHistory((prev) => {
      const next = [...prev, { ...s, i: sampleIdRef.current }]
      return next.length > HISTORY_LIMIT
        ? next.slice(next.length - HISTORY_LIMIT)
        : next
    })
  }

  // Normalise the URL once on mount: if a param was missing or
  // invalid, write the defaults back so the URL always reflects the
  // resolved state.
  useEffect(() => {
    const haveBucket = searchParams.get("bucket")
    const havePoints = searchParams.get("points")
    const haveRenderer = searchParams.get("renderer")
    if (
      haveBucket !== bucket ||
      havePoints !== String(points) ||
      haveRenderer !== renderer
    ) {
      const next = new URLSearchParams(searchParams)
      next.set("bucket", bucket)
      next.set("points", String(points))
      next.set("renderer", renderer)
      setSearchParams(next, { replace: true })
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const setBucket = (v: CacheDemoBucket) => {
    const next = new URLSearchParams(searchParams)
    next.set("bucket", v)
    setSearchParams(next, { replace: true })
  }
  const setPoints = (n: number) => {
    const next = new URLSearchParams(searchParams)
    next.set("points", String(n))
    setSearchParams(next, { replace: true })
  }

  const series = useQuery({
    queryKey: ["cache-demo", "series", bucket, points],
    queryFn: async () => {
      const t0 = performance.now()
      try {
        const data = await api.cacheDemo.series({ bucket, points })
        const ms = Math.round(performance.now() - t0)
        setClientFetchMs(ms)
        pushSample({
          ms,
          from_cache: data.from_cache,
          gen_ms: data.generated_in_ms,
          bytes: estimateBytes(data),
          kind: "natural",
        })
        return data
      } catch (e) {
        setClientFetchMs(Math.round(performance.now() - t0))
        throw e
      }
    },
    // Don't let react-query mask cold/warm differences by serving
    // a stale in-memory snapshot \u2014 every mount/refocus should hit
    // the backend so the cache demo is honest.
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
  })

  const stats = useQuery({
    queryKey: ["cache-demo", "stats"],
    queryFn: () => api.cacheDemo.stats(),
    refetchInterval: 1500,
  })

  const clear = useMutation({
    mutationFn: () => api.cacheDemo.clear(),
    onSuccess: async () => {
      setClientFetchMs(null)
      await Promise.all([
        qc.invalidateQueries({ queryKey: ["cache-demo", "stats"] }),
        qc.invalidateQueries({ queryKey: ["cache-demo", "series"] }),
      ])
    },
  })

  /// Run a synchronous N-fetch benchmark that bypasses react-query
  /// (raw `fetch`) so the timings reflect the real network + server
  /// + JSON-parse cost. First clears the backend cache so fetch #1
  /// is a true cold load and fetches #2..N are warm.
  const runBenchmark = async () => {
    if (benchRunning) return
    setBenchRunning(true)
    try {
      // Drop bench history from previous runs so the chart is clean.
      setHistory((prev) => prev.filter((s) => s.kind !== "bench"))
      await api.cacheDemo.clear()
      const url = `/api/cache-demo/series?bucket=${bucket}&points=${points}`
      for (let n = 0; n < BENCH_FETCHES; n += 1) {
        const t0 = performance.now()
        const res = await fetch(url, { cache: "no-store" })
        const text = await res.text()
        const ms = Math.round(performance.now() - t0)
        const body = JSON.parse(text) as CacheDemoSeries
        pushSample({
          ms,
          from_cache: body.from_cache,
          gen_ms: body.generated_in_ms,
          bytes: text.length,
          kind: "bench",
        })
      }
      await qc.invalidateQueries({ queryKey: ["cache-demo", "stats"] })
    } finally {
      setBenchRunning(false)
    }
  }

  const totalPoints = series.data
    ? series.data.series.reduce((acc, s) => acc + s.points.length, 0)
    : 0

  return (
    <div className="flex flex-col gap-6 px-4 py-6 lg:px-6">
      <PageHero
        icon={IconBolt}
        accent="var(--accent-warning, var(--accent-info))"
        title="Cache demo · starter-cache"
        description="Four deterministic metric series rendered on the backend, cached in-process by moka. First load runs honest work (synth + smooth + aggregate + serialise); subsequent loads come straight from the cache."
        actions={
          <Badge variant="secondary">
            {series.data
              ? `${series.data.series.length} series · ${totalPoints.toLocaleString()} pts`
              : "—"}
          </Badge>
        }
      />

      <StatsCard
        stats={stats.data}
        clientFetchMs={clientFetchMs}
        lastFromCache={series.data?.from_cache ?? null}
        onClear={() => clear.mutate()}
        clearing={clear.isPending}
      />

      <BenchmarkCard
        history={history}
        running={benchRunning}
        onRun={runBenchmark}
        onClear={() => setHistory([])}
      />

      <Card>
        <CardHeader className="flex flex-row items-end justify-between gap-4">
          <div>
            <CardTitle className="text-base">Time-series</CardTitle>
            <CardDescription>
              <code className="font-mono text-xs">{`/cache-demo?bucket=${bucket}&points=${points}`}</code>{" "}
              · changing the picker rewrites the URL and forms a new
              cache key.
            </CardDescription>
          </div>
          <div className="flex items-center gap-2">
            <RendererToggle value={renderer} onChange={setRenderer} />
            <BucketPicker value={bucket} onChange={setBucket} />
            <PointsPicker value={points} onChange={setPoints} />
          </div>
        </CardHeader>
        <CardContent>
          <ChartGrid
            series={series.data}
            loading={series.isLoading}
            renderer={renderer}
            onRender={onChartRender}
          />
          {series.data ? (
            <div className="mt-3 flex flex-wrap items-center justify-between gap-3 text-xs text-muted-foreground">
              <span>
                {series.data.series.length} series ·{" "}
                {totalPoints.toLocaleString()} rendered points from{" "}
                {series.data.raw_points.toLocaleString()} raw samples ·
                cold load generated in {series.data.generated_in_ms} ms ·
                this request{" "}
                {series.data.from_cache ? (
                  <span className="font-semibold text-emerald-600">
                    served from cache
                  </span>
                ) : (
                  <span className="font-semibold text-amber-600">cold</span>
                )}
              </span>
              <RenderTimesBadge
                renderer={renderer}
                times={renderTimes}
              />
            </div>
          ) : null}
        </CardContent>
      </Card>
    </div>
  )
}

function StatsCard({
  stats,
  clientFetchMs,
  lastFromCache,
  onClear,
  clearing,
}: {
  stats: CacheDemoStats | undefined
  clientFetchMs: number | null
  lastFromCache: boolean | null
  onClear: () => void
  clearing: boolean
}) {
  const hitRatioPct = stats ? Math.round(stats.hit_ratio * 100) : 0
  return (
    <Card className="border-dashed">
      <CardHeader className="flex flex-row items-start justify-between gap-4">
        <div>
          <CardTitle className="flex items-center gap-2 text-base">
            <IconBolt className="size-4 text-amber-500" />
            Cache stats · backend{" "}
            <code className="font-mono text-xs">
              {stats?.backend ?? "moka"}
            </code>
          </CardTitle>
          <CardDescription>
            Updates every 1.5s. Click <strong>Clear cache</strong> to
            reset counters and drop entries.
          </CardDescription>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={onClear}
          disabled={clearing}
        >
          {clearing ? (
            <IconRefresh className="mr-1 size-4 animate-spin" />
          ) : (
            <IconDatabaseOff className="mr-1 size-4" />
          )}
          Clear cache
        </Button>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
          <Stat label="Hits" value={stats?.hits ?? 0} tone="positive" />
          <Stat label="Misses" value={stats?.misses ?? 0} tone="warning" />
          <Stat
            label="Hit ratio"
            value={`${hitRatioPct}%`}
            tone={hitRatioPct >= 50 ? "positive" : "muted"}
          />
          <Stat label="Entries" value={stats?.entries ?? 0} />
          <Stat
            label="Last cold load"
            value={formatMs(stats?.last_cold_load_ms ?? 0)}
            tone="warning"
          />
          <Stat
            label="Client last fetch"
            value={
              clientFetchMs == null ? "—" : formatMs(clientFetchMs)
            }
            tone={lastFromCache ? "positive" : "default"}
          />
        </div>
      </CardContent>
    </Card>
  )
}

function ChartGrid({
  series,
  loading,
  renderer,
  onRender,
}: {
  series: CacheDemoSeries | undefined
  loading: boolean
  renderer: Renderer
  onRender: (chart: string, ms: number) => void
}) {
  if (loading) {
    return (
      <div className="flex h-[420px] items-center justify-center rounded-md border border-dashed text-sm text-muted-foreground">
        Rendering series… (cold path runs real CPU work, no sleeps)
      </div>
    )
  }
  if (!series || series.series.length === 0) {
    return (
      <div className="flex h-[420px] items-center justify-center rounded-md border border-dashed text-sm text-muted-foreground">
        No data.
      </div>
    )
  }
  return (
    <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
      {series.series.map((s) =>
        renderer === "uplot" ? (
          <UPlotMetricChart
            key={`up-${s.name}`}
            series={s}
            onRender={(ms) => onRender(s.name, ms)}
          />
        ) : (
          <MetricChart
            key={`rc-${s.name}`}
            series={s}
            onRender={(ms) => onRender(s.name, ms)}
          />
        ),
      )}
    </div>
  )
}

function MetricChart({
  series,
  onRender,
}: {
  series: CacheDemoMetricSeries
  onRender?: (ms: number) => void
}) {
  const data = useMemo(
    () =>
      series.points.map((p) => ({
        label: p.t.slice(5, 16).replace("T", " "),
        avg: p.avg,
        min: p.min,
        max: p.max,
      })),
    [series],
  )
  const fadedColor = series.color + "55" // ~33% alpha for min/max

  // Measure render → next paint (rAF) so the comparison vs uPlot is
  // honest. Recharts commits SVG synchronously inside React's commit
  // phase but the browser still has to layout & paint thousands of
  // <path> nodes — rAF catches that.
  const t0Ref = useRef<number>(0)
  t0Ref.current = performance.now()
  useLayoutEffect(() => {
    const t0 = t0Ref.current
    const raf = requestAnimationFrame(() => {
      onRender?.(performance.now() - t0)
    })
    return () => cancelAnimationFrame(raf)
  }, [data, onRender])

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
      <div className="h-[260px] w-full">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart
            data={data}
            margin={{ top: 4, right: 12, bottom: 4, left: 0 }}
          >
            <CartesianGrid strokeDasharray="3 3" opacity={0.4} />
            <XAxis
              dataKey="label"
              minTickGap={48}
              tick={{ fontSize: 10 }}
            />
            <YAxis tick={{ fontSize: 10 }} width={48} />
            <Tooltip
              formatter={(v) =>
                typeof v === "number" ? v.toFixed(2) : String(v)
              }
              contentStyle={{ fontSize: 12 }}
            />
            <Line
              type="monotone"
              dataKey="avg"
              stroke={series.color}
              dot={false}
              strokeWidth={1.5}
              isAnimationActive={false}
            />
            <Line
              type="monotone"
              dataKey="min"
              stroke={fadedColor}
              dot={false}
              strokeWidth={1}
              isAnimationActive={false}
            />
            <Line
              type="monotone"
              dataKey="max"
              stroke={fadedColor}
              dot={false}
              strokeWidth={1}
              isAnimationActive={false}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  )
}

function RendererToggle({
  value,
  onChange,
}: {
  value: Renderer
  onChange: (v: Renderer) => void
}) {
  return (
    <div className="inline-flex rounded-md border bg-card p-0.5 text-xs">
      <button
        type="button"
        onClick={() => onChange("uplot")}
        className={
          "rounded-sm px-2 py-1 font-medium transition " +
          (value === "uplot"
            ? "bg-primary text-primary-foreground"
            : "text-muted-foreground hover:text-foreground")
        }
        title="Canvas (fast)"
      >
        uPlot
      </button>
      <button
        type="button"
        onClick={() => onChange("recharts")}
        className={
          "rounded-sm px-2 py-1 font-medium transition " +
          (value === "recharts"
            ? "bg-primary text-primary-foreground"
            : "text-muted-foreground hover:text-foreground")
        }
        title="SVG (slow on big data)"
      >
        Recharts
      </button>
    </div>
  )
}

function RenderTimesBadge({
  renderer,
  times,
}: {
  renderer: Renderer
  times: Record<string, number>
}) {
  const values = Object.values(times)
  if (values.length === 0) {
    return (
      <Badge variant="outline" className="font-mono text-[10px]">
        {renderer} · measuring…
      </Badge>
    )
  }
  const total = values.reduce((a, b) => a + b, 0)
  const max = Math.max(...values)
  const fast = renderer === "uplot"
  return (
    <Badge
      variant={fast ? "default" : "destructive"}
      className="font-mono text-[10px]"
      title={`${values.length} chart(s) measured · slowest ${formatMs(Math.round(max))}`}
    >
      {renderer} render · {formatMs(Math.round(total))} total ·{" "}
      {formatMs(Math.round(max))} slowest
    </Badge>
  )
}

function BucketPicker({
  value,
  onChange,
}: {
  value: CacheDemoBucket
  onChange: (v: CacheDemoBucket) => void
}) {
  return (
    <Select value={value} onValueChange={(v) => onChange(v as CacheDemoBucket)}>
      <SelectTrigger className="w-24">
        <SelectValue placeholder="bucket" />
      </SelectTrigger>
      <SelectContent>
        {BUCKETS.map((b) => (
          <SelectItem key={b} value={b}>
            {b}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}

function PointsPicker({
  value,
  onChange,
}: {
  value: number
  onChange: (n: number) => void
}) {
  return (
    <Select value={String(value)} onValueChange={(v) => onChange(Number(v))}>
      <SelectTrigger className="w-32">
        <SelectValue placeholder="points" />
      </SelectTrigger>
      <SelectContent>
        {POINTS_OPTIONS.map((n) => (
          <SelectItem key={n} value={String(n)}>
            {n.toLocaleString()} pts
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}

type StatTone = "positive" | "warning" | "muted" | "default"

function Stat({
  label,
  value,
  tone = "default",
}: {
  label: string
  value: number | string
  tone?: StatTone
}) {
  return (
    <div className="flex flex-col gap-1 rounded-md border bg-card p-3">
      <span className="text-[10px] uppercase tracking-wide text-muted-foreground">
        {label}
      </span>
      <span>
        <Badge
          variant={
            tone === "positive"
              ? "default"
              : tone === "warning"
                ? "destructive"
                : tone === "muted"
                  ? "outline"
                  : "secondary"
          }
        >
          <span className="font-mono">{value}</span>
        </Badge>
      </span>
    </div>
  )
}

function formatMs(ms: number): string {
  if (ms <= 0) return "0 ms"
  if (ms < 1000) return `${ms} ms`
  return `${(ms / 1000).toFixed(2)} s`
}

function parseBucket(raw: string | null): CacheDemoBucket {
  return (BUCKETS as string[]).includes(raw ?? "")
    ? (raw as CacheDemoBucket)
    : DEFAULT_BUCKET
}

function parsePoints(raw: string | null): number {
  const n = Number(raw)
  return POINTS_OPTIONS.includes(n) ? n : DEFAULT_POINTS
}

function parseRenderer(raw: string | null, fallback: Renderer): Renderer {
  return (RENDERERS as string[]).includes(raw ?? "")
    ? (raw as Renderer)
    : fallback
}

function estimateBytes(data: CacheDemoSeries): number {
  // Rough size of the JSON envelope. Cheap, good enough for the
  // history chart's tooltip — we don't want to re-serialise.
  return data.series.reduce((acc, s) => acc + s.points.length * 48, 0)
}

const HISTORY_STORAGE_KEY = "cache-demo:history:v1"

function loadHistory(): LoadSample[] {
  if (typeof window === "undefined") return []
  try {
    const raw = window.localStorage.getItem(HISTORY_STORAGE_KEY)
    if (!raw) return []
    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []
    // Trust the shape loosely — we only render numeric/string fields.
    return parsed.slice(-HISTORY_LIMIT) as LoadSample[]
  } catch {
    return []
  }
}

function saveHistory(history: LoadSample[]): void {
  if (typeof window === "undefined") return
  try {
    window.localStorage.setItem(
      HISTORY_STORAGE_KEY,
      JSON.stringify(history),
    )
  } catch {
    // localStorage may be full or disabled; the chart still works,
    // it just won't survive a reload.
  }
}


function BenchmarkCard({
  history,
  running,
  onRun,
  onClear,
}: {
  history: LoadSample[]
  running: boolean
  onRun: () => void
  onClear: () => void
}) {
  const bench = history.filter((s) => s.kind === "bench")
  const natural = history.filter((s) => s.kind === "natural")

  const cold = bench.filter((s) => !s.from_cache)
  const warm = bench.filter((s) => s.from_cache)
  const avg = (xs: LoadSample[]) =>
    xs.length === 0 ? 0 : xs.reduce((a, s) => a + s.ms, 0) / xs.length
  const avgCold = Math.round(avg(cold))
  const avgWarm = Math.round(avg(warm))
  const speedup =
    avgWarm > 0 && avgCold > 0 ? (avgCold / avgWarm).toFixed(1) : null

  const chartData = history.map((s) => ({
    label: s.kind === "bench" ? `#${s.i}` : `n${s.i}`,
    ms: s.ms,
    from_cache: s.from_cache,
    kind: s.kind,
  }))

  return (
    <Card>
      <CardHeader className="flex flex-row items-start justify-between gap-4">
        <div>
          <CardTitle className="text-base">Benchmark \u00b7 cold vs warm</CardTitle>
          <CardDescription>
            Clicking <strong>Run benchmark</strong> clears the server
            cache, then fires {BENCH_FETCHES} back-to-back requests via
            raw <code className="font-mono">fetch()</code> \u2014 react-query
            is bypassed so you see real round-trips. Fetch #1 is cold;
            #2\u2013#{BENCH_FETCHES} are warm.
          </CardDescription>
        </div>
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={onClear}
            disabled={running || history.length === 0}
          >
            Clear history
          </Button>
          <Button size="sm" onClick={onRun} disabled={running}>
            {running ? (
              <IconRefresh className="mr-1 size-4 animate-spin" />
            ) : (
              <IconPlayerPlay className="mr-1 size-4" />
            )}
            Run benchmark
          </Button>
        </div>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <p className="text-sm leading-relaxed">
          {bench.length === 0 ? (
            <span className="text-muted-foreground">
              No benchmark yet. {natural.length > 0 ? (
                <>
                  Natural page loads so far:{" "}
                  <strong>{natural.length}</strong> (most recent{" "}
                  <strong>{formatMs(natural[natural.length - 1]?.ms ?? 0)}</strong>
                  ).{" "}
                </>
              ) : null}
              Click <strong>Run benchmark</strong> for honest cold-vs-warm
              numbers.
            </span>
          ) : (
            <>
              Last benchmark: cold avg{" "}
              <strong className="text-amber-600">
                {formatMs(avgCold)}
              </strong>{" "}
              ({cold.length} fetch{cold.length === 1 ? "" : "es"}), warm
              avg{" "}
              <strong className="text-emerald-600">
                {formatMs(avgWarm)}
              </strong>{" "}
              ({warm.length} fetches)
              {speedup != null ? (
                <>
                  {" "}\u2014 cache speedup{" "}
                  <strong className="text-emerald-600">{speedup}\u00d7</strong>
                </>
              ) : null}
              . Server-side cold synth took{" "}
              <strong>
                {cold[0] ? formatMs(cold[0].gen_ms) : "\u2014"}
              </strong>{" "}
              of that round-trip; the rest is network + JSON parse.
            </>
          )}
        </p>

        <div className="h-[220px] w-full">
          {chartData.length === 0 ? (
            <div className="flex h-full items-center justify-center rounded-md border border-dashed text-sm text-muted-foreground">
              No data yet.
            </div>
          ) : (
            <ResponsiveContainer width="100%" height="100%">
              <BarChart
                data={chartData}
                margin={{ top: 8, right: 16, bottom: 8, left: 0 }}
              >
                <CartesianGrid strokeDasharray="3 3" opacity={0.4} />
                <XAxis dataKey="label" tick={{ fontSize: 10 }} />
                <YAxis
                  tick={{ fontSize: 10 }}
                  width={48}
                  label={{
                    value: "ms",
                    angle: -90,
                    position: "insideLeft",
                    style: { fontSize: 10 },
                  }}
                />
                <Tooltip
                  formatter={(v, _name, item) => {
                    const row = item.payload as (typeof chartData)[number]
                    return [
                      `${typeof v === "number" ? v : Number(v)} ms`,
                      row.from_cache ? "warm (cached)" : "cold (computed)",
                    ]
                  }}
                  contentStyle={{ fontSize: 12 }}
                />
                <Legend
                  wrapperStyle={{ fontSize: 11 }}
                />
                <Bar dataKey="ms" isAnimationActive={false}>
                  {chartData.map((row, idx) => (
                    <Cell
                      key={idx}
                      fill={row.from_cache ? "#22c55e" : "#f59e0b"}
                    />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          )}
        </div>
      </CardContent>
    </Card>
  )
}
