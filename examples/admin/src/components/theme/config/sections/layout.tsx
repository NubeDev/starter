// Layout tab. Density + Motion use the packaged TileChoiceSection;
// the rest (Shell, Sidebar, Layout, Radius, Direction) stay local
// because they depend on ui-5-specific stores (layout-provider,
// direction-provider) or on custom icon assets.

import { type SVGProps } from 'react'
import { Root as Radio, Item } from '@radix-ui/react-radio-group'
import { CircleCheck, PanelLeft, PanelTop } from 'lucide-react'
import { useIntl } from 'react-intl'
import {
  RadioIconTile,
  SectionTitle,
  TileChoiceSection,
  type TileChoiceItem,
} from '@nube/starter-ui-kit/theme-editor/config-drawer'
import { IconDir } from '@/assets/custom/icon-dir'
import { IconLayoutCompact } from '@/assets/custom/icon-layout-compact'
import { IconLayoutDefault } from '@/assets/custom/icon-layout-default'
import { IconLayoutFull } from '@/assets/custom/icon-layout-full'
import { IconSidebarFloating } from '@/assets/custom/icon-sidebar-floating'
import { IconSidebarInset } from '@/assets/custom/icon-sidebar-inset'
import { IconSidebarSidebar } from '@/assets/custom/icon-sidebar-sidebar'
import { useDirection } from '@nube/starter-ui-core/layout'
import { type Collapsible, useLayout } from '@nube/starter-ui-core/layout'
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

function ShellConfig() {
  const { defaultMode, mode, setMode } = useLayout()
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  return (
    <div>
      <SectionTitle
        title={tr('settings.row.layout')}
        showReset={mode !== defaultMode}
        onReset={() => setMode(defaultMode)}
        resetAriaLabel={tr('settings.row.layout.hint')}
      />
      <Radio
        value={mode}
        onValueChange={(v) => setMode(v as typeof mode)}
        className='grid w-full max-w-md grid-cols-2 gap-4'
        aria-label={tr('settings.row.layout.hint')}
      >
        {[
          { value: 'header',  label: tr('layoutToggle.header'),  Icon: PanelTop },
          { value: 'sidebar', label: tr('layoutToggle.sidebar'), Icon: PanelLeft },
        ].map(({ value, label, Icon }) => (
          <Item
            key={value}
            value={value}
            className='group outline-none'
            aria-label={label}
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
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  return (
    <div className='max-md:hidden'>
      <SectionTitle
        title={tr('settings.row.sidebarVariant')}
        showReset={defaultVariant !== variant}
        onReset={() => setVariant(defaultVariant)}
        resetAriaLabel={tr('settings.row.sidebarVariant.hint')}
      />
      <Radio
        value={variant}
        onValueChange={(v) => setVariant(v as typeof variant)}
        className='grid w-full max-w-md grid-cols-3 gap-4'
        aria-label={tr('settings.row.sidebarVariant.hint')}
      >
        {[
          { value: 'inset',    label: tr('settings.variant.inset'),    icon: IconSidebarInset },
          { value: 'floating', label: tr('settings.variant.floating'), icon: IconSidebarFloating },
          { value: 'sidebar',  label: tr('settings.variant.sidebar'),  icon: IconSidebarSidebar },
        ].map((item) => (
          <RadioIconTile key={item.value} item={item} />
        ))}
      </Radio>
    </div>
  )
}

function LayoutConfig() {
  const { open, setOpen } = useSidebar()
  const { defaultCollapsible, collapsible, setCollapsible } = useLayout()
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  const radioState = open ? 'default' : collapsible
  return (
    <div className='max-md:hidden'>
      <SectionTitle
        title={tr('settings.row.sidebarCollapse')}
        showReset={radioState !== 'default'}
        onReset={() => {
          setOpen(true)
          setCollapsible(defaultCollapsible)
        }}
        resetAriaLabel={tr('settings.row.sidebarCollapse.hint')}
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
        aria-label={tr('settings.row.sidebarCollapse.hint')}
      >
        {[
          { value: 'default',   label: tr('common.default'),                icon: IconLayoutDefault },
          { value: 'icon',      label: tr('settings.collapsible.icon'),     icon: IconLayoutCompact },
          { value: 'offcanvas', label: tr('settings.collapsible.offcanvas'),icon: IconLayoutFull },
        ].map((item) => (
          <RadioIconTile key={item.value} item={item} />
        ))}
      </Radio>
    </div>
  )
}

function RadiusConfig() {
  const { radius, setRadius } = useTheme()
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  const items: { value: Radius; label: string }[] = [
    { value: 'none', label: tr('settings.collapsible.none') },
    { value: 'sm',   label: tr('config.fontSize.sm') },
    { value: 'md',   label: tr('config.fontSize.md') },
    { value: 'lg',   label: tr('config.fontSize.lg') },
  ]
  return (
    <div>
      <SectionTitle
        title={tr('settings.row.layout')}
        showReset={radius !== DEFAULT_RADIUS}
        onReset={() => setRadius(DEFAULT_RADIUS)}
        resetAriaLabel={tr('settings.row.layout.hint')}
      />
      <Radio
        value={radius}
        onValueChange={(v) => setRadius(v as Radius)}
        className='grid w-full max-w-md grid-cols-4 gap-2'
        aria-label={tr('settings.row.layout.hint')}
      >
        {items.map((item) => (
          <Item
            key={item.value}
            value={item.value}
            className='group outline-none'
            aria-label={item.label}
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
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  const items: TileChoiceItem<Density>[] = [
    { value: 'compact',     label: tr('config.density.compact') },
    { value: 'comfortable', label: tr('config.density.comfortable') },
    { value: 'spacious',    label: tr('config.density.spacious') },
  ]
  return (
    <TileChoiceSection
      value={density}
      onChange={setDensity}
      items={items}
      defaultValue={DEFAULT_DENSITY}
      i18n={{
        title: tr('config.density.title'),
        groupAriaLabel: tr('config.density.groupAria'),
        resetAriaLabel: tr('config.density.resetAria'),
      }}
    />
  )
}

function MotionConfig() {
  const { motion, setMotion } = useTheme()
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  const items: TileChoiceItem<Motion>[] = [
    { value: 'full',    label: tr('config.motion.full') },
    { value: 'reduced', label: tr('config.motion.reduced') },
  ]
  return (
    <TileChoiceSection
      value={motion}
      onChange={setMotion}
      items={items}
      defaultValue={DEFAULT_MOTION}
      i18n={{
        title: tr('config.motion.title'),
        groupAriaLabel: tr('config.motion.groupAria'),
        resetAriaLabel: tr('config.motion.resetAria'),
      }}
    />
  )
}

function DirConfig() {
  const { defaultDir, dir, setDir } = useDirection()
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  return (
    <div>
      <SectionTitle
        title={tr('settings.row.layout')}
        showReset={defaultDir !== dir}
        onReset={() => setDir(defaultDir)}
        resetAriaLabel={tr('settings.row.layout.hint')}
      />
      <Radio
        value={dir}
        onValueChange={(v) => setDir(v as typeof dir)}
        className='grid w-full max-w-md grid-cols-3 gap-4'
        aria-label={tr('settings.row.layout.hint')}
      >
        {[
          {
            value: 'ltr',
            label: 'LTR',
            icon: (props: SVGProps<SVGSVGElement>) => <IconDir dir='ltr' {...props} />,
          },
          {
            value: 'rtl',
            label: 'RTL',
            icon: (props: SVGProps<SVGSVGElement>) => <IconDir dir='rtl' {...props} />,
          },
        ].map((item) => (
          <RadioIconTile key={item.value} item={item} />
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
