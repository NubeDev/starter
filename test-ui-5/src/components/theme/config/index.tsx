import { useState } from 'react'
import { Layers, Palette as PaletteIcon, Settings, Sliders, Sparkles } from 'lucide-react'
import { useDirection } from '@/context/direction-provider'
import { useLayout } from '@/context/layout-provider'
import { cn } from '@/lib/utils'
import { useTheme } from '@/stores/theme-store'
import { Button } from '@/components/ui/button'
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from '@/components/ui/sheet'
import { useSidebar } from '@/components/ui/sidebar'
import { AdvancedTab } from './sections/advanced'
import { AppearanceTab } from './sections/appearance'
import { BrandingTab } from './sections/branding'
import { LayoutTab } from './sections/layout'

type TabId = 'appearance' | 'layout' | 'branding' | 'advanced'

const TABS: { id: TabId; label: string; icon: typeof Settings }[] = [
  { id: 'appearance', label: 'Appearance', icon: PaletteIcon },
  { id: 'layout', label: 'Layout', icon: Layers },
  { id: 'branding', label: 'Branding', icon: Sparkles },
  { id: 'advanced', label: 'Advanced', icon: Sliders },
]

export function ConfigDrawer() {
  const [tab, setTab] = useState<TabId>('appearance')
  const { setOpen } = useSidebar()
  const { resetDir } = useDirection()
  const { resetTheme } = useTheme()
  const { resetLayout } = useLayout()

  const handleReset = () => {
    setOpen(true)
    resetDir()
    resetTheme()
    resetLayout()
  }

  return (
    <Sheet>
      <SheetTrigger
        aria-label='Open theme settings'
        className='flex h-9 w-9 cursor-pointer items-center justify-center rounded-full text-[color:var(--color-muted)] transition-colors hover:bg-[color:var(--color-surface-2)]/50 hover:text-[color:var(--color-text)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--color-ring)]'
      >
        <Settings className='h-4 w-4' aria-hidden='true' />
      </SheetTrigger>
      <SheetContent className='flex flex-col'>
        <SheetHeader className='pb-0 text-start'>
          <SheetTitle>Theme Settings</SheetTitle>
          <SheetDescription>
            Adjust the appearance and layout to suit your preferences.
          </SheetDescription>
        </SheetHeader>

        <nav
          className='flex shrink-0 gap-1 border-b border-[color:var(--color-border)] px-4'
          role='tablist'
          aria-label='Theme settings sections'
        >
          {TABS.map(({ id, label, icon: Icon }) => {
            const active = tab === id
            return (
              <button
                key={id}
                type='button'
                role='tab'
                aria-selected={active}
                aria-controls={`config-panel-${id}`}
                id={`config-tab-${id}`}
                onClick={() => setTab(id)}
                className={cn(
                  'group relative flex items-center gap-1.5 px-3 py-2.5 text-xs font-medium transition-colors',
                  'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--color-ring)]',
                  active
                    ? 'text-[color:var(--color-text)]'
                    : 'text-[color:var(--color-subtle)] hover:text-[color:var(--color-muted)]',
                )}
              >
                <Icon className='size-3.5' aria-hidden='true' />
                {label}
                {active && (
                  <span
                    className='absolute inset-x-2 -bottom-px h-0.5 rounded-full bg-[color:var(--color-leaf)]'
                    aria-hidden='true'
                  />
                )}
              </button>
            )
          })}
        </nav>

        <div
          className='flex-1 overflow-y-auto px-4 py-4'
          role='tabpanel'
          id={`config-panel-${tab}`}
          aria-labelledby={`config-tab-${tab}`}
        >
          {tab === 'appearance' && <AppearanceTab />}
          {tab === 'layout' && <LayoutTab />}
          {tab === 'branding' && <BrandingTab />}
          {tab === 'advanced' && <AdvancedTab />}
        </div>

        <SheetFooter className='gap-2'>
          <Button
            variant='outline'
            onClick={handleReset}
            aria-label='Reset all settings to default values'
          >
            Reset
          </Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  )
}
