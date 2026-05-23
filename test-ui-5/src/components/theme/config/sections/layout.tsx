import { type SVGProps } from 'react'
import { Root as Radio, Item } from '@radix-ui/react-radio-group'
import { CircleCheck, PanelLeft, PanelTop } from 'lucide-react'
import { IconDir } from '@/assets/custom/icon-dir'
import { IconLayoutCompact } from '@/assets/custom/icon-layout-compact'
import { IconLayoutDefault } from '@/assets/custom/icon-layout-default'
import { IconLayoutFull } from '@/assets/custom/icon-layout-full'
import { IconSidebarFloating } from '@/assets/custom/icon-sidebar-floating'
import { IconSidebarInset } from '@/assets/custom/icon-sidebar-inset'
import { IconSidebarSidebar } from '@/assets/custom/icon-sidebar-sidebar'
import { useDirection } from '@/context/direction-provider'
import { type Collapsible, useLayout } from '@/context/layout-provider'
import { cn } from '@/lib/utils'
import {
  DEFAULT_DENSITY,
  DEFAULT_MOTION,
  DEFAULT_RADIUS,
  RADIUS_SCALE,
  useTheme,
  type Density,
  type Motion,
  type Radius,
} from '@/stores/theme-store'
import { useSidebar } from '@/components/ui/sidebar'
import { RadioGroupItem, RadioTile, SectionTitle } from '../shared'

function ShellConfig() {
  const { defaultMode, mode, setMode } = useLayout()
  return (
    <div>
      <SectionTitle
        title='Shell'
        showReset={mode !== defaultMode}
        onReset={() => setMode(defaultMode)}
        resetAriaLabel='Reset shell layout to default'
      />
      <Radio
        value={mode}
        onValueChange={(v) => setMode(v as typeof mode)}
        className='grid w-full max-w-md grid-cols-2 gap-4'
        aria-label='Select shell layout'
      >
        {[
          { value: 'header', label: 'Top header', Icon: PanelTop },
          { value: 'sidebar', label: 'Sidebar', Icon: PanelLeft },
        ].map(({ value, label, Icon }) => (
          <Item
            key={value}
            value={value}
            className='group outline-none'
            aria-label={`Select ${label.toLowerCase()} shell`}
          >
            <div
              className={cn(
                'relative flex h-16 items-center justify-center rounded-[6px] ring-[1px] ring-[color:var(--color-border)]',
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
              <Icon className='h-7 w-7 text-[color:var(--color-muted)] group-data-[state=checked]:text-[color:var(--color-leaf)]' />
            </div>
            <div className='mt-1 text-xs'>{label}</div>
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
          { value: 'inset', label: 'Inset', icon: IconSidebarInset },
          { value: 'floating', label: 'Floating', icon: IconSidebarFloating },
          { value: 'sidebar', label: 'Sidebar', icon: IconSidebarSidebar },
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
          { value: 'default', label: 'Default', icon: IconLayoutDefault },
          { value: 'icon', label: 'Compact', icon: IconLayoutCompact },
          { value: 'offcanvas', label: 'Full layout', icon: IconLayoutFull },
        ].map((item) => (
          <RadioGroupItem key={item.value} item={item} />
        ))}
      </Radio>
    </div>
  )
}

function RadiusConfig() {
  const { radius, setRadius } = useTheme()
  const items: { value: Radius; label: string }[] = [
    { value: 'none', label: 'None' },
    { value: 'sm', label: 'Small' },
    { value: 'md', label: 'Medium' },
    { value: 'lg', label: 'Large' },
  ]
  return (
    <div>
      <SectionTitle
        title='Corner radius'
        showReset={radius !== DEFAULT_RADIUS}
        onReset={() => setRadius(DEFAULT_RADIUS)}
        resetAriaLabel='Reset corner radius to default'
      />
      <Radio
        value={radius}
        onValueChange={(v) => setRadius(v as Radius)}
        className='grid w-full max-w-md grid-cols-4 gap-2'
        aria-label='Select corner radius'
      >
        {items.map((item) => (
          <Item
            key={item.value}
            value={item.value}
            className='group outline-none'
            aria-label={`Select ${item.label.toLowerCase()} radius`}
          >
            <div
              className={cn(
                'relative flex h-14 items-center justify-center ring-[1px] ring-[color:var(--color-border)] bg-[color:var(--color-surface-2)]',
                'group-data-[state=checked]:shadow-2xl group-data-[state=checked]:ring-[color:var(--color-leaf)]',
              )}
              style={{ borderRadius: `${Number(RADIUS_SCALE[item.value]) * 14}px` }}
            >
              <CircleCheck
                className={cn(
                  'size-5 fill-[color:var(--color-leaf)] stroke-white',
                  'group-data-[state=unchecked]:hidden',
                  'absolute top-0 right-0 translate-x-1/2 -translate-y-1/2',
                )}
              />
            </div>
            <div className='mt-1 text-xs'>{item.label}</div>
          </Item>
        ))}
      </Radio>
    </div>
  )
}

function DensityConfig() {
  const { density, setDensity } = useTheme()
  const items: { value: Density; label: string }[] = [
    { value: 'compact', label: 'Compact' },
    { value: 'comfortable', label: 'Comfortable' },
    { value: 'spacious', label: 'Spacious' },
  ]
  return (
    <div>
      <SectionTitle
        title='Density'
        showReset={density !== DEFAULT_DENSITY}
        onReset={() => setDensity(DEFAULT_DENSITY)}
        resetAriaLabel='Reset density to default'
      />
      <Radio
        value={density}
        onValueChange={(v) => setDensity(v as Density)}
        className='grid w-full max-w-md grid-cols-3 gap-2'
        aria-label='Select density'
      >
        {items.map((item) => (
          <RadioTile
            key={item.value}
            value={item.value}
            label={item.label}
            ariaLabel={`Select ${item.label.toLowerCase()} density`}
          >
            <div className='text-sm font-medium text-[color:var(--color-text)]'>{item.label}</div>
          </RadioTile>
        ))}
      </Radio>
    </div>
  )
}

function MotionConfig() {
  const { motion, setMotion } = useTheme()
  const items: { value: Motion; label: string }[] = [
    { value: 'full', label: 'Full motion' },
    { value: 'reduced', label: 'Reduced' },
  ]
  return (
    <div>
      <SectionTitle
        title='Motion'
        showReset={motion !== DEFAULT_MOTION}
        onReset={() => setMotion(DEFAULT_MOTION)}
        resetAriaLabel='Reset motion preference to default'
      />
      <Radio
        value={motion}
        onValueChange={(v) => setMotion(v as Motion)}
        className='grid w-full max-w-md grid-cols-2 gap-2'
        aria-label='Select motion preference'
      >
        {items.map((item) => (
          <RadioTile
            key={item.value}
            value={item.value}
            label={item.label}
            ariaLabel={`Select ${item.label.toLowerCase()}`}
          >
            <div className='text-sm font-medium text-[color:var(--color-text)]'>{item.label}</div>
          </RadioTile>
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

export function LayoutTab() {
  return (
    <div className='space-y-6'>
      <ShellConfig />
      <SidebarConfig />
      <LayoutConfig />
      <RadiusConfig />
      <DensityConfig />
      <MotionConfig />
      <DirConfig />
    </div>
  )
}
