import { AnimatePresence, motion, useScroll, useSpring } from 'motion/react'
import { Outlet, createRootRoute, useRouterState } from '@tanstack/react-router'
import { BootIntro } from '@/components/boot-intro'
import { TopHeader } from '@/components/top-header'
import { ActionDock } from '@/components/action-dock'
import { AppSidebar } from '@/components/layout/app-sidebar'
import { Header } from '@/components/layout/header'
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar'
import { useIsMobile, LayoutProvider, useLayout } from '@nube/starter-ui-core/layout'
import { getCookie } from '@/lib/cookies'
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

function HeaderShellInner() {
  const { location } = useRouterState()
  const isMobile = useIsMobile()
  return (
    <div className="min-h-screen bg-[color:var(--color-bg)] text-[color:var(--color-text)]">
      <BootIntro />
      <ScrollProgress />
      {/* Mobile-only slide-in nav so the SidebarTrigger in the top bar can open
          AppSidebar on small screens. On desktop the inline HeaderNav is the nav. */}
      {isMobile && <AppSidebar />}
      <div className="relative">
        <TopHeader />
        <AnimatePresence mode="wait">
          <motion.main
            key={location.pathname}
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -4 }}
            transition={{ duration: 0.45, ease: [0.22, 1, 0.36, 1] }}
            className={cn('pt-24')}
          >
            <Outlet />
          </motion.main>
        </AnimatePresence>
      </div>
    </div>
  )
}

function SidebarShellInner() {
  const { location } = useRouterState()
  return (
    <>
      <BootIntro />
      <ScrollProgress />
      <AppSidebar />
      <SidebarInset className="@container/content min-h-svh bg-transparent text-[color:var(--color-text)]">
        <Header>
          <div className="ml-auto">
            <ActionDock inline />
          </div>
        </Header>
        <AnimatePresence mode="wait">
          <motion.main
            key={location.pathname}
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -4 }}
            transition={{ duration: 0.45, ease: [0.22, 1, 0.36, 1] }}
            className="pt-2"
          >
            <Outlet />
          </motion.main>
        </AnimatePresence>
      </SidebarInset>
    </>
  )
}

function Shell() {
  const { mode } = useLayout()
  const defaultOpen = getCookie('sidebar_state') !== 'false'
  return (
    <SidebarProvider
      defaultOpen={defaultOpen}
      className={mode === 'header' ? 'flex-col' : undefined}
    >
      {mode === 'sidebar' ? <SidebarShellInner /> : <HeaderShellInner />}
    </SidebarProvider>
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
