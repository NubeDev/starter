import { useEffect, useRef, useState } from 'react'
import { MeshGradient, PulsingBorder } from '@paper-design/shaders-react'
import { motion } from 'motion/react'
import { Leaf, ArrowRight, Play, Wind, Droplet, Sun } from 'lucide-react'

/**
 * Eco-themed shader hero adapted from 21st.dev/r/reuno-ui/hero.
 * Palette: forest / leaf / aqua / sun.
 * Uses @paper-design/shaders-react for the WebGL mesh gradient + pulsing border.
 */
export default function ShaderHero() {
  const containerRef = useRef<HTMLDivElement>(null)
  const [, setIsActive] = useState(false)

  useEffect(() => {
    const container = containerRef.current
    if (!container) return
    const enter = () => setIsActive(true)
    const leave = () => setIsActive(false)
    container.addEventListener('mouseenter', enter)
    container.addEventListener('mouseleave', leave)
    return () => {
      container.removeEventListener('mouseenter', enter)
      container.removeEventListener('mouseleave', leave)
    }
  }, [])

  return (
    <section
      ref={containerRef}
      className="relative min-h-[100svh] w-full overflow-hidden bg-[color:var(--color-bg)]"
    >
      {/* SVG filter defs */}
      <svg className="absolute inset-0 h-0 w-0">
        <defs>
          <filter id="glass-effect" x="-50%" y="-50%" width="200%" height="200%">
            <feTurbulence baseFrequency="0.005" numOctaves="1" result="noise" />
            <feDisplacementMap in="SourceGraphic" in2="noise" scale="0.3" />
            <feColorMatrix
              type="matrix"
              values="1 0 0 0 0.02
                      0 1 0 0 0.06
                      0 0 1 0 0.04
                      0 0 0 0.9 0"
              result="tint"
            />
          </filter>
          <filter id="text-glow" x="-50%" y="-50%" width="200%" height="200%">
            <feGaussianBlur stdDeviation="2" result="coloredBlur" />
            <feMerge>
              <feMergeNode in="coloredBlur" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
        </defs>
      </svg>

      {/* Mesh gradient layers — eco palette */}
      <MeshGradient
        className="absolute inset-0 h-full w-full"
        colors={['#06100c', '#15803d', '#4ade80', '#06b6d4', '#fde68a']}
        speed={0.28}
        distortion={0.8}
        swirl={0.6}
      />
      <MeshGradient
        className="absolute inset-0 h-full w-full opacity-40 mix-blend-screen"
        colors={['#06100c', '#ccfbf1', '#4ade80', '#fde68a']}
        speed={0.18}
        distortion={1}
        swirl={0.3}
        grainOverlay={0.15}
      />

      {/* Subtle vignette to keep text readable */}
      <div
        aria-hidden
        className="pointer-events-none absolute inset-0 z-10"
        style={{
          background:
            'radial-gradient(80% 60% at 50% 40%, transparent 0%, rgba(6,16,12,0.45) 70%, rgba(6,16,12,0.85) 100%)',
        }}
      />

      {/* Foreground content — bottom-left lockup, matches the original composition */}
      <main className="absolute bottom-10 left-6 right-6 z-20 max-w-2xl sm:left-10 md:bottom-16">
        <motion.div
          className="relative mb-6 inline-flex items-center gap-2 rounded-full border border-white/15 bg-white/[0.04] px-4 py-2 backdrop-blur-sm"
          style={{ filter: 'url(#glass-effect)' }}
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6, delay: 2.6 }}
        >
          <div className="absolute left-1 right-1 top-0 h-px rounded-full bg-gradient-to-r from-transparent via-[color:var(--color-leaf)]/50 to-transparent" />
          <Leaf className="h-3.5 w-3.5 text-[color:var(--color-leaf)]" />
          <span className="relative z-10 text-sm font-medium tracking-wide text-white/90">
            Earth Day '26 · Living Shader Edition
          </span>
        </motion.div>

        <motion.h1
          className="mb-6 text-6xl font-bold leading-none tracking-tight text-white md:text-7xl lg:text-8xl"
          initial={{ opacity: 0, y: 30 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.9, delay: 2.8 }}
        >
          <motion.span
            className="mb-2 block text-4xl font-light tracking-wider md:text-5xl lg:text-6xl"
            style={{
              background:
                'linear-gradient(135deg, #ffffff 0%, #4ade80 30%, #67e8f9 60%, #fde68a 100%)',
              WebkitBackgroundClip: 'text',
              WebkitTextFillColor: 'transparent',
              backgroundClip: 'text',
              backgroundSize: '200% 200%',
              filter: 'url(#text-glow)',
            }}
            animate={{ backgroundPosition: ['0% 50%', '100% 50%', '0% 50%'] }}
            transition={{ duration: 10, repeat: Infinity, ease: 'linear' }}
          >
            Breathe
          </motion.span>
          <span className="block font-black text-white drop-shadow-2xl">Cleaner</span>
          <span className="serif-italic block font-light text-white/85">air, water, energy.</span>
        </motion.h1>

        <motion.p
          className="mb-8 max-w-xl text-lg font-light leading-relaxed text-white/75"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6, delay: 3.1 }}
        >
          A living ecosystem for your home. Real-time air, water,
          and energy sensing — wrapped in a quiet interface that
          knows when to whisper and when to speak.
        </motion.p>

        <motion.div
          className="flex flex-wrap items-center gap-4"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6, delay: 3.3 }}
        >
          <motion.button
            className="group inline-flex cursor-pointer items-center gap-2 rounded-full bg-gradient-to-r from-[color:var(--color-leaf)] via-[color:var(--color-leaf-2)] to-[color:var(--color-aqua)] px-8 py-4 text-sm font-semibold text-[color:var(--color-bg)] shadow-[0_10px_40px_-10px_rgba(74,222,128,0.6)] transition-all"
            whileHover={{ scale: 1.04 }}
            whileTap={{ scale: 0.96 }}
          >
            Start your free audit
            <ArrowRight className="h-4 w-4 transition-transform group-hover:translate-x-0.5" />
          </motion.button>
          <motion.button
            className="inline-flex cursor-pointer items-center gap-2 rounded-full border-2 border-white/25 bg-transparent px-8 py-4 text-sm font-medium text-white backdrop-blur-sm transition-all hover:border-[color:var(--color-aqua)]/50 hover:bg-white/10 hover:text-[color:var(--color-mist)]"
            whileHover={{ scale: 1.04 }}
            whileTap={{ scale: 0.96 }}
          >
            <Play className="h-4 w-4 fill-current" />
            See it breathe
          </motion.button>
        </motion.div>

        {/* Vital signs row */}
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.6, delay: 3.5 }}
          className="mt-10 flex flex-wrap items-center gap-x-6 gap-y-3 text-xs uppercase tracking-[0.2em] text-white/60"
        >
          <span className="flex items-center gap-2">
            <Wind className="h-3.5 w-3.5 text-[color:var(--color-aqua)]" />
            AQI 12
          </span>
          <span className="h-3 w-px bg-white/20" />
          <span className="flex items-center gap-2">
            <Droplet className="h-3.5 w-3.5 text-[color:var(--color-sky)]" />
            pH 7.2
          </span>
          <span className="h-3 w-px bg-white/20" />
          <span className="flex items-center gap-2">
            <Sun className="h-3.5 w-3.5 text-[color:var(--color-sun)]" />
            +42 kWh today
          </span>
        </motion.div>
      </main>

      {/* Pulsing border medallion — eco palette + rotating tagline */}
      <div className="absolute bottom-8 right-8 z-30 hidden md:block">
        <div className="relative flex h-20 w-20 items-center justify-center">
          <PulsingBorder
            colors={['#4ade80', '#22c55e', '#67e8f9', '#fde68a', '#15803d']}
            colorBack="#00000000"
            speed={1.4}
            roundness={1}
            thickness={0.1}
            softness={0.2}
            intensity={5}
            spots={5}
            spotSize={0.1}
            pulse={0.1}
            smoke={0.5}
            smokeSize={4}
            scale={0.65}
            rotation={0}
            style={{ width: '60px', height: '60px', borderRadius: '50%' }}
          />
          <motion.svg
            className="absolute inset-0 h-full w-full"
            viewBox="0 0 100 100"
            animate={{ rotate: 360 }}
            transition={{ duration: 22, repeat: Infinity, ease: 'linear' }}
            style={{ transform: 'scale(1.6)' }}
          >
            <defs>
              <path
                id="circle-eco"
                d="M 50, 50 m -38, 0 a 38,38 0 1,1 76,0 a 38,38 0 1,1 -76,0"
              />
            </defs>
            <text className="fill-white/80 text-[8px] font-medium uppercase tracking-[0.25em]">
              <textPath href="#circle-eco" startOffset="0%">
                · Breathe · Drink · Grow · Verdant Living Systems
              </textPath>
            </text>
          </motion.svg>
        </div>
      </div>
    </section>
  )
}
