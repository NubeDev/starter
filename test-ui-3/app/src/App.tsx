import {
  motion,
  AnimatePresence,
  useScroll,
  useTransform,
  useSpring,
} from 'motion/react'
import { useRef, useState } from 'react'
import {
  Leaf,
  Droplet,
  Wind,
  Sun,
  Sprout,
  Recycle,
  TreePine,
  Waves,
  ArrowUpRight,
} from 'lucide-react'
import GlassmorphismTrustHero from '@/components/ui/glassmorphism-trust-hero'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { MetricCard } from '@/components/metric-card'
import { PerformanceChart } from '@/components/performance-chart'
import { ActivityFeed } from '@/components/activity-feed'
import { RadialProgress } from '@/components/radial-progress'
import { FeatureTile } from '@/components/feature-tile'
import { BootIntro } from '@/components/boot-intro'
import { LayoutProvider, useLayout } from '@/context/layout-provider'
import { FloatingSidebar } from '@/components/floating-sidebar'
import { LayoutToggle } from '@/components/layout-toggle'
import { NAV_GROUPS } from '@/lib/nav'
import { cn } from '@/lib/utils'

const SPARK_AIR = [22, 18, 15, 14, 12, 11, 10, 9, 10, 8, 9, 8, 7]
const SPARK_WATER = [88, 90, 89, 92, 94, 93, 95, 96, 95, 97, 96, 98, 99]
const SPARK_ENERGY = [40, 38, 45, 52, 58, 55, 62, 68, 72, 78, 82, 86, 92]
const SPARK_CARBON = [4, 6, 5, 8, 7, 9, 11, 10, 13, 12, 15, 14, 17]

const ENERGY = [12, 14, 18, 22, 28, 26, 32, 38, 36, 42, 46, 44, 50]
const ENERGY_LABELS = ['MON', 'TUE', 'WED', 'THU', 'FRI', 'SAT', 'SUN']

function ScrollProgress() {
  const { scrollYProgress } = useScroll()
  const scaleX = useSpring(scrollYProgress, { stiffness: 100, damping: 30 })
  return (
    <motion.div
      style={{ scaleX, transformOrigin: '0% 50%' }}
      className="fixed inset-x-0 top-0 z-[60] h-[2px] bg-gradient-to-r from-[color:var(--color-leaf)] via-[color:var(--color-aqua)] to-[color:var(--color-sun)]"
    />
  )
}

function HeaderNav() {
  // Flatten the nav groups into a single inline list for the header layout.
  const items = NAV_GROUPS.flatMap((g) => g.items).slice(0, 5)
  return (
    <motion.header
      initial={{ y: -60, opacity: 0 }}
      animate={{ y: 0, opacity: 1 }}
      exit={{ y: -60, opacity: 0 }}
      transition={{ duration: 0.7, ease: [0.22, 1, 0.36, 1] }}
      className="fixed inset-x-0 top-3 z-50"
    >
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="glass flex items-center justify-between rounded-full px-4 py-2 pl-6">
          <div className="flex items-center gap-2.5">
            <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-[color:var(--color-leaf)] text-[color:var(--color-bg)]">
              <Leaf className="h-4 w-4" strokeWidth={2.25} />
            </div>
            <span className="text-sm font-semibold tracking-tight">Verdant</span>
            <span className="ml-2 hidden text-[10px] uppercase tracking-[0.18em] text-[color:var(--color-subtle)] sm:inline">
              Living Systems
            </span>
          </div>
          <nav className="hidden items-center gap-5 text-sm text-[color:var(--color-muted)] md:flex">
            {items.map((item) => (
              <a
                key={item.href}
                href={item.href}
                className="flex items-center gap-1.5 transition-colors hover:text-white"
              >
                <item.icon className="h-3.5 w-3.5" />
                {item.label}
              </a>
            ))}
          </nav>
          <Button size="sm" variant="leaf">
            Get a sensor
            <ArrowUpRight className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>
    </motion.header>
  )
}

