import { type SVGProps } from 'react'
import { Root as Radio, Item } from '@radix-ui/react-radio-group'
import { CircleCheck, RotateCcw, Settings } from 'lucide-react'
import { IconDir } from '@/assets/custom/icon-dir'
import { IconLayoutCompact } from '@/assets/custom/icon-layout-compact'
import { IconLayoutDefault } from '@/assets/custom/icon-layout-default'
import { IconLayoutFull } from '@/assets/custom/icon-layout-full'
import { IconSidebarFloating } from '@/assets/custom/icon-sidebar-floating'
import { IconSidebarInset } from '@/assets/custom/icon-sidebar-inset'
import { IconSidebarSidebar } from '@/assets/custom/icon-sidebar-sidebar'
import { IconThemeDark } from '@/assets/custom/icon-theme-dark'
import { IconThemeLight } from '@/assets/custom/icon-theme-light'
import { IconThemeSystem } from '@/assets/custom/icon-theme-system'
import { useDirection } from '@/context/direction-provider'
import { type Collapsible, useLayout } from '@/context/layout-provider'
import { cn } from '@/lib/utils'
import {
  DEFAULT_FONT,
  DEFAULT_MODE,
  DEFAULT_PALETTE,
  FONT_STACKS,
  useTheme,
  type Font,
  type Mode,
  type Palette,
} from '@/stores/theme-store'
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
import { useSidebar } from './ui/sidebar'

