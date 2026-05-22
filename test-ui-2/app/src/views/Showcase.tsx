import { useEffect, useRef } from 'react'
import {
  motion,
  useInView,
  useMotionValue,
  useScroll,
  useSpring,
  useTransform,
  animate,
} from 'motion/react'
import { ArrowRight, Sparkles } from 'lucide-react'
import GlassmorphismTrustHero from '@/components/showcase/glassmorphism-trust-hero'

const AMBER = '#ffcd75'

function Counter({ to, suffix = '' }: { to: number; suffix?: string }) {
  const ref = useRef<HTMLSpanElement>(null)
  const inView = useInView(ref, { once: true, margin: '-20% 0px' })
  const mv = useMotionValue(0)
  useEffect(() => {
    if (!inView) return
    const c = animate(mv, to, { duration: 1.6, ease: [0.22, 1, 0.36, 1] })
    return () => c.stop()
  }, [inView, mv, to])
  const text = useTransform(mv, (v) => Math.round(v).toLocaleString() + suffix)
  return <motion.span ref={ref}>{text}</motion.span>
}

function KineticHeadline() {
  const ref = useRef<HTMLDivElement>(null)
  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ['start end', 'end start'],
  })
  const x = useTransform(scrollYProgress, [0, 1], [-120, 120])
  return (
    <section
      ref={ref}
      className="relative overflow-hidden border-y border-white/5 bg-zinc-950 py-24 sm:py-32"
    >
      <motion.div
        style={{ x }}
        className="whitespace-nowrap text-[18vw] font-medium leading-none tracking-tighter text-white/[0.06] select-none"
        aria-hidden
      >
        DESIGNED · IN · MOTION · DESIGNED · IN · MOTION ·
      </motion.div>
      <div className="absolute inset-0 flex items-center justify-center px-4">
        <div className="max-w-4xl text-center">
          <motion.p
            initial={{ opacity: 0, y: 24 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: '-15% 0px' }}
            transition={{ duration: 0.8, ease: [0.22, 1, 0.36, 1] }}
            className="font-mono text-xs uppercase tracking-[0.3em] text-zinc-500"
          >
            · 02 · Philosophy
          </motion.p>
          <motion.h2
            initial={{ opacity: 0, y: 24 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: '-15% 0px' }}
            transition={{ duration: 0.9, delay: 0.1, ease: [0.22, 1, 0.36, 1] }}
            className="mt-6 text-5xl font-medium tracking-tight text-white sm:text-6xl lg:text-7xl"
          >
            Restraint is a feature.
          </motion.h2>
          <motion.p
            initial={{ opacity: 0, y: 24 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: '-15% 0px' }}
            transition={{ duration: 0.9, delay: 0.2, ease: [0.22, 1, 0.36, 1] }}
            className="mt-6 text-lg text-zinc-400 sm:text-xl"
          >
            We removed the gradients, the glow, the noise. What's left is what matters —
            typography, space, and motion that earns its place.
          </motion.p>
        </div>
      </div>
    </section>
  )
}

const FEATURES = [
  {
    n: '01',
    title: 'Editorial typography',
    body: 'Tight, confident headlines paired with quiet body copy. No competing for attention.',
  },
  {
    n: '02',
    title: 'Considered motion',
    body: 'Every easing curve is intentional. Motion guides — it never decorates.',
  },
  {
    n: '03',
    title: 'Honest materials',
    body: 'Real depth from light, not from gradients. Surfaces respond to your cursor.',
  },
]

