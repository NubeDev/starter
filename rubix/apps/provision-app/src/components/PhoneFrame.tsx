import type { ReactNode } from 'react'

// Wraps the app in a phone-shaped viewport so it reads as a native app on
// desktop/browser. On a real phone (or Tauri mobile) it just fills the screen.
// Copied verbatim from the design system; only the title comment changed.
export function PhoneFrame({ children }: { children: ReactNode }) {
  return (
    <div className="min-h-full w-full flex items-center justify-center sm:py-8">
      <div
        className="relative w-full sm:max-w-[390px] sm:h-[844px] sm:rounded-[44px] sm:border-[10px] sm:border-black/80 sm:shadow-[0_40px_120px_-20px_rgba(0,0,0,0.8)] overflow-hidden bg-obsidian"
        style={{ aspectRatio: 'auto' }}
      >
        {/* status-bar notch (cosmetic) */}
        <div className="hidden sm:flex absolute top-0 inset-x-0 h-9 z-50 items-center justify-center pointer-events-none">
          <div className="w-28 h-6 bg-black rounded-full" />
        </div>
        <div className="h-full min-h-[100svh] sm:min-h-0 overflow-hidden">{children}</div>
      </div>
    </div>
  )
}
