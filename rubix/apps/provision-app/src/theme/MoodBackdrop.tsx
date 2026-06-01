import { AnimatePresence, motion } from 'framer-motion'
import { useLook } from './useLook'

// Animated ambient backdrop = theme base gradient + an optional status tint
// layered on top. Cross-fades whenever the theme or live status changes.
// Adapted from the design system's MoodBackdrop (mood → device status).
export function MoodBackdrop() {
  const look = useLook()
  const layers = [...look.baseGradient, ...(look.statusTint ? [look.statusTint] : [])]
  const key = `${look.themeAccent}|${look.statusAccent ?? 'none'}`

  return (
    <div
      className="pointer-events-none absolute inset-0 -z-10 overflow-hidden"
      style={{ backgroundColor: look.base }}
    >
      <AnimatePresence mode="sync">
        <motion.div
          key={key}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.7, ease: 'easeInOut' }}
          className="absolute inset-0"
          style={{ backgroundImage: layers.join(', ') }}
        />
      </AnimatePresence>
    </div>
  )
}
