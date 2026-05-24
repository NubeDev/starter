import { createFileRoute } from '@tanstack/react-router'
import { motion } from 'motion/react'
import { useIntl } from 'react-intl'
import { Cpu, Droplet, Gauge, Wind } from 'lucide-react'
import { MetricCard } from '@/components/dashboard/metric-card'
import { PerformanceChart } from '@/components/dashboard/performance-chart'
import { RadialProgress } from '@/components/dashboard/radial-progress'
import { ActivityFeed } from '@/components/dashboard/activity-feed'

const SPARK_DEVICES = [380, 384, 388, 390, 395, 398, 402, 405, 408, 410, 411, 412]
const SPARK_LOAD    = [22, 24, 26, 28, 30, 28, 26, 28, 30, 31, 30, 28]
const SPARK_LATENCY = [60, 55, 52, 50, 48, 46, 44, 43, 42, 42, 42, 41]
const SPARK_EVENTS  = [2.1, 2.3, 2.4, 2.5, 2.7, 2.9, 3.0, 3.1, 3.2, 3.3, 3.35, 3.4]

const LOAD = [12, 14, 18, 22, 28, 26, 32, 38, 36, 42, 46, 44, 50]
const LOAD_LABELS = ['MON', 'TUE', 'WED', 'THU', 'FRI', 'SAT', 'SUN']

function Section() {
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  const sites = [
    { labelKey: 'dashboard.site.northPlant',  value: 96, color: 'var(--color-leaf)',   icon: Gauge },
    { labelKey: 'dashboard.site.southCampus', value: 88, color: 'var(--color-aqua)',   icon: Droplet },
    { labelKey: 'dashboard.site.eastTower',   value: 92, color: 'var(--color-sun)',    icon: Wind },
    { labelKey: 'dashboard.site.westDepot',   value: 78, color: 'var(--color-leaf-2)', icon: Cpu },
  ]
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
        <MetricCard label={tr('dashboard.metric.devicesOnline')}    value={412}   delta={2.4}  spark={SPARK_DEVICES} accent="leaf" />
        <MetricCard label={tr('dashboard.metric.siteLoad')}         value={2184}  suffix="kW" delta={-1.2} spark={SPARK_LOAD}    accent="aqua" />
        <MetricCard label={tr('dashboard.metric.avgLatency')}       value={42}    suffix="ms" delta={-8.5} spark={SPARK_LATENCY} accent="sun"  />
        <MetricCard label={tr('dashboard.metric.eventsPerSecond')}  value={3.4}   suffix="k"  delta={4.1}  spark={SPARK_EVENTS}  accent="leaf" />
      </div>

      <div className="mt-5 grid grid-cols-1 gap-5 lg:grid-cols-3">
        <PerformanceChart data={LOAD} labels={LOAD_LABELS} className="lg:col-span-2" />
        <RadialProgress value={94} label={tr('dashboard.deviceHealth')} subLabel={tr('dashboard.deviceHealthSub')} />
      </div>

      <div className="mt-5 grid grid-cols-1 gap-5 lg:grid-cols-3">
        <ActivityFeed className="lg:col-span-2" />
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

export const Route = createFileRoute('/dashboard')({ component: Section })
