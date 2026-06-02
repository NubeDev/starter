import { createContext, useContext } from 'react'

// Toast context + hook, separated from the provider component (Fast Refresh).
export interface ToastApi {
  show: (text: string, accent?: string) => void
}

export const ToastContext = createContext<ToastApi | null>(null)

export function useToast() {
  const ctx = useContext(ToastContext)
  if (!ctx) throw new Error('useToast must be used within ToastProvider')
  return ctx
}
