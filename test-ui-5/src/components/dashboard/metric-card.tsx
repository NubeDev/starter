import { motion, useMotionValue, useSpring, useTransform } from 'motion/react'
import { useEffect } from 'react'
import { cn } from '@/lib/utils'

interface MetricCardProps {
  label: string
  value: number
  suffix?: string
  prefix?: string
  delta?: number
  spark?: number[]
  accent?: 'leaf' | 'aqua' | 'sun' | 'white'
  className?: string
}

function useAnimatedNumber(target: number) {
  const mv = useMotionValue(0)
  const spring = useSpring(mv, { stiffness: 80, damping: 20 })
  const rounded = useTransform(spring, (v) => Math.round(v).toLocaleString())
  useEffect(() => {
    mv.set(target)
  }, [target, mv])
  return rounded
}

function Spark({ data, color }: { data: number[]; color: string }) {
  if (!data.length) return null
  const max = Math.max(...data)
  const min = Math.min(...data)
  const range = max - min || 1
  const w = 120
  const h = 36
  const points = data
    .map((v, i) => {
      const x = (i / (data.length - 1)) * w
      const y = h - ((v - min) / range) * h
      return `${x},${y}`
    })
    .join(' ')
  return (
    <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`} className="overflow-visible">
      <defs>
        <linearGradient id={`g-${color}`} x1="0" x2="0" y1="0" y2="1">
          <stop offset="0%" stopColor={color} stopOpacity="0.4" />
          <stop offset="100%" stopColor={color} stopOpacity="0" />
        </linearGradient>
      </defs>
      <motion.polyline
        initial={{ pathLength: 0, opacity: 0 }}
        animate={{ pathLength: 1, opacity: 1 }}
        transition={{ duration: 1.6, ease: [0.22, 1, 0.36, 1] }}
        fill="none"
        stroke={color}
        strokeWidth={1.75}
        strokeLinecap="round"
        strokeLinejoin="round"
        points={points}
      />
      <polygon points={`0,${h} ${points} ${w},${h}`} fill={`url(#g-${color})`} />
    </svg>
  )
}

export function MetricCard({
  label,
  value,
  suffix,
  prefix,
  delta,
  spark = [],
  accent = 'white',
  className,
}: MetricCardProps) {
  const animated = useAnimatedNumber(value)
  const accentColor =
    accent === 'leaf'
      ? '#4ade80'
      : accent === 'aqua'
      ? '#67e8f9'
      : accent === 'sun'
      ? '#fde68a'
      : '#ffffff'
  const deltaPositive = (delta ?? 0) >= 0

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: '-50px' }}
      transition={{ duration: 0.6, ease: [0.22, 1, 0.36, 1] }}
      whileHover={{ y: -2 }}
      className={cn(
        'glass group relative flex flex-col gap-4 overflow-hidden rounded-3xl p-6',
        className,
      )}
    >
      <div className="flex items-start justify-between">
        <div className="text-[11px] font-medium uppercase tracking-[0.18em] text-zinc-500">
          {label}
        </div>
        {typeof delta === 'number' && (
          <div
            className={cn(
              'tabular rounded-full px-2 py-0.5 text-[10px] font-semibold',
              deltaPositive
                ? 'bg-emerald-400/10 text-emerald-300'
                : 'bg-rose-400/10 text-rose-300',
            )}
          >
            {deltaPositive ? '↑' : '↓'} {Math.abs(delta).toFixed(1)}%
          </div>
        )}
      </div>
      <div className="flex items-end justify-between gap-3">
        <div className="tabular flex items-baseline gap-1 text-4xl font-semibold tracking-[-0.03em] text-white">
          {prefix && <span className="text-2xl text-zinc-500">{prefix}</span>}
          <motion.span>{animated}</motion.span>
          {suffix && <span className="text-xl text-zinc-500">{suffix}</span>}
        </div>
        <div className="opacity-90">
          <Spark data={spark} color={accentColor} />
        </div>
      </div>
    </motion.div>
  )
}
