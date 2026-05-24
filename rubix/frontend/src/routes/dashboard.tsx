import { createFileRoute } from '@tanstack/react-router'
import { motion } from 'motion/react'
import { useIntl } from 'react-intl'
import {
  Cpu,
  Droplet,
  Gauge,
  Wind,
  Leaf,
  Sun,
  Recycle,
  Sprout,
  type LucideIcon,
} from 'lucide-react'
import {
  MetricCard,
  PerformanceChart,
  RadialProgress,
  ActivityFeed,
  type ActivityItem,
} from '@nube/starter-ui-dashboard'
import { useDiskUsage } from '@nube/rubix-client-react'
import { ErrorBoundary } from '@/components/error-boundary'

const SPARK_DEVICES = [380, 384, 388, 390, 395, 398, 402, 405, 408, 410, 411, 412]
const SPARK_LOAD    = [22, 24, 26, 28, 30, 28, 26, 28, 30, 31, 30, 28]
const SPARK_LATENCY = [60, 55, 52, 50, 48, 46, 44, 43, 42, 42, 42, 41]
const SPARK_EVENTS  = [2.1, 2.3, 2.4, 2.5, 2.7, 2.9, 3.0, 3.1, 3.2, 3.3, 3.35, 3.4]

const LOAD = [12, 14, 18, 22, 28, 26, 32, 38, 36, 42, 46, 44, 50]
const LOAD_LABELS = ['MON', 'TUE', 'WED', 'THU', 'FRI', 'SAT', 'SUN']

// Map the demo's biophilic accent palette to CSS colors so the package
// stays theme-agnostic but this page keeps its established look.
const ACCENT_LEAF = 'var(--color-leaf, #4ade80)'
const ACCENT_AQUA = 'var(--color-aqua, #67e8f9)'
const ACCENT_SUN  = 'var(--color-sun,  #fde68a)'

// Source data for the activity feed; titles/metas are resolved through
// `useIntl` at render time so the package itself stays i18n-free.
type ActivitySeed = {
  id: string
  icon: LucideIcon
  titleKey: string
  metaKey: string
  time: string
  accent: string
}
const ACTIVITY_SEEDS: ActivitySeed[] = [
  { id: 'air',       icon: Leaf,    titleKey: 'activity.item.airUpgraded.title', metaKey: 'activity.item.airUpgraded.meta', time: '0m',  accent: ACCENT_LEAF },
  { id: 'water',     icon: Droplet, titleKey: 'activity.item.waterFilter.title', metaKey: 'activity.item.waterFilter.meta', time: '2m',  accent: ACCENT_AQUA },
  { id: 'solar',     icon: Sun,     titleKey: 'activity.item.solarPeak.title',   metaKey: 'activity.item.solarPeak.meta',   time: '14m', accent: ACCENT_SUN  },
  { id: 'seedling',  icon: Sprout,  titleKey: 'activity.item.seedling.title',    metaKey: 'activity.item.seedling.meta',    time: '1h',  accent: ACCENT_LEAF },
  { id: 'co2',       icon: Wind,    titleKey: 'activity.item.co2Vented.title',   metaKey: 'activity.item.co2Vented.meta',   time: '2h',  accent: ACCENT_AQUA },
  { id: 'greywater', icon: Recycle, titleKey: 'activity.item.greywater.title',   metaKey: 'activity.item.greywater.meta',   time: '3h',  accent: ACCENT_LEAF },
]

