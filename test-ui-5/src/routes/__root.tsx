import { AnimatePresence, motion, useScroll, useSpring } from 'motion/react'
import { Outlet, createRootRoute, useRouterState } from '@tanstack/react-router'
import { useState } from 'react'
import { BootIntro } from '@/components/boot-intro'
import { FloatingSidebar } from '@/components/floating-sidebar'
import { TopHeader } from '@/components/top-header'
import { LayoutProvider, useLayout } from '@/context/layout-provider'
import { cn } from '@/lib/utils'

function ScrollProgress() {
  const { scrollYProgress } = useScroll()
  const scaleX = useSpring(scrollYProgress, { stiffness: 100, damping: 30 })
  return (
    <motion.div
      style={{ scaleX, transformOrigin: '0% 50%' }}
      className="fixed inset-x-0 top-0 z-[60] h-[2px] bg-gradient-to-r from-[color:var(--color-leaf)] via-[color:var(--color-aqua)] to-[color:var(--color-sun)]"
    />
  )
}

function Shell() {
  const { mode } = useLayout()
  const { location } = useRouterState()
  const [collapsed, setCollapsed] = useState(false)

  return (
    <div className="min-h-screen bg-[color:var(--color-bg)] text-white">
      <BootIntro />
      <ScrollProgress />

      <AnimatePresence mode="wait">
        {mode === 'sidebar' && (
          <FloatingSidebar
            key="sidebar"
            collapsed={collapsed}
            onToggleCollapse={() => setCollapsed((c) => !c)}
          />
        )}
      </AnimatePresence>

      <div
        className={cn(
          'relative transition-[padding] duration-700 ease-[cubic-bezier(0.22,1,0.36,1)]',
          mode === 'sidebar' ? (collapsed ? 'lg:pl-[100px]' : 'lg:pl-[288px]') : 'lg:pl-0',
        )}
      >
        <TopHeader />
        <AnimatePresence mode="wait">
          <motion.main
            key={location.pathname}
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -4 }}
            transition={{ duration: 0.45, ease: [0.22, 1, 0.36, 1] }}
            className={cn(mode === 'header' ? 'pt-24' : 'pt-6')}
          >
            <Outlet />
          </motion.main>
        </AnimatePresence>
      </div>
    </div>
  )
}

function RootComponent() {
  return (
    <LayoutProvider>
      <Shell />
    </LayoutProvider>
  )
}

export const Route = createRootRoute({ component: RootComponent })
