import { motion } from 'framer-motion'
import { LogOut, Wifi } from 'lucide-react'
import { useAuth } from '../auth/authContext'
import { useAppTheme } from '../theme/themeContext'
import { STATUSES } from '../theme/statuses'
import { useLook } from '../theme/useLook'

// Floating status bar: live connection dot + the signed-in principal + logout.
// Sits above the page content; the page bodies pad past it via their pt-14.
export function TopBar() {
  const { user, logout } = useAuth()
  const { status } = useAppTheme()
  const look = useLook()
  const s = status ? STATUSES[status] : null

  return (
    <div className="pointer-events-none absolute inset-x-0 top-0 z-30 flex justify-center px-margin pt-3 sm:pt-9">
      <div className="glass-strong pointer-events-auto flex w-full items-center gap-2 rounded-full px-3 py-1.5 shadow-glass">
        <span
          className="h-2 w-2 rounded-full"
          style={{ backgroundColor: s?.accent ?? look.accent, animation: 'var(--animate-breath)' }}
        />
        <Wifi className="h-3.5 w-3.5 text-ink-muted" />
        <span className="flex-1 truncate text-xs font-semibold text-ink">{user?.email ?? 'Connected'}</span>
        <span className="label">{s?.label ?? ''}</span>
        <motion.button
          whileTap={{ scale: 0.9 }}
          onClick={() => void logout()}
          aria-label="Log out"
          className="grid h-7 w-7 cursor-pointer place-items-center rounded-full text-ink-muted hover:text-ink"
        >
          <LogOut className="h-4 w-4" />
        </motion.button>
      </div>
    </div>
  )
}
