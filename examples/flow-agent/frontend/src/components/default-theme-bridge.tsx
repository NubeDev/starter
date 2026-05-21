// Stamps theme tokens onto `<html>` so the WHOLE app (sidebar,
// header, pages — not just the editor's scoped preview pane) honours
// the resolved theme.
//
// Priority on every render:
//   1. tokens in the editor's Zustand store, if non-empty (covers
//      live-edits and post-save state once the user has opened the
//      editor), OR
//   2. tokens saved to the same `localStorageThemeTransport` key the
//      Settings page uses, on first mount, OR
//   3. our flow-agent brand defaults.

import { useEffect, useState } from "react"
import { useTheme } from "@kit/theme"
import {
  applyThemeToElement,
  useThemeEditorStore,
  type ThemeStyleProps,
  type ThemeStyles,
} from "@nube/starter-ui-core/theme-editor"

import { flowAgentDarkTheme, flowAgentLightTheme } from "@/lib/default-theme"

const STORAGE_KEY = "fa-theme"

function isNonEmpty(p: ThemeStyleProps | undefined): p is ThemeStyleProps {
  return !!p && Object.keys(p).length > 0
}

function readSavedDocument(): { theme_styles: ThemeStyles } | null {
  if (typeof window === "undefined") return null
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    if (!raw) return null
    return JSON.parse(raw)
  } catch {
    return null
  }
}

export function DefaultThemeBridge() {
  const { resolved } = useTheme()
  const storeStyles = useThemeEditorStore((s) => s.styles)
  const [saved] = useState(readSavedDocument)

  useEffect(() => {
    const fallback =
      resolved === "dark" ? flowAgentDarkTheme : flowAgentLightTheme

    const fromStore = storeStyles?.[resolved]
    if (isNonEmpty(fromStore)) {
      applyThemeToElement(
        document.documentElement,
        { ...fallback, ...fromStore },
        resolved,
      )
      return
    }

    const fromSaved = saved?.theme_styles?.[resolved]
    if (isNonEmpty(fromSaved)) {
      applyThemeToElement(
        document.documentElement,
        { ...fallback, ...fromSaved },
        resolved,
      )
      return
    }

    applyThemeToElement(document.documentElement, fallback, resolved)
  }, [resolved, storeStyles, saved])

  return null
}
