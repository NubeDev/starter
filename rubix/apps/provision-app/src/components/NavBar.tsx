import { useState } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { Plus, MoreHorizontal, type LucideIcon } from 'lucide-react'
import { primaryPages, secondaryPages, type Tab } from '../pages/registry'

export type { Tab }

// Registry-driven bottom nav with a center FAB. Copied verbatim from the design
// system; the FAB action here jumps to the Scan tab (the app's fast action).
export function NavBar({
  active,
  onChange,
  onFab,
  accent,
}: {
  active: Tab
  onChange: (t: Tab) => void
  onFab: () => void
  accent: string
}) {
  const [overflowOpen, setOverflowOpen] = useState(false)
  const primary = primaryPages()
  const secondary = secondaryPages()

  // Split the icons evenly around the center FAB so it stays visually centered.
  const rightSlots = secondary.length > 0 ? 1 : 0 // the "•••" button
  const leftCount = Math.round((primary.length + rightSlots) / 2)
  const left = primary.slice(0, leftCount)
  const right = primary.slice(leftCount)

  function go(t: Tab) {
    onChange(t)
    setOverflowOpen(false)
  }

  return (
    <div className="pointer-events-none absolute inset-x-0 bottom-0 z-40 flex flex-col items-center gap-2 pb-6">
      {/* overflow rail — secondary pages, revealed above the bar */}
      <AnimatePresence>
        {overflowOpen && secondary.length > 0 && (
          <motion.div
            initial={{ opacity: 0, y: 12, scale: 0.96 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 12, scale: 0.96 }}
            transition={{ type: 'spring', stiffness: 380, damping: 28 }}
            className="glass-strong pointer-events-auto flex items-center gap-1 rounded-full px-2 py-2 shadow-glass"
          >
            {secondary.map((p) => (
              <NavButton
                key={p.tab}
                icon={p.icon}
                label={p.label}
                active={active === p.tab}
                accent={accent}
                onClick={() => go(p.tab)}
              />
            ))}
          </motion.div>
        )}
      </AnimatePresence>

      <nav className="glass-strong pointer-events-auto flex items-center gap-1 rounded-full px-2 py-2 shadow-glass">
        {left.map((p) => (
          <NavButton
            key={p.tab}
            icon={p.icon}
            label={p.label}
            active={active === p.tab}
            accent={accent}
            onClick={() => go(p.tab)}
          />
        ))}

        {/* center FAB — jump straight to Scan, the app's fast action */}
        <motion.button
          whileTap={{ scale: 0.9 }}
          transition={{ type: 'spring', stiffness: 400, damping: 22 }}
          onClick={onFab}
          aria-label="Scan a device"
          style={{ backgroundColor: accent, boxShadow: `0 8px 32px -8px ${accent}` }}
          className="mx-1 grid h-14 w-14 cursor-pointer place-items-center rounded-full text-black"
        >
          <Plus strokeWidth={2.75} className="h-7 w-7" />
        </motion.button>

        {right.map((p) => (
          <NavButton
            key={p.tab}
            icon={p.icon}
            label={p.label}
            active={active === p.tab}
            accent={accent}
            onClick={() => go(p.tab)}
          />
        ))}

        {/* overflow toggle — only when there are secondary pages to show */}
        {secondary.length > 0 && (
          <NavButton
            icon={MoreHorizontal}
            label="More"
            active={overflowOpen || secondary.some((p) => p.tab === active)}
            accent={accent}
            onClick={() => setOverflowOpen((v) => !v)}
          />
        )}
      </nav>
    </div>
  )
}

function NavButton({
  icon: Icon,
  label,
  active,
  accent,
  onClick,
}: {
  icon: LucideIcon
  label: string
  active: boolean
  accent: string
  onClick: () => void
}) {
  return (
    <motion.button
      whileTap={{ scale: 0.88 }}
      onClick={onClick}
      aria-label={label}
      aria-current={active ? 'page' : undefined}
      className="relative grid h-12 w-12 cursor-pointer place-items-center rounded-full"
    >
      <Icon
        strokeWidth={active ? 2.5 : 2}
        className="h-6 w-6 transition-colors"
        style={{ color: active ? accent : '#7c8a8a' }}
      />
      {active && (
        <motion.span
          layoutId="nav-dot"
          className="absolute -bottom-0.5 h-1 w-1 rounded-full"
          style={{ backgroundColor: accent }}
        />
      )}
    </motion.button>
  )
}