function LayoutChrome() {
  const { mode } = useLayout()
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false)
  return (
    <>
      <AnimatePresence mode="wait">
        {mode === 'header' ? (
          <HeaderNav key="header" />
        ) : (
          <FloatingSidebar
            key="sidebar"
            collapsed={sidebarCollapsed}
            onToggleCollapse={() => setSidebarCollapsed((c) => !c)}
          />
        )}
      </AnimatePresence>
      <LayoutToggle />
    </>
  )
}

function SectionHeading({
  eyebrow,
  title,
  description,
  accent = 'leaf',
}: {
  eyebrow: string
  title: React.ReactNode
  description?: string
  accent?: 'leaf' | 'aqua' | 'sun'
}) {
  const color =
    accent === 'aqua'
      ? 'var(--color-aqua)'
      : accent === 'sun'
      ? 'var(--color-sun)'
      : 'var(--color-leaf)'
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: '-80px' }}
      transition={{ duration: 0.7, ease: [0.22, 1, 0.36, 1] }}
      className="mb-10 flex flex-col gap-3"
    >
      <div className="flex items-center gap-3">
        <span
          className="h-px w-8"
          style={{ background: `color-mix(in oklab, ${color} 100%, transparent)` }}
        />
        <span
          className="text-[11px] font-semibold uppercase tracking-[0.22em]"
          style={{ color: `color-mix(in oklab, ${color} 100%, transparent)` }}
        >
          {eyebrow}
        </span>
      </div>
      <h2 className="max-w-3xl text-4xl font-medium leading-[1.05] tracking-[-0.03em] text-white sm:text-5xl">
        {title}
      </h2>
      {description && (
        <p className="max-w-2xl text-base leading-relaxed text-[color:var(--color-muted)]">
          {description}
        </p>
      )}
    </motion.div>
  )
}

