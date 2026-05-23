import React from 'react'
import { motion, useMotionValue, useSpring, useTransform } from 'motion/react'
import { useEffect, useRef } from 'react'
import {
  ArrowRight,
  Play,
  Wind,
  Droplet,
  Leaf,
  Sun,
  Sprout,
  Recycle,
  TreePine,
  Waves,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'

const CLIENTS = [
  { name: 'Aerogrow', icon: Sprout },
  { name: 'PureFlow', icon: Droplet },
  { name: 'Verdant', icon: TreePine },
  { name: 'Solstice', icon: Sun },
  { name: 'BlueCycle', icon: Recycle },
  { name: 'Tidewell', icon: Waves },
]

const fadeUp = {
  hidden: { opacity: 0, y: 28, filter: 'blur(8px)' },
  show: (i: number) => ({
    opacity: 1,
    y: 0,
    filter: 'blur(0px)',
    transition: { duration: 0.9, delay: 2.4 + 0.1 * i, ease: [0.22, 1, 0.36, 1] },
  }),
}

function useAnimatedNumber(target: number, delay = 0) {
  const mv = useMotionValue(0)
  const spring = useSpring(mv, { stiffness: 60, damping: 18 })
  const rounded = useTransform(spring, (v) =>
    v < 100 ? v.toFixed(1) : Math.round(v).toLocaleString(),
  )
  useEffect(() => {
    const t = setTimeout(() => mv.set(target), delay * 1000)
    return () => clearTimeout(t)
  }, [target, delay, mv])
  return rounded
}

function AmbientOrbs() {
  // Slow drifting blobs — water/air feel
  return (
    <div className="pointer-events-none absolute inset-0 overflow-hidden">
      <motion.div
        className="absolute h-[520px] w-[520px] rounded-full"
        style={{
          background:
            'radial-gradient(circle, rgba(74,222,128,0.22), transparent 60%)',
          top: '5%',
          left: '60%',
          filter: 'blur(40px)',
        }}
        animate={{ x: [0, 40, -20, 0], y: [0, -30, 20, 0] }}
        transition={{ duration: 22, repeat: Infinity, ease: 'easeInOut' }}
      />
      <motion.div
        className="absolute h-[480px] w-[480px] rounded-full"
        style={{
          background:
            'radial-gradient(circle, rgba(103,232,249,0.18), transparent 60%)',
          top: '40%',
          left: '-5%',
          filter: 'blur(40px)',
        }}
        animate={{ x: [0, 30, -40, 0], y: [0, 30, -10, 0] }}
        transition={{ duration: 26, repeat: Infinity, ease: 'easeInOut' }}
      />
      <motion.div
        className="absolute h-[340px] w-[340px] rounded-full"
        style={{
          background:
            'radial-gradient(circle, rgba(253,230,138,0.10), transparent 60%)',
          top: '60%',
          left: '70%',
          filter: 'blur(40px)',
        }}
        animate={{ x: [0, -30, 15, 0], y: [0, 20, -30, 0] }}
        transition={{ duration: 30, repeat: Infinity, ease: 'easeInOut' }}
      />
    </div>
  )
}

function LivingAirRing() {
  // Animated SVG halo around the AQI stat
  const ref = useRef<SVGSVGElement>(null)
  return (
    <svg
      ref={ref}
      viewBox="0 0 200 200"
      className="absolute inset-0 h-full w-full"
    >
      <defs>
        <linearGradient id="airring" x1="0" x2="1" y1="0" y2="1">
          <stop offset="0%" stopColor="#67e8f9" />
          <stop offset="50%" stopColor="#4ade80" />
          <stop offset="100%" stopColor="#fde68a" />
        </linearGradient>
      </defs>
      <motion.circle
        cx="100"
        cy="100"
        r="88"
        fill="none"
        stroke="url(#airring)"
        strokeWidth="1.5"
        strokeDasharray="4 8"
        animate={{ rotate: 360 }}
        transition={{ duration: 60, repeat: Infinity, ease: 'linear' }}
        style={{ transformOrigin: '100px 100px' }}
      />
      <motion.circle
        cx="100"
        cy="100"
        r="78"
        fill="none"
        stroke="rgba(74,222,128,0.15)"
        strokeWidth="0.5"
        animate={{ scale: [1, 1.04, 1] }}
        transition={{ duration: 4, repeat: Infinity, ease: 'easeInOut' }}
        style={{ transformOrigin: '100px 100px' }}
      />
    </svg>
  )
}

const StatItem = ({
  value,
  label,
}: {
  value: React.ReactNode
  label: string
}) => (
  <div className="flex cursor-default flex-col items-center justify-center transition-transform hover:-translate-y-0.5">
    <span className="tabular text-2xl font-semibold tracking-tight text-white">{value}</span>
    <span className="mt-1 text-[10px] font-medium uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
      {label}
    </span>
  </div>
)

export default function GlassmorphismTrustHero() {
  const aqi = useAnimatedNumber(12, 3.2)
  const ph = useAnimatedNumber(7.2, 3.4)

  return (
    <section className="relative w-full overflow-hidden bg-[color:var(--color-bg)] text-white">
      <AmbientOrbs />

      {/* soft sun shaft from top */}
      <motion.div
        aria-hidden
        initial={{ opacity: 0, scaleY: 0.6 }}
        animate={{ opacity: 1, scaleY: 1 }}
        transition={{ duration: 2, delay: 2.5, ease: [0.22, 1, 0.36, 1] }}
        className="pointer-events-none absolute -top-20 left-1/2 z-0 h-[700px] w-[1100px] -translate-x-1/2 origin-top"
        style={{
          background:
            'conic-gradient(from 200deg at 50% 50%, rgba(253,230,138,0.14), transparent 25%, rgba(74,222,128,0.10) 50%, transparent 75%, rgba(103,232,249,0.10))',
          filter: 'blur(60px)',
        }}
      />

      <div className="relative z-10 mx-auto max-w-7xl px-4 pt-28 pb-12 sm:px-6 md:pt-36 md:pb-20 lg:px-8">
        <div className="grid grid-cols-1 items-start gap-12 lg:grid-cols-12 lg:gap-10">
          {/* LEFT */}
          <div className="flex flex-col justify-center space-y-8 pt-4 lg:col-span-7">
            <motion.div custom={0} initial="hidden" animate="show" variants={fadeUp}>
              <Badge variant="live">
                Earth Day '26 · Carbon negative
              </Badge>
            </motion.div>

            <motion.h1
              custom={1}
              initial="hidden"
              animate="show"
              variants={fadeUp}
              className="text-5xl font-medium leading-[0.92] tracking-[-0.04em] sm:text-6xl lg:text-7xl xl:text-[108px]"
            >
              Breathe{' '}
              <span className="serif-italic text-[color:var(--color-leaf)]">cleaner.</span>
              <br />
              Drink{' '}
              <span className="serif-italic text-[color:var(--color-aqua)]">clearer.</span>
              <br />
              <span className="bg-gradient-to-br from-white via-[color:var(--color-mist)] to-[color:var(--color-leaf)] bg-clip-text text-transparent">
                Live lighter on Earth.
              </span>
            </motion.h1>

            <motion.p
              custom={2}
              initial="hidden"
              animate="show"
              variants={fadeUp}
              className="max-w-xl text-lg leading-relaxed text-[color:var(--color-muted)]"
            >
              A connected ecosystem for indoor air, water, and energy.
              Real-time sensing, plant-powered filtration, and the
              calmest dashboard you've ever owned.
            </motion.p>

            <motion.div
              custom={3}
              initial="hidden"
              animate="show"
              variants={fadeUp}
              className="flex flex-col gap-3 sm:flex-row"
            >
              <Button size="lg" variant="leaf">
                Start your free audit
                <ArrowRight className="h-4 w-4 transition-transform group-hover:translate-x-1" />
              </Button>
              <Button size="lg" variant="ghost">
                <Play className="h-4 w-4 fill-current" />
                See it breathe
              </Button>
            </motion.div>

            {/* Vital signs strip */}
            <motion.div
              custom={4}
              initial="hidden"
              animate="show"
              variants={fadeUp}
              className="flex flex-wrap items-center gap-x-6 gap-y-3 pt-2 text-xs uppercase tracking-[0.2em] text-[color:var(--color-subtle)]"
            >
              <span className="flex items-center gap-2">
                <Wind className="h-3.5 w-3.5 text-[color:var(--color-aqua)]" />
                AQI 12
              </span>
              <span className="h-3 w-px bg-white/10" />
              <span className="flex items-center gap-2">
                <Droplet className="h-3.5 w-3.5 text-[color:var(--color-sky)]" />
                pH 7.2
              </span>
              <span className="h-3 w-px bg-white/10" />
              <span className="flex items-center gap-2">
                <Leaf className="h-3.5 w-3.5 text-[color:var(--color-leaf)]" />
                +18 plants today
              </span>
            </motion.div>
          </div>

          {/* RIGHT */}
          <div className="space-y-6 lg:col-span-5 lg:mt-12">
            <motion.div
              custom={5}
              initial="hidden"
              animate="show"
              variants={fadeUp}
              className="hairline glass relative overflow-hidden rounded-3xl p-8 shadow-2xl"
            >
              <div className="pointer-events-none absolute -right-16 -top-16 h-64 w-64 rounded-full bg-[color:var(--color-leaf)]/10 blur-3xl" />

              <div className="relative z-10">
                <div className="mb-8 flex items-center gap-4">
                  <div className="relative flex h-16 w-16 items-center justify-center rounded-2xl bg-[color:var(--color-leaf)]/10 ring-1 ring-[color:var(--color-leaf)]/30">
                    <LivingAirRing />
                    <Wind className="relative z-10 h-6 w-6 text-[color:var(--color-leaf)]" />
                  </div>
                  <div>
                    <div className="tabular flex items-baseline gap-1 text-3xl font-semibold tracking-tight text-white">
                      <motion.span>{aqi}</motion.span>
                      <span className="text-base text-[color:var(--color-subtle)]">AQI</span>
                    </div>
                    <div className="text-sm text-[color:var(--color-muted)]">
                      Excellent · indoor air
                    </div>
                  </div>
                </div>

                <div className="mb-8 space-y-3">
                  <div className="flex justify-between text-sm">
                    <span className="text-[color:var(--color-muted)]">Oxygen efficiency</span>
                    <span className="font-medium text-white">98%</span>
                  </div>
                  <div className="h-2 w-full overflow-hidden rounded-full bg-[color:var(--color-bg-2)]">
                    <motion.div
                      initial={{ width: 0 }}
                      animate={{ width: '98%' }}
                      transition={{ duration: 1.6, delay: 3.0, ease: [0.22, 1, 0.36, 1] }}
                      className="h-full rounded-full bg-gradient-to-r from-[color:var(--color-leaf)] to-[color:var(--color-aqua)]"
                    />
                  </div>
                </div>

                <div className="mb-6 h-px w-full bg-white/10" />

                <div className="grid grid-cols-3 gap-4 text-center">
                  <StatItem value="0" label="VOCs" />
                  <div className="mx-auto h-full w-px bg-white/10" />
                  <StatItem value={<motion.span>{ph}</motion.span>} label="pH" />
                  <div className="mx-auto h-full w-px bg-white/10" />
                  <StatItem value="24°" label="Climate" />
                </div>

                <div className="mt-8 flex flex-wrap gap-2">
                  <Badge variant="live">SENSING</Badge>
                  <Badge variant="leaf">
                    <Leaf className="h-3 w-3" />
                    ECO MODE
                  </Badge>
                  <Badge variant="aqua">
                    <Droplet className="h-3 w-3" />
                    FILTERED
                  </Badge>
                </div>
              </div>
            </motion.div>

            <motion.div
              custom={6}
              initial="hidden"
              animate="show"
              variants={fadeUp}
              className="glass relative overflow-hidden rounded-3xl py-8"
            >
              <h3 className="mb-6 px-8 text-xs font-medium uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
                Trusted by climate-positive teams
              </h3>
              <div
                className="relative flex overflow-hidden"
                style={{
                  maskImage:
                    'linear-gradient(to right, transparent, black 15%, black 85%, transparent)',
                  WebkitMaskImage:
                    'linear-gradient(to right, transparent, black 15%, black 85%, transparent)',
                }}
              >
                <div className="animate-marquee flex gap-12 whitespace-nowrap px-4">
                  {[...CLIENTS, ...CLIENTS, ...CLIENTS].map((client, i) => (
                    <div
                      key={i}
                      className="flex cursor-default items-center gap-2 opacity-50 grayscale transition-all hover:scale-105 hover:opacity-100 hover:grayscale-0"
                    >
                      <client.icon className="h-5 w-5 text-[color:var(--color-leaf)]" />
                      <span className="text-base font-bold tracking-tight text-white">
                        {client.name}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            </motion.div>
          </div>
        </div>
      </div>
    </section>
  )
}
