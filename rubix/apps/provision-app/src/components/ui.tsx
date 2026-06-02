import type { CSSProperties, ReactNode } from 'react'
import { motion } from 'framer-motion'

// Shared glass primitives — adapted from the design system's ui.tsx.
// Couples-specific icon maps were dropped; everything below is generic.

// A pressable glass card with a spring-y tap. Used everywhere.
export function GlassCard({
  children,
  onClick,
  className = '',
  glow,
  style,
}: {
  children: ReactNode
  onClick?: () => void
  className?: string
  glow?: 'primary' | 'coral'
  style?: CSSProperties
}) {
  // `style` (e.g. a theme-driven boxShadow) overrides the default glow class.
  const glowClass = style?.boxShadow
    ? ''
    : glow === 'primary'
      ? 'shadow-glow-primary'
      : glow === 'coral'
        ? 'shadow-glow-coral'
        : 'shadow-glass'
  return (
    <motion.div
      whileTap={onClick ? { scale: 0.97 } : undefined}
      transition={{ type: 'spring', stiffness: 400, damping: 28 }}
      onClick={onClick}
      role={onClick ? 'button' : undefined}
      tabIndex={onClick ? 0 : undefined}
      style={style}
      className={`glass rounded-xl ${glowClass} ${onClick ? 'cursor-pointer' : ''} ${className}`}
    >
      {children}
    </motion.div>
  )
}

export function Chip({
  label,
  active,
  onClick,
}: {
  label: string
  active?: boolean
  onClick?: () => void
}) {
  return (
    <motion.button
      whileTap={{ scale: 0.94 }}
      onClick={onClick}
      className={`cursor-pointer rounded-full px-3.5 py-2 text-sm font-medium transition-colors duration-200 ${
        active
          ? 'bg-primary text-primary-on'
          : 'bg-white/[0.06] text-ink-variant hover:bg-white/[0.1]'
      }`}
    >
      {label}
    </motion.button>
  )
}

export function PrimaryButton({
  children,
  onClick,
  tone = 'primary',
  accent,
  disabled,
  type = 'button',
}: {
  children: ReactNode
  onClick?: () => void
  tone?: 'primary' | 'coral'
  // when set (from the resolved look), overrides the static tone
  accent?: string
  disabled?: boolean
  type?: 'button' | 'submit'
}) {
  const bg = accent
    ? ''
    : tone === 'coral'
      ? 'bg-coral text-coral-on shadow-glow-coral'
      : 'bg-primary text-primary-on shadow-glow-primary'
  return (
    <motion.button
      whileTap={disabled ? undefined : { scale: 0.97 }}
      transition={{ type: 'spring', stiffness: 400, damping: 28 }}
      onClick={onClick}
      disabled={disabled}
      type={type}
      style={accent ? { backgroundColor: accent, color: '#002019', boxShadow: `0 8px 32px -8px ${accent}` } : undefined}
      className={`w-full cursor-pointer rounded-2xl py-4 text-base font-bold transition-opacity ${bg} ${
        disabled ? 'opacity-40' : ''
      }`}
    >
      {children}
    </motion.button>
  )
}

export function SectionLabel({ children }: { children: ReactNode }) {
  return <p className="label mb-3">{children}</p>
}

// Page header: uppercase eyebrow + headline. The recurring page-top recipe.
export function PageHeader({ eyebrow, title }: { eyebrow: string; title: string }) {
  return (
    <header className="mb-6">
      <p className="label">{eyebrow}</p>
      <h1 className="text-headline-mobile text-ink">{title}</h1>
    </header>
  )
}
