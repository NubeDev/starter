import { createFileRoute, Link } from '@tanstack/react-router'
import { motion } from 'motion/react'
import { useIntl } from 'react-intl'
import {
  ArrowUpRight,
  Boxes,
  Cpu,
  GitBranch,
  Layers,
  Lock,
  Play,
  Sparkles,
  Workflow,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { FeatureTile } from '@/components/feature-tile'

const STATS = [
  { labelKey: 'home.stats.devicesOnline',   v: '412',  accent: 'leaf' as const },
  { labelKey: 'home.stats.flowsPerSecond',  v: '3.4k', accent: 'aqua' as const },
  { labelKey: 'home.stats.extensions',      v: '7',    accent: 'sun'  as const },
]

function Hero() {
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  return (
    <section className="relative mx-auto max-w-7xl px-4 pt-12 pb-24 sm:px-6 lg:px-8">
      <div
        aria-hidden
        className="pointer-events-none absolute inset-0 -z-10"
        style={{
          background:
            'radial-gradient(700px 400px at 15% 10%, rgba(51,153,153,0.22), transparent 60%), radial-gradient(600px 400px at 95% 30%, rgba(103,232,249,0.16), transparent 60%)',
        }}
      />

      <motion.div
        initial={{ opacity: 0, y: 14 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.6, ease: [0.22, 1, 0.36, 1] }}
      >
        <Badge variant="live">{intl.formatMessage({ id: 'home.badge.live' }, { count: 412 })}</Badge>
      </motion.div>

      <motion.h1
        initial={{ opacity: 0, y: 24, filter: 'blur(8px)' }}
        animate={{ opacity: 1, y: 0, filter: 'blur(0px)' }}
        transition={{ duration: 0.85, ease: [0.22, 1, 0.36, 1], delay: 0.1 }}
        className="mt-6 max-w-4xl text-5xl font-medium leading-[1.02] tracking-[-0.04em] text-[color:var(--color-text)] sm:text-7xl lg:text-[88px]"
      >
        {tr('home.hero.titlePrefix')}{' '}
        <span className="serif-italic text-[color:var(--color-leaf)]">{tr('home.hero.titleAccent')}</span>
      </motion.h1>

      <motion.p
        initial={{ opacity: 0, y: 16 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.7, ease: [0.22, 1, 0.36, 1], delay: 0.25 }}
        className="mt-6 max-w-xl text-base leading-relaxed text-[color:var(--color-muted)] sm:text-lg"
      >
        {tr('home.hero.subtitle')}
      </motion.p>

      <motion.div
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.7, ease: [0.22, 1, 0.36, 1], delay: 0.4 }}
        className="mt-10 flex flex-col gap-3 sm:flex-row"
      >
        <Link to="/dashboard">
          <Button size="lg" variant="leaf">
            {tr('home.cta.enterDashboard')} <ArrowUpRight className="h-4 w-4" />
          </Button>
        </Link>
        <Button size="lg" variant="ghost">
          <Play className="h-3.5 w-3.5" /> {tr('home.cta.watchTour')}
        </Button>
      </motion.div>

      <motion.div
        initial={{ opacity: 0, y: 24 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.8, ease: [0.22, 1, 0.36, 1], delay: 0.55 }}
        className="mt-16 grid grid-cols-1 gap-3 sm:grid-cols-3"
      >
        {STATS.map((s) => {
          const c =
            s.accent === 'leaf' ? 'var(--color-leaf)' :
            s.accent === 'aqua' ? 'var(--color-aqua)' :
            'var(--color-sun)'
          return (
            <div key={s.labelKey} className="glass relative overflow-hidden rounded-3xl p-6">
              <div
                aria-hidden
                className="absolute -right-10 -top-10 h-32 w-32 rounded-full opacity-50 blur-2xl"
                style={{ background: `color-mix(in oklab, ${c} 50%, transparent)` }}
              />
              <div className="flex items-baseline gap-3">
                <span
                  className="tabular text-5xl font-medium tracking-[-0.04em] text-[color:var(--color-text)]"
                  style={{ color: c }}
                >
                  {s.v}
                </span>
              </div>
              <div className="mt-2 text-[11px] font-medium uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
                {tr(s.labelKey)}
              </div>
            </div>
          )
        })}
      </motion.div>
    </section>
  )
}

