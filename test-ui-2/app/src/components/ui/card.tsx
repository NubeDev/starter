import * as React from 'react'
import { cn } from '@/lib/utils'

export function Card({
  className,
  hairline = false,
  ...props
}: React.HTMLAttributes<HTMLDivElement> & { hairline?: boolean }) {
  return (
    <div
      className={cn(
        'relative overflow-hidden rounded-2xl glass',
        hairline && 'hairline-top',
        className,
      )}
      {...props}
    />
  )
}

export function CardHeader({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn('flex items-start justify-between gap-2 px-4 pt-4 pb-2', className)}
      {...props}
    />
  )
}

export function CardTitle({ className, ...props }: React.HTMLAttributes<HTMLHeadingElement>) {
  return (
    <h3
      className={cn(
        'font-mono text-[11px] font-medium uppercase tracking-[0.14em] text-[var(--color-muted)]',
        className,
      )}
      {...props}
    />
  )
}

export function CardContent({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn('px-4 pb-4', className)} {...props} />
}
