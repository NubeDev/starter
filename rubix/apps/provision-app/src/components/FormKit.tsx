import type { ReactNode } from 'react'
import { motion } from 'framer-motion'

// Glass-styled form primitives shared across Place / Connect / Devices etc.
// Inline-typed props, lucide-free (pure inputs). One concept per export.

export function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="flex flex-col gap-1.5">
      <span className="label">{label}</span>
      {children}
    </label>
  )
}

// Native <select> styled as glass — accessible, keyboard-friendly.
export function Picker({
  value,
  options,
  placeholder,
  onChange,
}: {
  value: string
  options: ReadonlyArray<{ value: string; label: string }>
  placeholder: string
  onChange: (v: string) => void
}) {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="glass w-full cursor-pointer rounded-xl px-4 py-3.5 text-base text-ink outline-none focus:ring-2 focus:ring-primary/60"
    >
      <option value="" className="bg-surface-low text-ink">
        {placeholder}
      </option>
      {options.map((o) => (
        <option key={o.value} value={o.value} className="bg-surface-low text-ink">
          {o.label}
        </option>
      ))}
    </select>
  )
}

export function TextInput({
  value,
  onChange,
  placeholder,
  type = 'text',
  onEnter,
  ariaLabel,
}: {
  value: string
  onChange: (v: string) => void
  placeholder?: string
  type?: 'text' | 'email' | 'password' | 'url'
  onEnter?: () => void
  ariaLabel?: string
}) {
  return (
    <input
      value={value}
      type={type}
      aria-label={ariaLabel}
      onChange={(e) => onChange(e.target.value)}
      onKeyDown={(e) => e.key === 'Enter' && onEnter?.()}
      placeholder={placeholder}
      className="glass w-full rounded-xl px-4 py-3.5 text-base text-ink placeholder:text-ink-muted outline-none focus:ring-2 focus:ring-primary/60"
    />
  )
}

// Pill toggle with a spring-sliding knob. Used for trend/alarm + per-point.
export function Toggle({
  on,
  onToggle,
  label,
  accent = '#36e2c4',
}: {
  on: boolean
  onToggle: () => void
  label?: string
  accent?: string
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      aria-label={label}
      onClick={onToggle}
      className="flex w-11 cursor-pointer items-center rounded-full p-0.5 transition-colors duration-200"
      style={{ backgroundColor: on ? accent : 'rgba(255,255,255,0.14)' }}
    >
      <motion.span
        layout
        transition={{ type: 'spring', stiffness: 500, damping: 32 }}
        className="block h-5 w-5 rounded-full bg-white shadow"
        style={{ marginLeft: on ? 'auto' : 0 }}
      />
    </button>
  )
}
