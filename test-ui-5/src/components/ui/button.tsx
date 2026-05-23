import * as React from 'react'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from '@/lib/utils'

const buttonVariants = cva(
  'group inline-flex items-center justify-center gap-2 rounded-full text-sm font-semibold transition-all active:scale-[0.98] disabled:pointer-events-none disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--color-ring)] focus-visible:ring-offset-2 focus-visible:ring-offset-[color:var(--color-bg)] cursor-pointer',
  {
    variants: {
      variant: {
        default:
          'bg-white text-zinc-950 hover:scale-[1.02] hover:bg-zinc-100 shadow-[0_10px_30px_-10px_rgba(255,255,255,0.25)]',
        leaf:
          'bg-[color:var(--color-leaf)] text-[color:var(--color-bg)] hover:bg-[color:var(--color-leaf-2)] hover:scale-[1.02] shadow-[0_10px_40px_-10px_rgba(74,222,128,0.55)]',
        aqua:
          'bg-[color:var(--color-aqua)] text-[color:var(--color-bg)] hover:bg-[color:var(--color-aqua-2)] hover:scale-[1.02] shadow-[0_10px_40px_-10px_rgba(103,232,249,0.55)]',
        sun:
          'bg-[color:var(--color-sun)] text-[color:var(--color-bg)] hover:scale-[1.02] shadow-[0_10px_40px_-10px_rgba(253,230,138,0.55)]',
        ghost:
          'border border-white/10 bg-white/[0.04] text-white backdrop-blur-md hover:bg-white/[0.08] hover:border-[color:var(--color-leaf)]/30',
        outline:
          'border border-white/15 bg-transparent text-white hover:bg-white/5',
      },
      size: {
        default: 'h-11 px-6 py-3',
        sm: 'h-9 px-4 text-xs',
        lg: 'h-14 px-8 text-base',
        icon: 'h-10 w-10',
      },
    },
    defaultVariants: { variant: 'default', size: 'default' },
  },
)

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, ...props }, ref) => (
    <button
      ref={ref}
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  ),
)
Button.displayName = 'Button'

export { buttonVariants }
