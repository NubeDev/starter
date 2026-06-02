import type { ReactNode } from 'react'

// Wraps the app in a phone-shaped viewport so it reads as a native app on
// desktop/browser. On a real phone (or Tauri mobile) it just fills the screen.
// Copied verbatim from the design system; only the title comment changed.
export function PhoneFrame({ children }: { children: ReactNode }) {
  return (
    // Mobile: a FIXED-height viewport box (100dvh), never content-sized — so a
    // bottom-anchored child (the NavBar) pins to the visible bottom and tall
    // screens scroll *inside* instead of pushing the dock under the system bar.
    // Desktop (sm:): the cosmetic 390x844 phone shell, centered.
    <div className="h-[100dvh] w-full flex items-center justify-center overflow-hidden sm:py-8">
      <div
        className="relative h-full w-full sm:max-h-full sm:h-[844px] sm:max-w-[390px] sm:rounded-[44px] sm:border-[10px] sm:border-black/80 sm:shadow-[0_40px_120px_-20px_rgba(0,0,0,0.8)] overflow-hidden bg-obsidian"
      >
        {/* status-bar notch (cosmetic) */}
        <div className="hidden sm:flex absolute top-0 inset-x-0 h-9 z-50 items-center justify-center pointer-events-none">
          <div className="w-28 h-6 bg-black rounded-full" />
        </div>
        {/* Fills the fixed frame exactly; the App shell inside owns scrolling. */}
        <div className="h-full overflow-hidden">{children}</div>
      </div>
    </div>
  )
}
