// Three-segment pill theme switcher (light / system / dark) inspired
// by https://www.kibo-ui.com/components/theme-switcher. Drives
// `useTheme().setMode` and uses `motion/react`'s `layoutId` to slide
// the highlight pill between segments.

import { Monitor, Moon, Sun } from 'lucide-react'
import { motion } from 'motion/react'
import { useIntl } from 'react-intl'
import { useTheme, type Mode } from '@/stores/theme-store'
import { cn } from '@/lib/utils'

const THEMES: { key: Mode; icon: typeof Sun; labelKey: string }[] = [
  { key: 'system', icon: Monitor, labelKey: 'mode.system' },
  { key: 'light',  icon: Sun,     labelKey: 'mode.light' },
  { key: 'dark',   icon: Moon,    labelKey: 'mode.dark' },
]

interface ThemeSwitcherProps {
  className?: string
}

export function ThemeSwitcher({ className }: ThemeSwitcherProps) {
  const { mode, setMode } = useTheme()
  const intl = useIntl()
  return (
    <div
      className={cn(
        'relative isolate flex h-8 items-center rounded-full bg-[color:var(--color-surface)] p-1 ring-1 ring-[color:var(--color-border)]',
        className,
      )}
      role='radiogroup'
      aria-label={intl.formatMessage({ id: 'header.themeToggle' })}
    >
      {THEMES.map(({ key, icon: Icon, labelKey }) => {
        const isActive = mode === key
        const label = intl.formatMessage({ id: labelKey })
        return (
          <button
            key={key}
            type='button'
            role='radio'
            aria-checked={isActive}
            aria-label={label}
            title={label}
            onClick={() => setMode(key)}
            className='relative h-6 w-6 cursor-pointer rounded-full outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--color-ring)]'
          >
            {isActive && (
              <motion.div
                layoutId='themeSwitcherActive'
                transition={{ type: 'spring', duration: 0.5 }}
                className='absolute inset-0 rounded-full bg-[color:var(--color-surface-2)]'
              />
            )}
            <Icon
              className={cn(
                'relative z-10 m-auto h-4 w-4',
                isActive
                  ? 'text-[color:var(--color-text)]'
                  : 'text-[color:var(--color-muted)]',
              )}
            />
          </button>
        )
      })}
    </div>
  )
}