function FeatureGrid() {
  return (
    <section className="bg-zinc-950 px-4 py-24 sm:px-6 sm:py-32 lg:px-8">
      <div className="mx-auto max-w-7xl">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: '-15% 0px' }}
          transition={{ duration: 0.7, ease: [0.22, 1, 0.36, 1] }}
          className="mb-16 max-w-2xl"
        >
          <p className="font-mono text-xs uppercase tracking-[0.3em] text-zinc-500">
            · 03 · Principles
          </p>
          <h2 className="mt-6 text-4xl font-medium tracking-tight text-white sm:text-5xl">
            Three rules. Nothing else.
          </h2>
        </motion.div>

        <div className="grid grid-cols-1 gap-px overflow-hidden rounded-2xl bg-white/10 sm:grid-cols-3">
          {FEATURES.map((f, i) => (
            <motion.article
              key={f.n}
              initial={{ opacity: 0, y: 30 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true, margin: '-10% 0px' }}
              transition={{
                duration: 0.7,
                delay: i * 0.08,
                ease: [0.22, 1, 0.36, 1],
              }}
              whileHover={{ y: -4 }}
              className="group relative cursor-default bg-zinc-950 p-10 transition-colors hover:bg-zinc-900/60"
            >
              <div
                aria-hidden
                className="absolute top-0 left-0 h-px w-0 bg-[var(--amber)] transition-all duration-700 group-hover:w-full"
                style={{ ['--amber' as string]: AMBER }}
              />
              <span className="font-mono text-xs tracking-wider text-zinc-600">{f.n}</span>
              <h3 className="mt-8 text-2xl font-medium tracking-tight text-white">{f.title}</h3>
              <p className="mt-3 text-sm leading-relaxed text-zinc-400">{f.body}</p>
              <ArrowRight className="mt-10 size-4 text-zinc-500 transition-all group-hover:translate-x-1 group-hover:text-[color:var(--amber)]" />
            </motion.article>
          ))}
        </div>
      </div>
    </section>
  )
}

function StatsRow() {
  const items = [
    { value: 150, suffix: '+', label: 'Shipped projects' },
    { value: 98, suffix: '%', label: 'Client retention' },
    { value: 12, suffix: 'yr', label: 'Avg. team tenure' },
    { value: 24, suffix: '/7', label: 'On-call support' },
  ]
  return (
    <section className="bg-zinc-950 px-4 py-24 sm:px-6 lg:px-8">
      <div className="mx-auto grid max-w-7xl grid-cols-2 gap-y-12 border-t border-white/10 pt-16 sm:grid-cols-4">
        {items.map((it) => (
          <div key={it.label} className="text-center">
            <div className="text-5xl font-medium tracking-tight text-white sm:text-6xl">
              <Counter to={it.value} suffix={it.suffix} />
            </div>
            <div className="mt-2 font-mono text-[10px] uppercase tracking-[0.25em] text-zinc-500">
              {it.label}
            </div>
          </div>
        ))}
      </div>
    </section>
  )
}