function Section() {
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  // Live disk-usage probe via rubix-agent. While loading we keep the
  // dial at the last-known value (or 0); on error the dial drops to 0
  // and the surrounding ErrorBoundary surfaces the localised
  // diagnostic. `percent_used` is already a 0-100 integer per the
  // `rubix.system.disk` DTO.
  const disk = useDiskUsage()
  const diskPercent = Math.round(disk.data?.percent_used ?? 0)
  const sites = [
    { labelKey: 'dashboard.site.northPlant',  value: 96, color: 'var(--color-leaf)',   icon: Gauge },
    { labelKey: 'dashboard.site.southCampus', value: 88, color: 'var(--color-aqua)',   icon: Droplet },
    { labelKey: 'dashboard.site.eastTower',   value: 92, color: 'var(--color-sun)',    icon: Wind },
    { labelKey: 'dashboard.site.westDepot',   value: 78, color: 'var(--color-leaf-2)', icon: Cpu },
  ]

  const activityItems: ActivityItem[] = ACTIVITY_SEEDS.map((seed) => ({
    id: seed.id,
    icon: seed.icon,
    title: tr(seed.titleKey),
    meta: tr(seed.metaKey),
    time: seed.time,
    accent: seed.accent,
  }))

  return (
    <section className="relative mx-auto max-w-7xl px-4 pb-24 pt-6 sm:px-6 lg:px-8">
      <motion.div
        initial={{ opacity: 0, y: 14 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.7, ease: [0.22, 1, 0.36, 1] }}
        className="mb-10 flex items-end justify-between gap-4"
      >
        <div className="flex flex-col gap-3">
          <div className="flex items-center gap-3">
            <span className="h-px w-8 bg-[color:var(--color-leaf)]" />
            <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-[color:var(--color-leaf)]">
              {tr('dashboard.eyebrow')}
            </span>
          </div>
          <h1 className="max-w-3xl text-4xl font-medium leading-[1.05] tracking-[-0.03em] text-[color:var(--color-text)] sm:text-5xl">
            {tr('dashboard.titlePrefix')}{' '}
            <span className="serif-italic text-[color:var(--color-leaf)]">{tr('dashboard.titleAccent')}</span>
          </h1>
        </div>
        <div className="text-xs text-[color:var(--color-subtle)]">
          {tr('dashboard.updated')}{' '}
          <span className="text-[color:var(--color-text)]">{tr('dashboard.updatedJustNow')}</span>{' '}
          {intl.formatMessage({ id: 'dashboard.sitesSuffix' }, { count: 3 })}
        </div>
      </motion.div>

      <div className="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-4">
        <MetricCard label={tr('dashboard.metric.devicesOnline')}    value={412}   delta={2.4}  spark={SPARK_DEVICES} accent={ACCENT_LEAF} />
        <MetricCard label={tr('dashboard.metric.siteLoad')}         value={2184}  suffix="kW" delta={-1.2} spark={SPARK_LOAD}    accent={ACCENT_AQUA} />
        <MetricCard label={tr('dashboard.metric.avgLatency')}       value={42}    suffix="ms" delta={-8.5} spark={SPARK_LATENCY} accent={ACCENT_SUN}  />
        <MetricCard label={tr('dashboard.metric.eventsPerSecond')}  value={3.4}   suffix="k"  delta={4.1}  spark={SPARK_EVENTS}  accent={ACCENT_LEAF} />
      </div>

      <div className="mt-5 grid grid-cols-1 gap-5 lg:grid-cols-3">
        <PerformanceChart
          data={LOAD}
          labels={LOAD_LABELS}
          title={tr('chart.energyHarvested')}
          headline="42.3"
          headlineSuffix="kWh"
          delta="↑ 12.4%"
          periods={['1D', '1W', '1M', '1Y']}
          activePeriodIndex={1}
          accent={ACCENT_LEAF}
          className="lg:col-span-2"
        />
        <RadialProgress
          value={diskPercent}
          label={tr('dashboard.diskUsage')}
          subLabel={tr('dashboard.diskUsageSub')}
          accent={ACCENT_LEAF}
        />
      </div>

      <div className="mt-5 grid grid-cols-1 gap-5 lg:grid-cols-3">
        <ActivityFeed
          className="lg:col-span-2"
          items={activityItems}
          title={tr('activity.title')}
          streamingLabel={tr('activity.streaming')}
          nowLabel={tr('activity.now')}
        />
        <div className="glass relative overflow-hidden rounded-3xl p-[var(--pad-card)]">
          <div className="text-[11px] font-medium uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
            {tr('dashboard.siteBySite')}
          </div>
          <div className="mt-6 space-y-5">
            {sites.map((b) => (
              <div key={b.labelKey}>
                <div className="mb-1.5 flex items-center justify-between text-xs">
                  <span className="flex items-center gap-2 text-[color:var(--color-muted)]">
                    <b.icon className="h-3.5 w-3.5" />
                    {tr(b.labelKey)}
                  </span>
                  <span className="tabular font-medium text-[color:var(--color-text)]">{b.value}%</span>
                </div>
                <div className="h-1.5 w-full overflow-hidden rounded-full bg-[color:var(--color-surface-2)]">
                  <motion.div
                    initial={{ width: 0 }}
                    whileInView={{ width: `${b.value}%` }}
                    viewport={{ once: true }}
                    transition={{ duration: 1.4, ease: [0.22, 1, 0.36, 1] }}
                    className="h-full rounded-full"
                    style={{ background: b.color }}
                  />
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  )
}

function DashboardRoute() {
  return (
    <ErrorBoundary>
      <Section />
    </ErrorBoundary>
  )
}

export const Route = createFileRoute('/dashboard')({ component: DashboardRoute })
