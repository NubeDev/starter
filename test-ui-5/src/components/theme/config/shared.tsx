import { type SVGProps } from 'react'
import { Item } from '@radix-ui/react-radio-group'
import { CircleCheck, RotateCcw } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'

export function SectionTitle({
  title,
  description,
  showReset = false,
  onReset,
  resetAriaLabel,
  className,
}: {
  title: string
  description?: string
  showReset?: boolean
  onReset?: () => void
  resetAriaLabel?: string
  className?: string
}) {
  return (
    <div className={cn('mb-2', className)}>
      <div className='flex items-center gap-2 text-sm font-semibold text-[color:var(--color-muted)]'>
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
      {description && (
        <p className='mt-0.5 text-xs text-[color:var(--color-subtle)]'>{description}</p>
      )}
    </div>
  )
}

export function RadioGroupItem({
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

export function RadioTile({
  value,
  label,
  ariaLabel,
  className,
  style,
  children,
}: {
  value: string
  label: string
  ariaLabel?: string
  className?: string
  style?: React.CSSProperties
  children?: React.ReactNode
}) {
  return (
    <Item
      value={value}
      className='group outline-none'
      aria-label={ariaLabel ?? `Select ${label.toLowerCase()}`}
    >
      <div
        className={cn(
          'relative rounded-[6px] ring-[1px] ring-[color:var(--color-border)] px-3 py-2 text-center',
          'group-data-[state=checked]:shadow-2xl group-data-[state=checked]:ring-[color:var(--color-leaf)]',
          className,
        )}
        style={style}
      >
        <CircleCheck
          className={cn(
            'size-5 fill-[color:var(--color-leaf)] stroke-white',
            'group-data-[state=unchecked]:hidden',
            'absolute top-0 right-0 translate-x-1/2 -translate-y-1/2',
          )}
        />
        {children}
      </div>
      <div className='mt-1 text-xs'>{label}</div>
    </Item>
  )
}

export function ComingSoonField({
  label,
  description,
  control,
}: {
  label: string
  description?: string
  control: React.ReactNode
}) {
  return (
    <div className='space-y-1.5'>
      <div className='flex items-center justify-between gap-2'>
        <label className='text-xs font-medium text-[color:var(--color-text)]'>{label}</label>
        <span className='rounded-full bg-[color:var(--color-surface-2)] px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider text-[color:var(--color-subtle)]'>
          Coming soon
        </span>
      </div>
      {control}
      {description && (
        <p className='text-[11px] text-[color:var(--color-subtle)]'>{description}</p>
      )}
    </div>
  )
}
