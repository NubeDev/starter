import { motion } from 'framer-motion'
import { CheckCircle2 } from 'lucide-react'
import type { ProvisionResult } from '../api/bc-types'
import { useLook } from '../theme/useLook'

// "Device provisioned!" celebration — adapted from the design system's
// MatchReveal: full-screen radial glow, spring-popped content, a deterministic
// sparkle burst (pre-computed vectors, no Math.random), and the points/widgets
// counts as the payoff. Reused for the scan-flow success moment.
export function ProvisionedReveal({
  result,
  onPreview,
  onAddAnother,
}: {
  result: ProvisionResult
  // Absent when the device was commissioned as pending (no page to view).
  onPreview?: () => void
  onAddAnother: () => void
}) {
  const look = useLook()
  const accent = look.accent
  // No page_id → commissioned as pending (no widgets, nothing to view yet).
  const pending = !result.page_id

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="absolute inset-0 z-50 grid place-items-center px-8"
      style={{
        backgroundImage: `radial-gradient(80% 60% at 50% 40%, ${accent}3a, rgba(6,8,10,0.94) 72%)`,
        backdropFilter: 'blur(6px)',
      }}
    >
      {SPARKS.map((s, i) => (
        <motion.span
          key={i}
          className="pointer-events-none absolute rounded-full"
          style={{ backgroundColor: accent, height: s.size, width: s.size, left: '50%', top: '40%' }}
          initial={{ x: 0, y: 0, opacity: 0, scale: 0 }}
          animate={{ x: s.x, y: s.y, opacity: [0, 1, 0], scale: [0, 1, 0.6] }}
          transition={{ duration: 1.1, delay: 0.1 + i * 0.02, ease: 'easeOut' }}
        />
      ))}

      <motion.div
        initial={{ scale: 0.85, y: 16 }}
        animate={{ scale: 1, y: 0 }}
        transition={{ type: 'spring', stiffness: 280, damping: 22 }}
        className="relative flex flex-col items-center text-center"
      >
        <motion.span
          initial={{ scale: 0 }}
          animate={{ scale: 1 }}
          transition={{ type: 'spring', stiffness: 400, damping: 14, delay: 0.25 }}
          className="grid h-20 w-20 place-items-center rounded-full"
          style={{ backgroundColor: accent, color: '#002019', boxShadow: `0 0 40px -4px ${accent}` }}
        >
          <CheckCircle2 className="h-10 w-10" />
        </motion.span>

        <p className="mt-6 text-[13px] font-black uppercase tracking-[0.3em]" style={{ color: accent }}>
          {pending ? 'Commissioned · pending' : 'Device provisioned'}
        </p>
        <p className="mt-1 text-xl font-extrabold text-white">{result.device_id}</p>

        <div className="mt-6 flex gap-3">
          <Stat n={result.points} label="points" />
          <Stat n={result.widgets} label="widgets" />
          <Stat n={result.alarms} label="alarms" />
        </div>

        {pending && (
          <p className="mt-4 max-w-[260px] text-xs text-ink-muted">
            Not on a dashboard yet — place it on a page anytime from Devices.
          </p>
        )}

        {result.warnings.length > 0 && (
          <p className="mt-4 max-w-[260px] text-xs text-coral">{result.warnings.join(' · ')}</p>
        )}

        {onPreview && (
          <motion.button
            whileTap={{ scale: 0.96 }}
            onClick={onPreview}
            className="mt-8 rounded-2xl px-8 py-3.5 text-base font-bold"
            style={{ backgroundColor: accent, color: '#002019', boxShadow: `0 10px 34px -8px ${accent}` }}
          >
            View on dashboard
          </motion.button>
        )}
        <button onClick={onAddAnother} className={`${onPreview ? 'mt-3' : 'mt-8'} text-sm font-semibold text-white/60`}>
          Add another device
        </button>
      </motion.div>
    </motion.div>
  )
}

function Stat({ n, label }: { n: number; label: string }) {
  return (
    <div className="glass min-w-[72px] rounded-xl px-3 py-2.5">
      <p className="text-xl font-extrabold text-white">{n}</p>
      <p className="label">{label}</p>
    </div>
  )
}

// Pre-computed sparkle vectors — deterministic, SSR-safe (no Math.random).
const SPARKS = [
  { x: -120, y: -90, size: 10 },
  { x: 130, y: -70, size: 8 },
  { x: -90, y: 60, size: 7 },
  { x: 110, y: 80, size: 11 },
  { x: 0, y: -140, size: 9 },
  { x: -160, y: 10, size: 6 },
  { x: 160, y: 20, size: 8 },
  { x: 40, y: 130, size: 7 },
  { x: -50, y: -120, size: 6 },
  { x: 70, y: -110, size: 9 },
]
