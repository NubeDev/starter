import * as React from 'react'
import { cn } from '@/lib/utils'

export interface BadgeProps extends React.HTMLAttributes<HTMLDivElement> {
  variant?: 'default' | 'leaf' | 'aqua' | 'sun' | 'live' | 'outline'
}

export const Badge = React.forwardRef<HTMLDivElement, BadgeProps>(
  ({ className, variant = 'default', children, ...props }, ref) => {
    const styles = {
      default:
        'border-white/10 bg-white/5 text-[color:var(--color-muted)]',
      leaf:
        'border-[color:var(--color-leaf)]/30 bg-[color:var(--color-leaf)]/10 text-[color:var(--color-leaf)]',
      aqua:
        'border-[color:var(--color-aqua)]/30 bg-[color:var(--color-aqua)]/10 text-[color:var(--color-aqua)]',
      sun:
        'border-[color:var(--color-sun)]/30 bg-[color:var(--color-sun)]/10 text-[color:var(--color-sun)]',
      live:
        'border-[color:var(--color-leaf)]/30 bg-[color:var(--color-leaf)]/10 text-[color:var(--color-leaf)]',
      outline: 'border-white/15 bg-transparent text-zinc-300',
    }[variant]
    return (
      <div
        ref={ref}
        className={cn(
          'inline-flex items-center gap-1.5 rounded-full border px-3 py-1 text-[10px] font-semibold uppercase tracking-[0.12em] backdrop-blur-md',
          styles,
          className,
        )}
        {...props}
      >
        {variant === 'live' && (
          <span className="relative flex h-1.5 w-1.5">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-[color:var(--color-leaf)] opacity-75" />
            <span className="relative inline-flex h-1.5 w-1.5 rounded-full bg-[color:var(--color-leaf)]" />
          </span>
        )}
        {children}
      </div>
    )
  },
)
Badge.displayName = 'Badge'