function DashboardSection() {
  return (
    <section className="relative mx-auto max-w-7xl px-4 py-24 sm:px-6 lg:px-8">
      <SectionHeading
        eyebrow="Living dashboard"
        title={
          <>
            Your home, in{' '}
            <span className="serif-italic text-[color:var(--color-leaf)]">balance.</span>
          </>
        }
        description="Air quality, water purity, energy use, and carbon offset — all on one calm surface. No charts wall. Just the truth, at a glance."
      />

      <div className="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-4">
        <MetricCard
          label="Indoor AQI"
          value={12}
          delta={-18.4}
          spark={SPARK_AIR}
          accent="leaf"
        />
        <MetricCard
          label="Water purity"
          value={99.7}
          suffix="%"
          delta={0.4}
          spark={SPARK_WATER}
          accent="aqua"
        />
        <MetricCard
          label="Solar today"
          value={42.3}
          suffix="kWh"
          delta={12.4}
          spark={SPARK_ENERGY}
          accent="sun"
        />
        <MetricCard
          label="CO₂ offset"
          value={184}
          suffix="kg"
          delta={6.1}
          spark={SPARK_CARBON}
          accent="leaf"
        />
      </div>

      <div className="mt-5 grid grid-cols-1 gap-5 lg:grid-cols-3">
        <PerformanceChart
          data={ENERGY}
          labels={ENERGY_LABELS}
          className="lg:col-span-2"
        />
        <RadialProgress value={94} label="Carbon-positive" subLabel="This week" />
      </div>

      <div className="mt-5 grid grid-cols-1 gap-5 lg:grid-cols-3">
        <ActivityFeed className="lg:col-span-2" />
        <div className="glass relative overflow-hidden rounded-3xl p-6">
          <div className="text-[11px] font-medium uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
            Room by room
          </div>
          <div className="mt-6 space-y-5">
            {[
              { label: 'Living room', value: 96, color: 'var(--color-leaf)', icon: TreePine },
              { label: 'Kitchen', value: 88, color: 'var(--color-aqua)', icon: Droplet },
              { label: 'Bedroom', value: 92, color: 'var(--color-sun)', icon: Wind },
              { label: 'Garden', value: 78, color: 'var(--color-leaf-2)', icon: Sprout },
            ].map((b) => (
              <div key={b.label}>
                <div className="mb-1.5 flex items-center justify-between text-xs">
                  <span className="flex items-center gap-2 text-[color:var(--color-muted)]">
                    <b.icon className="h-3.5 w-3.5" />
                    {b.label}
                  </span>
                  <span className="tabular font-medium text-white">{b.value}%</span>
                </div>
                <div className="h-1.5 w-full overflow-hidden rounded-full bg-white/[0.04]">
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

function ManifestoSection() {
  const ref = useRef<HTMLDivElement>(null)
  const { scrollYProgress } = useScroll({ target: ref, offset: ['start end', 'end start'] })
  const y = useTransform(scrollYProgress, [0, 1], [80, -80])
  const blur = useTransform(scrollYProgress, [0, 0.5, 1], [8, 0, 8])
  const filter = useTransform(blur, (b) => `blur(${b}px)`)

  return (
    <section ref={ref} className="relative overflow-hidden border-y border-white/5 py-32">
      <motion.div
        style={{ y }}
        className="pointer-events-none absolute -right-40 top-1/2 -translate-y-1/2 text-[260px] font-bold leading-none tracking-[-0.06em] text-[color:var(--color-leaf)]/[0.04] sm:text-[400px]"
      >
        VERDANT
      </motion.div>

      {/* Drifting leaf particles */}
      {Array.from({ length: 6 }).map((_, i) => (
        <motion.div
          key={i}
          className="pointer-events-none absolute"
          style={{
            left: `${10 + i * 14}%`,
            top: `${20 + (i % 3) * 25}%`,
          }}
          animate={{
            y: [0, -20, 0],
            rotate: [0, 10, -8, 0],
            opacity: [0.2, 0.5, 0.2],
          }}
          transition={{
            duration: 6 + i,
            repeat: Infinity,
            ease: 'easeInOut',
            delay: i * 0.4,
          }}
        >
          <Leaf
            className="h-6 w-6"
            style={{ color: i % 2 ? 'var(--color-leaf)' : 'var(--color-aqua)' }}
          />
        </motion.div>
      ))}

      <motion.div
        style={{ filter }}
        className="relative mx-auto max-w-5xl px-4 text-center sm:px-6 lg:px-8"
      >
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.9, ease: [0.22, 1, 0.36, 1] }}
        >
          <Badge variant="leaf">Manifesto · 01</Badge>
          <h2 className="mt-8 text-5xl font-medium leading-[1.05] tracking-[-0.04em] text-white sm:text-6xl lg:text-7xl">
            We don't sell{' '}
            <span className="serif-italic text-[color:var(--color-leaf)]">gadgets.</span>
            <br />
            We sell{' '}
            <span className="serif-italic text-[color:var(--color-aqua)]">a quieter</span>{' '}
            atmosphere.
          </h2>
          <p className="mx-auto mt-8 max-w-2xl text-lg leading-relaxed text-[color:var(--color-muted)]">
            Cleaner air. Clearer water. Lower carbon. The kind of
            change you don't notice — until you visit somewhere that
            hasn't made it yet.
          </p>
        </motion.div>
      </motion.div>
    </section>
  )
}

function CapabilitiesSection() {
  return (
    <section className="relative mx-auto max-w-7xl px-4 py-24 sm:px-6 lg:px-8">
      <SectionHeading
        eyebrow="The system"
        accent="aqua"
        title={
          <>
            One ecosystem.<br />
            <span className="serif-italic text-[color:var(--color-muted)]">Six living parts.</span>
          </>
        }
      />
      <div className="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-3">
        <FeatureTile
          icon={Wind}
          accent="leaf"
          eyebrow="01 · Air"
          title="HEPA + biofilter"
          body="Plant-rooted filtration removes PM2.5, VOCs, and CO₂ down to 0.1 microns. Silent at night. Strong at peak hour."
        />
        <FeatureTile
          icon={Droplet}
          accent="aqua"
          eyebrow="02 · Water"
          title="Six-stage RO"
          body="Sub-micron reverse osmosis with remineralization. The water tastes like it just came out of a mountain spring — because in spirit, it did."
        />
        <FeatureTile
          icon={Sun}
          accent="sun"
          eyebrow="03 · Energy"
          title="Sun-aware storage"
          body="LFP batteries that learn when your panels peak and your grid is dirtiest. Carbon-negative on most days."
        />
        <FeatureTile
          icon={Sprout}
          accent="leaf"
          eyebrow="04 · Garden"
          title="Indoor microclimate"
          body="Vertical hydroponics for herbs, leafy greens, and air-cleaning species. The soil tells the app when it's thirsty."
        />
        <FeatureTile
          icon={Recycle}
          accent="aqua"
          eyebrow="05 · Cycle"
          title="Greywater + compost"
          body="Sink and shower water diverted to the garden. Food scraps become next month's nutrients. Closed loop."
        />
        <FeatureTile
          icon={Waves}
          accent="leaf"
          eyebrow="06 · Quiet"
          title="Calm by default"
          body="No buzzing app, no panic-red alerts. The whole system whispers — and you sleep better because of it."
        />
      </div>
    </section>
  )
}

function CTASection() {
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
              'radial-gradient(600px 400px at 10% 20%, rgba(74,222,128,0.22), transparent 60%), radial-gradient(500px 400px at 95% 90%, rgba(103,232,249,0.18), transparent 60%), radial-gradient(400px 300px at 50% 50%, rgba(253,230,138,0.10), transparent 60%)',
          }}
        />

        {/* breathing pulse — visual heartbeat */}
        <motion.div
          aria-hidden
          className="absolute -right-20 -top-20 h-72 w-72 rounded-full"
          style={{
            background:
              'radial-gradient(circle, rgba(74,222,128,0.35), transparent 70%)',
          }}
          animate={{ scale: [1, 1.2, 1], opacity: [0.4, 0.7, 0.4] }}
          transition={{ duration: 6, repeat: Infinity, ease: 'easeInOut' }}
        />

        <div className="relative">
          <Badge variant="live">Spring '26 · 14-day trial</Badge>
          <h2 className="mt-6 max-w-3xl text-5xl font-medium leading-[1.05] tracking-[-0.04em] text-white sm:text-6xl">
            Bring a bit of{' '}
            <span className="serif-italic text-[color:var(--color-leaf)]">forest</span>{' '}
            indoors.
          </h2>
          <p className="mt-5 max-w-xl text-base leading-relaxed text-[color:var(--color-muted)]">
            Install in an afternoon. Cancel if you don't sleep
            better in the first two weeks.
          </p>
          <div className="mt-10 flex flex-col gap-3 sm:flex-row">
            <Button size="lg" variant="leaf">
              Start your free audit
              <ArrowUpRight className="h-4 w-4" />
            </Button>
            <Button size="lg" variant="ghost">
              Book a home visit
            </Button>
          </div>
        </div>
      </motion.div>
    </section>
  )
}

