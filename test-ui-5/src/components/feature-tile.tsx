import { motion } from 'motion/react'
import { ArrowUpRight, type LucideIcon } from 'lucide-react'
import { cn } from '@/lib/utils'

interface FeatureTileProps {
  icon: LucideIcon
  eyebrow: string
  title: string
  body: string
  accent?: 'leaf' | 'aqua' | 'sun' | 'white'
  className?: string
}

export function FeatureTile({
  icon: Icon,
  eyebrow,
  title,
  body,
  accent = 'white',
  className,
}: FeatureTileProps) {
  const ring =
    accent === 'leaf'
      ? 'ring-[color:var(--color-leaf)]/30 text-[color:var(--color-leaf)] bg-[color:var(--color-leaf)]/10'
      : accent === 'aqua'
      ? 'ring-[color:var(--color-aqua)]/30 text-[color:var(--color-aqua)] bg-[color:var(--color-aqua)]/10'
      : accent === 'sun'
      ? 'ring-[color:var(--color-sun)]/30 text-[color:var(--color-sun)] bg-[color:var(--color-sun)]/10'
      : 'ring-white/15 text-white bg-white/5'
  return (
    <motion.a
      href="#"
      initial={{ opacity: 0, y: 20 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: '-80px' }}
      transition={{ duration: 0.6, ease: [0.22, 1, 0.36, 1] }}
      whileHover={{ y: -3 }}
      className={cn(
        'glass group relative block overflow-hidden rounded-3xl p-6',
        className,
      )}
    >
      <div className="flex items-start justify-between">
        <div className={cn('flex h-11 w-11 items-center justify-center rounded-2xl ring-1', ring)}>
          <Icon className="h-5 w-5" />
        </div>
        <ArrowUpRight className="h-5 w-5 text-zinc-500 transition-all group-hover:-translate-y-0.5 group-hover:translate-x-0.5 group-hover:text-white" />
      </div>
      <div className="mt-8 text-[10px] font-medium uppercase tracking-[0.2em] text-zinc-500">
        {eyebrow}
      </div>
      <h3 className="mt-2 text-2xl font-medium leading-tight tracking-[-0.02em] text-white">
        {title}
      </h3>
      <p className="mt-3 text-sm leading-relaxed text-[color:var(--color-muted)]">{body}</p>
    </motion.a>
  )
}