export function ConfigDrawer() {
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
        className='flex h-9 w-9 cursor-pointer items-center justify-center rounded-full text-[color:var(--color-muted)] transition-colors hover:bg-white/[0.04] hover:text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--color-ring)]'
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
        <div className='space-y-6 overflow-y-auto px-4'>
          <ThemeConfig />
          <PaletteConfig />
          <FontConfig />
          <SidebarConfig />
          <LayoutConfig />
          <DirConfig />
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

function SectionTitle({
  title,
  showReset = false,
  onReset,
  resetAriaLabel,
  className,
}: {
  title: string
  showReset?: boolean
  onReset?: () => void
  resetAriaLabel?: string
  className?: string
}) {
  return (
    <div
      className={cn(
        'mb-2 flex items-center gap-2 text-sm font-semibold text-[color:var(--color-muted)]',
        className,
      )}
    >
      {title}
      {showReset && onReset && (
        <Button
          type='button'
          size='icon'
          variant='ghost'
          className='size-4 rounded-full'
          onClick={onReset}
          aria-label={resetAriaLabel}
        >
          <RotateCcw className='size-3' />
        </Button>
      )}
    </div>
  )
}

function RadioGroupItem({
  item,
  isTheme = false,
}: {
  item: {
    value: string
    label: string
    icon: (props: SVGProps<SVGSVGElement>) => React.ReactElement
  }
  isTheme?: boolean
}) {
  return (
    <Item
      value={item.value}
      className={cn('group outline-none', 'transition duration-200 ease-in')}
      aria-label={`Select ${item.label.toLowerCase()}`}
      aria-describedby={`${item.value}-description`}
    >
      <div
        className={cn(
          'relative rounded-[6px] ring-[1px] ring-[color:var(--color-border)]',
          'group-data-[state=checked]:shadow-2xl group-data-[state=checked]:ring-[color:var(--color-leaf)]',
          'group-focus-visible:ring-2',
        )}
        role='img'
        aria-hidden='false'
        aria-label={`${item.label} option preview`}
      >
        <CircleCheck
          className={cn(
            'size-6 fill-[color:var(--color-leaf)] stroke-white',
            'group-data-[state=unchecked]:hidden',
            'absolute top-0 right-0 translate-x-1/2 -translate-y-1/2',
          )}
          aria-hidden='true'
        />
        <item.icon
          className={cn(
            !isTheme &&
              'fill-[color:var(--color-leaf)] stroke-[color:var(--color-leaf)] group-data-[state=unchecked]:fill-[color:var(--color-muted)] group-data-[state=unchecked]:stroke-[color:var(--color-muted)]',
          )}
          aria-hidden='true'
        />
      </div>
      <div className='mt-1 text-xs' id={`${item.value}-description`} aria-live='polite'>
        {item.label}
      </div>
    </Item>
  )
}

function ThemeConfig() {
  const { mode, setMode } = useTheme()
  return (
    <div>
      <SectionTitle
        title='Theme'
        showReset={mode !== DEFAULT_MODE}
        onReset={() => setMode(DEFAULT_MODE)}
        resetAriaLabel='Reset theme preference to default'
      />
      <Radio
        value={mode}
        onValueChange={(v) => setMode(v as Mode)}
        className='grid w-full max-w-md grid-cols-3 gap-4'
        aria-label='Select theme preference'
      >
        {[
          { value: 'system', label: 'System', icon: IconThemeSystem },
          { value: 'light',  label: 'Light',  icon: IconThemeLight },
          { value: 'dark',   label: 'Dark',   icon: IconThemeDark },
        ].map((item) => (
          <RadioGroupItem key={item.value} item={item} isTheme />
        ))}
      </Radio>
    </div>
  )
}

const PALETTE_SWATCHES: Record<Palette, string> = {
  nube: 'linear-gradient(135deg,#339999,#184171)',
  ocean: 'linear-gradient(135deg,#3b82f6,#1e3a8a)',
  sunset: 'linear-gradient(135deg,#f97316,#b21368)',
}

function PaletteConfig() {
  const { palette, setPalette } = useTheme()
  return (
    <div>
      <SectionTitle
        title='Palette'
        showReset={palette !== DEFAULT_PALETTE}
        onReset={() => setPalette(DEFAULT_PALETTE)}
        resetAriaLabel='Reset palette to default'
      />
      <Radio
        value={palette}
        onValueChange={(v) => setPalette(v as Palette)}
        className='grid w-full max-w-md grid-cols-3 gap-4'
        aria-label='Select palette'
      >
        {(Object.keys(PALETTE_SWATCHES) as Palette[]).map((id) => (
          <Item
            key={id}
            value={id}
            className='group outline-none'
            aria-label={`Select ${id} palette`}
          >
            <div
              className={cn(
                'relative rounded-[6px] ring-[1px] ring-[color:var(--color-border)] p-3',
                'group-data-[state=checked]:shadow-2xl group-data-[state=checked]:ring-[color:var(--color-leaf)]',
              )}
            >
              <CircleCheck
                className={cn(
                  'size-6 fill-[color:var(--color-leaf)] stroke-white',
                  'group-data-[state=unchecked]:hidden',
                  'absolute top-0 right-0 translate-x-1/2 -translate-y-1/2',
                )}
              />
              <div
                className='h-10 w-full rounded'
                style={{ background: PALETTE_SWATCHES[id] }}
              />
            </div>
            <div className='mt-1 text-xs capitalize'>{id}</div>
          </Item>
        ))}
      </Radio>
    </div>
  )
}

function FontConfig() {
  const { font, setFont } = useTheme()
  const fonts: Font[] = ['geist', 'inter', 'manrope', 'system']
  return (
    <div>
      <SectionTitle
        title='Font'
        showReset={font !== DEFAULT_FONT}
        onReset={() => setFont(DEFAULT_FONT)}
        resetAriaLabel='Reset font to default'
      />
      <Radio
        value={font}
        onValueChange={(v) => setFont(v as Font)}
        className='grid w-full max-w-md grid-cols-2 gap-2'
        aria-label='Select font'
      >
        {fonts.map((id) => (
          <Item
            key={id}
            value={id}
            className='group outline-none'
            aria-label={`Select ${id} font`}
          >
            <div
              className={cn(
                'relative rounded-[6px] ring-[1px] ring-[color:var(--color-border)] px-3 py-2 text-left',
                'group-data-[state=checked]:shadow-2xl group-data-[state=checked]:ring-[color:var(--color-leaf)]',
              )}
              style={{ fontFamily: FONT_STACKS[id] }}
            >
              <CircleCheck
                className={cn(
                  'size-5 fill-[color:var(--color-leaf)] stroke-white',
                  'group-data-[state=unchecked]:hidden',
                  'absolute top-0 right-0 translate-x-1/2 -translate-y-1/2',
                )}
              />
              <div className='text-sm font-medium capitalize text-white'>{id}</div>
              <div className='text-[10px] text-[color:var(--color-subtle)]'>Ag 123</div>
            </div>
          </Item>
        ))}
      </Radio>
    </div>
  )
}

function SidebarConfig() {
  const { defaultVariant, variant, setVariant } = useLayout()
  return (
    <div className='max-md:hidden'>
      <SectionTitle
        title='Sidebar'
        showReset={defaultVariant !== variant}
        onReset={() => setVariant(defaultVariant)}
        resetAriaLabel='Reset sidebar style to default'
      />
      <Radio
        value={variant}
        onValueChange={(v) => setVariant(v as typeof variant)}
        className='grid w-full max-w-md grid-cols-3 gap-4'
        aria-label='Select sidebar style'
      >
        {[
          { value: 'inset',    label: 'Inset',    icon: IconSidebarInset },
          { value: 'floating', label: 'Floating', icon: IconSidebarFloating },
          { value: 'sidebar',  label: 'Sidebar',  icon: IconSidebarSidebar },
        ].map((item) => (
          <RadioGroupItem key={item.value} item={item} />
        ))}
      </Radio>
    </div>
  )
}

function LayoutConfig() {
  const { open, setOpen } = useSidebar()
  const { defaultCollapsible, collapsible, setCollapsible } = useLayout()

  const radioState = open ? 'default' : collapsible

  return (
    <div className='max-md:hidden'>
      <SectionTitle
        title='Layout'
        showReset={radioState !== 'default'}
        onReset={() => {
          setOpen(true)
          setCollapsible(defaultCollapsible)
        }}
        resetAriaLabel='Reset layout options to default'
      />
      <Radio
        value={radioState}
        onValueChange={(v) => {
          if (v === 'default') {
            setOpen(true)
            return
          }
          setOpen(false)
          setCollapsible(v as Collapsible)
        }}
        className='grid w-full max-w-md grid-cols-3 gap-4'
        aria-label='Select layout style'
      >
        {[
          { value: 'default',   label: 'Default',     icon: IconLayoutDefault },
          { value: 'icon',      label: 'Compact',     icon: IconLayoutCompact },
          { value: 'offcanvas', label: 'Full layout', icon: IconLayoutFull },
        ].map((item) => (
          <RadioGroupItem key={item.value} item={item} />
        ))}
      </Radio>
    </div>
  )
}

function DirConfig() {
  const { defaultDir, dir, setDir } = useDirection()
  return (
    <div>
      <SectionTitle
        title='Direction'
        showReset={defaultDir !== dir}
        onReset={() => setDir(defaultDir)}
        resetAriaLabel='Reset text direction to default'
      />
      <Radio
        value={dir}
        onValueChange={(v) => setDir(v as typeof dir)}
        className='grid w-full max-w-md grid-cols-3 gap-4'
        aria-label='Select site direction'
      >
        {[
          {
            value: 'ltr',
            label: 'Left to Right',
            icon: (props: SVGProps<SVGSVGElement>) => <IconDir dir='ltr' {...props} />,
          },
          {
            value: 'rtl',
            label: 'Right to Left',
            icon: (props: SVGProps<SVGSVGElement>) => <IconDir dir='rtl' {...props} />,
          },
        ].map((item) => (
          <RadioGroupItem key={item.value} item={item} />
        ))}
      </Radio>
    </div>
  )
}
