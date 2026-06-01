// Bento KPI tiles for the Energy dashboard.
//
// All tiles consume host theme tokens via `ext-glass`, `ext-eyebrow`,
// `ext-num`, `text-muted-foreground`, `text-foreground`. Source-accent
// hints (amber/cyan/sky/indigo/lime) stay as semantic text-color hints
// using Tailwind tones that read correctly on both light and dark
// host themes.

import * as React from "react";
import {
  Area,
  AreaChart,
  Cell,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
} from "recharts";
import {
  ArrowDownRight,
  ArrowUpRight,
  Cloud,
  CloudRain,
  Leaf,
  Sun,
  TrendingUp,
  Wind as WindIcon,
  Zap,
} from "lucide-react";

import {
  ACCENT,
  BATTERY_RATE_KW,
  BATTERY_SOC_PCT,
  CO2_AVOIDED_KG,
  CO2_DELTA_PCT,
  HERO_SPARK,
  LIVE_STATUS,
  MIX_SHARE,
  TIMELINE_24H,
  TOP_SITES,
  TOTAL_KWH_DELTA_PCT,
  TOTAL_KWH_TODAY,
  WEATHER_5D,
} from "./mock-data";
import type { SourceKind } from "./mock-data";
import {
  BatteryIllustration,
  HydroDamIllustration,
  SolarPanelIllustration,
  WaterDropletIllustration,
  WindTurbineIllustration,
} from "./illustrations";

/* ---------------------------- shared helpers ------------------------- */

function fmtInt(n: number): string {
  return n.toLocaleString();
}

/** Source-tone text class. Uses dual-tone for light/dark legibility. */
const SOURCE_TONE: Record<SourceKind, string> = {
  solar:   "text-amber-600 dark:text-amber-300",
  wind:    "text-cyan-600 dark:text-cyan-300",
  water:   "text-sky-600 dark:text-sky-300",
  hydro:   "text-indigo-600 dark:text-indigo-300",
  storage: "text-lime-600 dark:text-lime-300",
};