function Footer() {
  return (
    <footer className="border-t border-white/5 py-10">
      <div className="mx-auto flex max-w-7xl flex-col items-center justify-between gap-4 px-4 text-xs text-[color:var(--color-subtle)] sm:flex-row sm:px-6 lg:px-8">
        <div className="flex items-center gap-2">
          <Leaf className="h-3.5 w-3.5 text-[color:var(--color-leaf)]" />
          <span>Verdant Living Systems · © 2026 · Carbon negative</span>
        </div>
        <div className="flex items-center gap-6">
          <a className="hover:text-white" href="#">Manifesto</a>
          <a className="hover:text-white" href="#">Suppliers</a>
          <a className="hover:text-white" href="#">B Corp</a>
          <a className="hover:text-white" href="#">Press</a>
        </div>
      </div>
    </footer>
  )
}

function AppShell() {
  const { mode } = useLayout()
  return (
    <div className="min-h-screen bg-[color:var(--color-bg)] text-white">
      <BootIntro />
      <ScrollProgress />
      <LayoutChrome />
      <div
        className={cn(
          'relative transition-[padding] duration-700 ease-[cubic-bezier(0.22,1,0.36,1)]',
          mode === 'sidebar' ? 'lg:pl-[288px]' : 'lg:pl-0',
        )}
      >
        <GlassmorphismTrustHero />
        <DashboardSection />
        <ManifestoSection />
        <CapabilitiesSection />
        <CTASection />
        <Footer />
      </div>
    </div>
  )
}

export default function App() {
  return (
    <LayoutProvider>
      <AppShell />
    </LayoutProvider>
  )
}
