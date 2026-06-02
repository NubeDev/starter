import { useState } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { Loader2 } from 'lucide-react'
import { PhoneFrame } from './components/PhoneFrame'
import { NavBar } from './components/NavBar'
import { ToastProvider } from './components/Toast'
import { MoodBackdrop } from './theme/MoodBackdrop'
import { ThemeProvider } from './theme/ThemeProvider'
import { useLook } from './theme/useLook'
import { AuthProvider } from './auth/AuthProvider'
import { useAuth } from './auth/authContext'
import { Connect } from './auth/Connect'
import { TopBar } from './shell/TopBar'
import { PAGES, DEFAULT_TAB, pageByTab, type Tab } from './pages/registry'

export default function App() {
  return (
    <ThemeProvider>
      <AuthProvider>
        <PhoneFrame>
          <ToastProvider>
            <Shell />
          </ToastProvider>
        </PhoneFrame>
      </AuthProvider>
    </ThemeProvider>
  )
}

// Inside the providers so it reads the resolved look + session. Gates the whole
// app behind Connect until authenticated.
function Shell() {
  const { user, ready } = useAuth()
  const look = useLook()
  const [tab, setTab] = useState<Tab>(DEFAULT_TAB)
  // a page id handed to the Preview tab when the scan flow deep-links into it
  const [previewPageId, setPreviewPageId] = useState<string | undefined>(undefined)

  const goPreview = (pageId: string) => {
    setPreviewPageId(pageId)
    setTab('preview')
  }

  const actions = {
    onNavigate: (t: Tab) => setTab(t),
    onPreview: goPreview,
    previewPageId,
  }

  const active = pageByTab(tab) ?? pageByTab(DEFAULT_TAB) ?? PAGES[0]

  return (
    <div className="relative flex h-full flex-col overflow-hidden">
      <MoodBackdrop />

      {!ready ? (
        <div className="grid h-full place-items-center text-ink-muted">
          <Loader2 className="h-7 w-7 animate-spin" />
        </div>
      ) : !user ? (
        <Connect />
      ) : (
        <>
          <TopBar />
          {/* Fixed-height region between TopBar and the bottom of the frame.
              Each page owns its own internal scroll + dock spacing (e.g.
              ScanFlow's `h-full overflow-y-auto pb-32`), so this stays a plain
              non-scrolling flex child — its `h-full` is what lets the pages
              size to the viewport instead of to their content. */}
          <div className="relative min-h-0 flex-1 overflow-hidden">
            <AnimatePresence mode="wait">
              <motion.div
                key={active.tab}
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -8 }}
                transition={{ duration: 0.22 }}
                className="h-full"
              >
                {active.element(actions)}
              </motion.div>
            </AnimatePresence>
          </div>

          <NavBar active={tab} onChange={setTab} onFab={() => setTab('scan')} accent={look.accent} />
        </>
      )}
    </div>
  )
}