function useCountUp(target: number, durationMs = 1100): number {
  const [value, setValue] = React.useState(0);
  React.useEffect(() => {
    if (typeof window !== "undefined" &&
        window.matchMedia?.("(prefers-reduced-motion: reduce)").matches) {
      setValue(target);
      return;
    }
    let raf = 0;
    const t0 = performance.now();
    const step = (now: number) => {
      const p = Math.min(1, (now - t0) / durationMs);
      const eased = 1 - Math.pow(1 - p, 3);
      setValue(Math.round(target * eased));
      if (p < 1) raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  }, [target, durationMs]);
  return value;
}

function DeltaPill({
  pct,
  positiveIsGood = true,
}: {
  pct: number;
  positiveIsGood?: boolean;
}): React.ReactElement {
  const up = pct > 0;
  const good = up === positiveIsGood;
  const Icon = up ? ArrowUpRight : ArrowDownRight;
  return (
    <span
      className={
        "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-xs font-medium tabular-nums " +
        (good
          ? "border-emerald-400/40 bg-emerald-400/10 text-emerald-600 dark:text-emerald-300"
          : "border-amber-400/40 bg-amber-400/10 text-amber-600 dark:text-amber-300")
      }
    >
      <Icon size={12} />
      {Math.abs(pct).toFixed(1)}%
    </span>
  );
}

/* --------------------- Total generation (hero tile) ------------------ */

export function TotalGenerationTile(): React.ReactElement {
  const v = useCountUp(TOTAL_KWH_TODAY);
  const sparkData = HERO_SPARK.map((y, i) => ({ i, y }));
  return (
    <article className="ext-glass p-5 lg:col-span-2" aria-labelledby="nrg-total-h">
      <header className="flex items-start justify-between gap-3">
        <div className="flex items-center gap-2">
          <span className="rounded-lg bg-amber-400/15 p-1.5 text-amber-600 dark:text-amber-300">
            <Zap size={16} />
          </span>
          <div className="ext-eyebrow">Total generation · today</div>
        </div>
        <DeltaPill pct={TOTAL_KWH_DELTA_PCT} />
      </header>

      <div className="mt-4 flex items-end gap-3">
        <div
          className="ext-num text-5xl font-bold leading-none text-foreground sm:text-6xl"
          id="nrg-total-h"
          aria-live="polite"
        >
          {fmtInt(v)}
        </div>
        <div className="pb-2 text-sm font-medium text-muted-foreground">kWh</div>
      </div>

      <div className="mt-1 text-xs text-muted-foreground">
        equivalent to powering ~{Math.round(TOTAL_KWH_TODAY / 30).toLocaleString()} homes for a day
      </div>

      <div className="mt-4 h-20">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={sparkData} margin={{ top: 4, right: 0, bottom: 0, left: 0 }}>
            <defs>
              <linearGradient id="nrg-hero-spark" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%"   stopColor="var(--color-primary)" stopOpacity={0.55} />
                <stop offset="100%" stopColor="var(--color-primary)" stopOpacity={0} />
              </linearGradient>
            </defs>
            <Area
              type="monotone"
              dataKey="y"
              stroke="var(--color-primary)"
              strokeWidth={2}
              fill="url(#nrg-hero-spark)"
              isAnimationActive={false}
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </article>
  );
}

/* ------------------------- CO2 avoided ------------------------------ */

export function Co2AvoidedTile(): React.ReactElement {
  const v = useCountUp(CO2_AVOIDED_KG);
  return (
    <article className="ext-glass p-5">
      <header className="flex items-start justify-between gap-3">
        <div className="flex items-center gap-2">
          <span className="rounded-lg bg-lime-400/15 p-1.5 text-lime-600 dark:text-lime-300">
            <Leaf size={16} />
          </span>
          <div className="ext-eyebrow">CO₂ avoided · today</div>
        </div>
        <DeltaPill pct={CO2_DELTA_PCT} />
      </header>

      <div className="mt-4 flex items-end gap-2">
        <div className="ext-num text-4xl font-bold leading-none text-foreground sm:text-5xl">
          {fmtInt(v)}
        </div>
        <div className="pb-1 text-xs font-medium text-muted-foreground">kg CO₂</div>
      </div>

      <div className="mt-3 grid grid-cols-2 gap-2 text-xs">
        <div className="rounded-md border border-border/60 bg-muted/30 p-2">
          <div className="ext-eyebrow">≈ trees / yr</div>
          <div className="ext-num mt-0.5 font-semibold text-foreground">
            {Math.round(CO2_AVOIDED_KG / 21).toLocaleString()}
          </div>
        </div>
        <div className="rounded-md border border-border/60 bg-muted/30 p-2">
          <div className="ext-eyebrow">≈ km not driven</div>
          <div className="ext-num mt-0.5 font-semibold text-foreground">
            {Math.round(CO2_AVOIDED_KG * 5.3).toLocaleString()}
          </div>
        </div>
      </div>
    </article>
  );
}

/* ----------------------------- Donut --------------------------------- */

export function MixDonutTile(): React.ReactElement {
  const data = MIX_SHARE.map((d) => ({ name: d.label, value: d.value, color: d.color }));
  return (
    <article className="ext-glass p-5">
      <header className="flex items-center justify-between gap-2">
        <div className="ext-eyebrow">Generation mix</div>
        <span className="text-xs text-muted-foreground">today</span>
      </header>

      <div className="mt-2 grid grid-cols-[1fr_auto] items-center gap-3">
        <div className="h-44 min-w-0">
          <ResponsiveContainer width="100%" height="100%">
            <PieChart>
              <Tooltip
                cursor={{ fill: "transparent" }}
                contentStyle={{
                  background: "var(--color-popover, var(--color-card))",
                  border: "1px solid var(--color-border)",
                  borderRadius: 8,
                  color: "var(--color-foreground)",
                  fontSize: 12,
                }}
                formatter={(value) => [`${Number(value)}%`, ""]}
              />
              <Pie
                data={data}
                dataKey="value"
                nameKey="name"
                innerRadius={45}
                outerRadius={72}
                paddingAngle={2}
                stroke="var(--color-card)"
                strokeWidth={2}
              >
                {data.map((entry) => (
                  <Cell key={entry.name} fill={entry.color} />
                ))}
              </Pie>
            </PieChart>
          </ResponsiveContainer>
        </div>

        <ul className="flex shrink-0 flex-col gap-1.5 text-xs">
          {MIX_SHARE.map((d) => (
            <li key={d.kind} className="flex items-center gap-2">
              <span
                aria-hidden="true"
                className="size-2.5 shrink-0 rounded-sm"
                style={{ backgroundColor: d.color }}
              />
              <span className="text-muted-foreground">{d.label}</span>
              <span className="ext-num ml-auto font-medium text-foreground">
                {d.value}%
              </span>
            </li>
          ))}
        </ul>
      </div>
    </article>
  );
}

/* -------------------- 24h stacked-area timeline ---------------------- */

export function GenerationTimelineTile(): React.ReactElement {
  return (
    <article className="ext-glass p-5 lg:col-span-3">
      <header className="mb-2 flex items-end justify-between gap-3">
        <div>
          <div className="ext-eyebrow">Generation timeline · last 24 hours</div>
          <div className="mt-1 text-sm font-medium text-foreground">
            Stacked by source · solar / wind / water / hydro
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2 text-xs">
          {(["solar", "wind", "water", "hydro"] as const).map((k) => (
            <span
              key={k}
              className="inline-flex items-center gap-1.5 rounded-full border border-border/60 bg-muted/30 px-2 py-0.5"
            >
              <span
                aria-hidden="true"
                className="size-2 rounded-sm"
                style={{ backgroundColor: ACCENT[k].from }}
              />
              <span className="capitalize text-muted-foreground">{k}</span>
            </span>
          ))}
        </div>
      </header>

      <div className="h-64 sm:h-72">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={TIMELINE_24H} margin={{ top: 12, right: 8, bottom: 18, left: 4 }}>
            <defs>
              {(["solar", "wind", "water", "hydro"] as const).map((k) => (
                <linearGradient key={k} id={`nrg-area-${k}`} x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%"   stopColor={ACCENT[k].from} stopOpacity={0.75} />
                  <stop offset="100%" stopColor={ACCENT[k].to}   stopOpacity={0.05} />
                </linearGradient>
              ))}
            </defs>
            <Tooltip
              cursor={{ stroke: "var(--color-border)" }}
              contentStyle={{
                background: "var(--color-popover, var(--color-card))",
                border: "1px solid var(--color-border)",
                borderRadius: 8,
                color: "var(--color-foreground)",
                fontSize: 12,
              }}
              labelStyle={{ color: "var(--color-muted-foreground)", fontSize: 11 }}
              formatter={(value, name) => [`${Number(value).toLocaleString()} kW`, String(name)]}
            />
            {(["hydro", "water", "wind", "solar"] as const).map((k) => (
              <Area
                key={k}
                type="monotone"
                dataKey={k}
                stackId="1"
                name={k.charAt(0).toUpperCase() + k.slice(1)}
                stroke={ACCENT[k].from}
                strokeWidth={1.5}
                fill={`url(#nrg-area-${k})`}
                isAnimationActive={false}
              />
            ))}
          </AreaChart>
        </ResponsiveContainer>
      </div>
      <div className="ext-num mt-1 flex justify-between text-[0.65rem] text-muted-foreground">
        {TIMELINE_24H.filter((_, i) => i % 4 === 0).map((p) => (
          <span key={p.t}>{p.t}</span>
        ))}
      </div>
    </article>
  );
}

/* ----------------------- Top sites leaderboard ----------------------- */

export function TopSitesTile(): React.ReactElement {
  return (
    <article className="ext-glass p-5">
      <header className="mb-3 flex items-center justify-between">
        <div className="ext-eyebrow">Top sites · today</div>
        <TrendingUp size={14} className="text-muted-foreground" />
      </header>
      <ul className="flex flex-col gap-2.5">
        {TOP_SITES.map((s) => {
          const accent = ACCENT[s.kind];
          return (
            <li key={s.name}>
              <div className="flex items-center justify-between gap-3 text-xs">
                <div className="min-w-0">
                  <div className="truncate text-sm font-medium text-foreground">{s.name}</div>
                  <div className="ext-eyebrow truncate">
                    {s.region} · {s.kind}
                  </div>
                </div>
                <div className="ext-num shrink-0 font-semibold text-foreground">
                  {fmtInt(s.total)}
                  <span className="ml-1 text-[0.65rem] text-muted-foreground">kWh</span>
                </div>
              </div>
              <div
                className="mt-1 h-1.5 w-full overflow-hidden rounded-full bg-muted/40"
                role="progressbar"
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={Math.round(s.share * 100)}
                aria-label={`${s.name} share of top`}
              >
                <div
                  className="h-full rounded-full"
                  style={{
                    width: `${s.share * 100}%`,
                    background: `linear-gradient(90deg, ${accent.from}, ${accent.to})`,
                  }}
                />
              </div>
            </li>
          );
        })}
      </ul>
    </article>
  );
}

/* ------------------------- Live status ------------------------------- */

function statusColor(status: "ok" | "warn" | "fault"): { dot: string; pulse: string } {
  switch (status) {
    case "ok":    return { dot: "#22c55e", pulse: "rgba(34, 197, 94, 0.55)"  };
    case "warn":  return { dot: "#f59e0b", pulse: "rgba(245, 158, 11, 0.55)" };
    case "fault": return { dot: "#ef4444", pulse: "rgba(239, 68, 68, 0.55)"  };
  }
}

export function LiveStatusTile(): React.ReactElement {
  return (
    <article className="ext-glass p-5">
      <header className="mb-3 flex items-center justify-between">
        <div className="ext-eyebrow">Live status</div>
        <span className="ext-eyebrow">streaming</span>
      </header>
      <ul className="flex flex-col gap-3">
        {LIVE_STATUS.map((s) => {
          const c = statusColor(s.status);
          return (
            <li key={s.kind} className="flex items-start gap-3">
              <span
                className="nrg-dot-pulse mt-1 inline-block size-2.5 shrink-0 rounded-full"
                style={
                  {
                    backgroundColor: c.dot,
                    ["--nrg-pulse-color" as never]: c.pulse,
                  } as React.CSSProperties
                }
                aria-hidden="true"
              />
              <div className="min-w-0 flex-1">
                <div className="flex items-baseline justify-between gap-2">
                  <span className="truncate text-sm font-medium text-foreground">{s.label}</span>
                  <span className="ext-num shrink-0 text-sm font-semibold text-foreground">
                    {s.online}
                    {s.offline > 0 ? (
                      <span className="ml-1 text-xs font-normal text-amber-600 dark:text-amber-300">
                        /{s.offline}↓
                      </span>
                    ) : null}
                  </span>
                </div>
                <div className="text-[0.7rem] text-muted-foreground">{s.detail}</div>
              </div>
            </li>
          );
        })}
      </ul>
    </article>
  );
}

/* -------------------------- Weather strip ---------------------------- */

function WeatherGlyph({ icon }: { icon: "sun" | "cloud" | "rain" | "wind" }): React.ReactElement {
  switch (icon) {
    case "sun":   return <Sun size={20} className="text-amber-500 dark:text-amber-300" />;
    case "cloud": return <Cloud size={20} className="text-muted-foreground" />;
    case "rain":  return <CloudRain size={20} className="text-sky-500 dark:text-sky-300" />;
    case "wind":  return <WindIcon size={20} className="text-cyan-500 dark:text-cyan-300" />;
  }
}

export function WeatherTile(): React.ReactElement {
  return (
    <article className="ext-glass p-5">
      <header className="mb-3 flex items-center justify-between">
        <div className="ext-eyebrow">Forecast · 5 day</div>
        <span className="ext-eyebrow">drives plan</span>
      </header>
      <ul className="grid grid-cols-5 gap-2">
        {WEATHER_5D.map((d) => (
          <li
            key={d.day}
            className="rounded-lg border border-border/60 bg-muted/30 p-2 text-center"
          >
            <div className="ext-eyebrow">{d.day}</div>
            <div className="my-1.5 flex justify-center">
              <WeatherGlyph icon={d.icon} />
            </div>
            <div className="ext-num text-xs font-semibold text-foreground">
              {d.high}°
              <span className="font-normal text-muted-foreground">/{d.low}°</span>
            </div>
            <div className="mt-1 inline-flex items-center gap-0.5 text-[0.65rem] text-muted-foreground">
              <WindIcon size={9} />
              <span className="ext-num">{d.wind}</span>
            </div>
          </li>
        ))}
      </ul>
    </article>
  );
}

/* -------------------------- Source vignettes ------------------------- */

/** Four illustrated source vignettes (solar / wind / water / hydro)
 *  + battery storage. Each shows the source illustration, a label,
 *  and a tiny tabular metric. */
export function SourceVignettes(): React.ReactElement {
  return (
    <article className="ext-glass p-3 lg:col-span-3">
      <div className="grid grid-cols-2 gap-3 md:grid-cols-5">
        <Vignette
          kind="solar"
          metric="84.2 MW"
          label="Peak output 12:00"
          illustration={<SolarPanelIllustration className="h-24 w-full" />}
        />
        <Vignette
          kind="wind"
          metric="14.2 rpm"
          label="Avg rotor speed"
          illustration={<WindTurbineIllustration className="h-24 w-full" />}
        />
        <Vignette
          kind="water"
          metric="2,180 L/s"
          label="Aggregate flow"
          illustration={<WaterDropletIllustration className="h-24 w-full" />}
        />
        <Vignette
          kind="hydro"
          metric="11 GWh"
          label="Reservoir reserve"
          illustration={<HydroDamIllustration className="h-24 w-full" />}
        />
        <Vignette
          kind="storage"
          metric={`${BATTERY_RATE_KW.toLocaleString()} kW`}
          label={`Discharging · ${BATTERY_SOC_PCT}% SoC`}
          illustration={<BatteryIllustration className="h-24 w-full" socPct={BATTERY_SOC_PCT} />}
        />
      </div>
    </article>
  );
}

function Vignette({
  kind,
  metric,
  label,
  illustration,
}: {
  kind: SourceKind;
  metric: string;
  label: string;
  illustration: React.ReactElement;
}): React.ReactElement {
  return (
    <div className="rounded-xl border border-border/60 bg-muted/20 p-2 transition-colors duration-200 hover:bg-muted/40">
      <div className="overflow-hidden rounded-lg">{illustration}</div>
      <div className="mt-2 flex items-baseline justify-between gap-2 px-1">
        <span className={"ext-eyebrow " + SOURCE_TONE[kind]}>{kind}</span>
        <span className="ext-num text-sm font-semibold text-foreground">
          {metric}
        </span>
      </div>
      <div className="px-1 text-[0.7rem] text-muted-foreground">{label}</div>
    </div>
  );
}