function Capabilities() {
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  return (
    <section className="relative mx-auto max-w-7xl px-4 pb-24 sm:px-6 lg:px-8">
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: '-80px' }}
        transition={{ duration: 0.7, ease: [0.22, 1, 0.36, 1] }}
        className="mb-10 flex flex-col gap-3"
      >
        <div className="flex items-center gap-3">
          <span className="h-px w-8 bg-[color:var(--color-leaf)]" />
          <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-[color:var(--color-leaf)]">
            {tr('home.capabilities.eyebrow')}
          </span>
        </div>
        <h2 className="max-w-3xl text-4xl font-medium leading-[1.05] tracking-[-0.03em] text-[color:var(--color-text)] sm:text-5xl">
          {tr('home.capabilities.titlePrefix')}{' '}
          <span className="serif-italic text-[color:var(--color-muted)]">{tr('home.capabilities.titleAccent')}</span>
        </h2>
      </motion.div>

      <div className="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-3">
        <FeatureTile icon={Workflow}  accent="leaf" eyebrow={tr('home.feature.flows.eyebrow')}      title={tr('home.feature.flows.title')}      body={tr('home.feature.flows.body')} />
        <FeatureTile icon={Layers}    accent="aqua" eyebrow={tr('home.feature.extensions.eyebrow')} title={tr('home.feature.extensions.title')} body={tr('home.feature.extensions.body')} />
        <FeatureTile icon={Boxes}     accent="sun"  eyebrow={tr('home.feature.sdui.eyebrow')}       title={tr('home.feature.sdui.title')}       body={tr('home.feature.sdui.body')} />
        <FeatureTile icon={Cpu}       accent="leaf" eyebrow={tr('home.feature.warehouse.eyebrow')}  title={tr('home.feature.warehouse.title')}  body={tr('home.feature.warehouse.body')} />
        <FeatureTile icon={Lock}      accent="aqua" eyebrow={tr('home.feature.authz.eyebrow')}      title={tr('home.feature.authz.title')}      body={tr('home.feature.authz.body')} />
        <FeatureTile icon={GitBranch} accent="leaf" eyebrow={tr('home.feature.git.eyebrow')}        title={tr('home.feature.git.title')}        body={tr('home.feature.git.body')} />
      </div>
    </section>
  )
}

function CTA() {
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  return (
    <section className="relative mx-auto max-w-7xl px-4 pb-24 sm:px-6 lg:px-8">
      <motion.div
        initial={{ opacity: 0, y: 40 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.9, ease: [0.22, 1, 0.36, 1] }}
        className="relative overflow-hidden rounded-[2.5rem] border border-[color:var(--color-leaf)]/15 bg-gradient-to-br from-[color:var(--color-surface)] via-[color:var(--color-bg)] to-[color:var(--color-surface)] p-12 sm:p-20"
      >
        <div
          aria-hidden
          className="absolute inset-0 opacity-70"
          style={{
            background:
              'radial-gradient(600px 400px at 10% 20%, rgba(51,153,153,0.22), transparent 60%), radial-gradient(500px 400px at 95% 90%, rgba(103,232,249,0.18), transparent 60%)',
          }}
        />
        <motion.div
          aria-hidden
          className="absolute -right-20 -top-20 h-72 w-72 rounded-full"
          style={{ background: 'radial-gradient(circle, rgba(51,153,153,0.35), transparent 70%)' }}
          animate={{ scale: [1, 1.2, 1], opacity: [0.4, 0.7, 0.4] }}
          transition={{ duration: 6, repeat: Infinity, ease: 'easeInOut' }}
        />
        <div className="relative">
          <Badge variant="leaf">{tr('home.cta2.badge')}</Badge>
          <h2 className="mt-6 max-w-3xl text-5xl font-medium leading-[1.05] tracking-[-0.04em] text-[color:var(--color-text)] sm:text-6xl">
            {tr('home.cta2.titlePrefix')}{' '}
            <span className="serif-italic text-[color:var(--color-leaf)]">{tr('home.cta2.titleAccent')}</span>{' '}
            {tr('home.cta2.titleSuffix')}
          </h2>
          <p className="mt-5 max-w-xl text-base leading-relaxed text-[color:var(--color-muted)]">
            {tr('home.cta2.subtitle')}
          </p>
          <div className="mt-10 flex flex-col gap-3 sm:flex-row">
            <Link to="/dashboard">
              <Button size="lg" variant="leaf">
                {tr('home.cta2.openConsole')} <ArrowUpRight className="h-4 w-4" />
              </Button>
            </Link>
            <Button size="lg" variant="ghost">
              <Sparkles className="h-3.5 w-3.5" /> {tr('home.cta2.bookDemo')}
            </Button>
          </div>
        </div>
      </motion.div>
    </section>
  )
}

function Landing() {
  return (
    <>
      <Hero />
      <Capabilities />
      <CTA />
    </>
  )
}

export const Route = createFileRoute('/')({ component: Landing })