function ProductShowcase() {
  // Apple-style "scroll the product" parallax block
  const ref = useRef<HTMLDivElement>(null)
  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ['start end', 'end start'],
  })
  const y = useSpring(useTransform(scrollYProgress, [0, 1], [80, -80]), {
    stiffness: 80,
    damping: 20,
  })
  const rotate = useTransform(scrollYProgress, [0, 1], [-6, 6])
  const scale = useTransform(scrollYProgress, [0, 0.5, 1], [0.92, 1, 0.96])

  return (
    <section
      ref={ref}
      className="relative overflow-hidden bg-zinc-950 px-4 py-24 sm:px-6 sm:py-32 lg:px-8"
    >
      <div className="mx-auto grid max-w-7xl grid-cols-1 items-center gap-16 lg:grid-cols-2">
        <div>
          <p className="font-mono text-xs uppercase tracking-[0.3em] text-zinc-500">
            · 04 · Featured
          </p>
          <h2 className="mt-6 text-5xl font-medium tracking-tight text-white sm:text-6xl">
            Built once.
            <br />
            <span className="text-[color:var(--amber)]" style={{ ['--amber' as string]: AMBER }}>
              Felt everywhere.
            </span>
          </h2>
          <p className="mt-6 max-w-md text-lg leading-relaxed text-zinc-400">
            From the first hover to the last frame, every interaction was tuned by hand. No
            placeholders. No "we'll fix it later."
          </p>
          <div className="mt-10 flex items-center gap-4">
            <button className="group inline-flex cursor-pointer items-center gap-2 rounded-full bg-white px-8 py-4 text-sm font-semibold text-zinc-950 transition-all hover:scale-[1.02] active:scale-[0.98]">
              See the case study
              <ArrowRight className="h-4 w-4 transition-transform group-hover:translate-x-1" />
            </button>
            <button className="cursor-pointer text-sm font-medium text-zinc-400 underline-offset-4 transition-colors hover:text-white hover:underline">
              All projects
            </button>
          </div>
        </div>

        <motion.div
          style={{ y, rotate, scale }}
          className="relative aspect-[4/5] overflow-hidden rounded-3xl border border-white/10"
        >
          {/* faux product card */}
          <div
            className="absolute inset-0"
            style={{
              background:
                'linear-gradient(180deg, #18181b 0%, #09090b 60%, #000 100%)',
            }}
          />
          <div
            aria-hidden
            className="absolute inset-0"
            style={{
              background:
                'radial-gradient(600px 400px at 20% 10%, rgba(255,205,117,0.18), transparent 60%), radial-gradient(500px 300px at 80% 90%, rgba(255,255,255,0.06), transparent 60%)',
            }}
          />
          <div className="relative flex h-full flex-col justify-between p-10">
            <div className="flex items-center justify-between">
              <span className="font-mono text-xs uppercase tracking-[0.25em] text-zinc-400">
                Project · 24
              </span>
              <Sparkles className="size-4 text-[color:var(--amber)]" style={{ ['--amber' as string]: AMBER }} />
            </div>
            <div>
              <div className="font-mono text-[10px] uppercase tracking-[0.3em] text-zinc-500">
                Brand system
              </div>
              <div className="mt-2 text-3xl font-medium tracking-tight text-white sm:text-4xl">
                Northwind Type Co.
              </div>
              <div className="mt-1 text-sm text-zinc-500">2025 · Identity, Web, Motion</div>
            </div>
          </div>
        </motion.div>
      </div>
    </section>
  )
}

function FinalCTA() {
  return (
    <section className="relative overflow-hidden bg-zinc-950 px-4 py-32 sm:px-6 lg:px-8">
      <div
        aria-hidden
        className="absolute inset-0"
        style={{
          background:
            'radial-gradient(700px 350px at 50% 100%, rgba(255,205,117,0.15), transparent 60%)',
        }}
      />
      <div className="relative mx-auto max-w-3xl text-center">
        <motion.h2
          initial={{ opacity: 0, y: 30 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: '-10% 0px' }}
          transition={{ duration: 0.9, ease: [0.22, 1, 0.36, 1] }}
          className="text-6xl font-medium tracking-tight text-white sm:text-7xl lg:text-8xl"
        >
          Let's make
          <br />
          something
          <span className="text-[color:var(--amber)]" style={{ ['--amber' as string]: AMBER }}>
            .
          </span>
        </motion.h2>
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: '-10% 0px' }}
          transition={{ duration: 0.8, delay: 0.1, ease: [0.22, 1, 0.36, 1] }}
          className="mt-10"
        >
          <button className="group inline-flex cursor-pointer items-center gap-3 rounded-full bg-white px-10 py-5 text-base font-semibold text-zinc-950 transition-all hover:scale-[1.02] active:scale-[0.98]">
            Start a project
            <ArrowRight className="h-5 w-5 transition-transform group-hover:translate-x-1" />
          </button>
        </motion.div>
      </div>
    </section>
  )
}

export default function Showcase() {
  return (
    <div className="-m-4 -mt-6 sm:-m-6 lg:-m-8">
      <GlassmorphismTrustHero />
      <KineticHeadline />
      <FeatureGrid />
      <StatsRow />
      <ProductShowcase />
      <FinalCTA />
    </div>
  )
}
