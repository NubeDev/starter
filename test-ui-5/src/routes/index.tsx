import { createFileRoute, Link } from '@tanstack/react-router'
import { motion } from 'motion/react'
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
  Zap,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { FeatureTile } from '@/components/feature-tile'

const STATS = [
  { k: 'Devices online', v: '412',  accent: 'leaf' as const },
  { k: 'Flows / second', v: '3.4k', accent: 'aqua' as const },
  { k: 'Extensions',     v: '7',    accent: 'sun'  as const },
]

function Hero() {
  return (
    <section className="relative mx-auto max-w-7xl px-4 pt-12 pb-24 sm:px-6 lg:px-8">
      {/* breathing accent glow behind */}
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
        <Badge variant="live">Live · 412 devices streaming</Badge>
      </motion.div>

      <motion.h1
        initial={{ opacity: 0, y: 24, filter: 'blur(8px)' }}
        animate={{ opacity: 1, y: 0, filter: 'blur(0px)' }}
        transition={{ duration: 0.85, ease: [0.22, 1, 0.36, 1], delay: 0.1 }}
        className="mt-6 max-w-4xl text-5xl font-medium leading-[1.02] tracking-[-0.04em] text-white sm:text-7xl lg:text-[88px]"
      >
        The operating layer for the{' '}
        <span className="serif-italic text-[color:var(--color-leaf)]">physical world.</span>
      </motion.h1>

      <motion.p
        initial={{ opacity: 0, y: 16 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.7, ease: [0.22, 1, 0.36, 1], delay: 0.25 }}
        className="mt-6 max-w-xl text-base leading-relaxed text-[color:var(--color-muted)] sm:text-lg"
      >
        Stream telemetry, model assets, automate flows, and ship dashboards —
        all from one extensible Rust runtime. Built for energy, water and HVAC at scale.
      </motion.p>

      <motion.div
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.7, ease: [0.22, 1, 0.36, 1], delay: 0.4 }}
        className="mt-10 flex flex-col gap-3 sm:flex-row"
      >
        <Link to="/dashboard">
          <Button size="lg" variant="leaf">
            Enter dashboard <ArrowUpRight className="h-4 w-4" />
          </Button>
        </Link>
        <Button size="lg" variant="ghost">
          <Play className="h-3.5 w-3.5" /> Watch 90s tour
        </Button>
      </motion.div>

      {/* Live stat strip */}
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
            <div key={s.k} className="glass relative overflow-hidden rounded-3xl p-6">
              <div
                aria-hidden
                className="absolute -right-10 -top-10 h-32 w-32 rounded-full opacity-50 blur-2xl"
                style={{ background: `color-mix(in oklab, ${c} 50%, transparent)` }}
              />
              <div className="flex items-baseline gap-3">
                <span
                  className="tabular text-5xl font-medium tracking-[-0.04em] text-white"
                  style={{ color: c }}
                >
                  {s.v}
                </span>
              </div>
              <div className="mt-2 text-[11px] font-medium uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
                {s.k}
              </div>
            </div>
          )
        })}
      </motion.div>
    </section>
  )
}

function Capabilities() {
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
            What's inside
          </span>
        </div>
        <h2 className="max-w-3xl text-4xl font-medium leading-[1.05] tracking-[-0.03em] text-white sm:text-5xl">
          An IoT platform that{' '}
          <span className="serif-italic text-[color:var(--color-muted)]">gets out of the way.</span>
        </h2>
      </motion.div>

      <div className="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-3">
        <FeatureTile icon={Workflow}  accent="leaf" eyebrow="01 · Flows"      title="Visual runtime"      body="Visual flows compile to a deterministic Rust runtime. Hot-reload in dev, zero-downtime in prod." />
        <FeatureTile icon={Layers}    accent="aqua" eyebrow="02 · Extensions" title="Module-Federation"   body="Drop-in extensions contribute pages, widgets, and commands without forks. Singletons negotiated automatically." />
        <FeatureTile icon={Boxes}     accent="sun"  eyebrow="03 · SDUI"       title="Server-driven UI"    body="Design once, render on web, mobile, and panel. No native rebuild required." />
        <FeatureTile icon={Cpu}       accent="leaf" eyebrow="04 · Warehouse"  title="ClickHouse history"  body="Sub-second queries across millions of points and tags. Tags are Bool|Str — L1 to L3 marts on demand." />
        <FeatureTile icon={Lock}      accent="aqua" eyebrow="05 · AuthZ"      title="Per-user gating"     body="Dynamic resources, not static routes. Gate any SDUI page per user, per tenant, per role." />
        <FeatureTile icon={GitBranch} accent="leaf" eyebrow="06 · Git-native" title="Everything is a file" body="Tags, flows, dashboards — all in git. Branch, diff, review, revert." />
      </div>
    </section>
  )
}

function CTA() {
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
          <Badge variant="leaf">Public preview · Q2 2026</Badge>
          <h2 className="mt-6 max-w-3xl text-5xl font-medium leading-[1.05] tracking-[-0.04em] text-white sm:text-6xl">
            Bring your fleet{' '}
            <span className="serif-italic text-[color:var(--color-leaf)]">online</span> in an afternoon.
          </h2>
          <p className="mt-5 max-w-xl text-base leading-relaxed text-[color:var(--color-muted)]">
            Install the agent, point at your devices, and ship your first dashboard before lunch.
          </p>
          <div className="mt-10 flex flex-col gap-3 sm:flex-row">
            <Link to="/dashboard">
              <Button size="lg" variant="leaf">
                Open the console <ArrowUpRight className="h-4 w-4" />
              </Button>
            </Link>
            <Button size="lg" variant="ghost">
              <Sparkles className="h-3.5 w-3.5" /> Book a demo
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
