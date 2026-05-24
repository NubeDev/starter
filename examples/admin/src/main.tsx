import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider, createRouter } from '@tanstack/react-router'
import { routeTree } from './routeTree.gen'
import { DirectionProvider } from '@nube/starter-ui-core/layout'
import { ThemeProvider } from '@/components/theme/theme-provider'
import { I18nProvider } from '@/i18n/provider'
import '@xyflow/react/dist/style.css'
import '@nube/starter-ui-flow/styles.css'
import './styles/theme.css'

const router = createRouter({ routeTree, defaultPreload: 'intent' })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <DirectionProvider>
      <ThemeProvider>
        <I18nProvider>
          <RouterProvider router={router} />
        </I18nProvider>
      </ThemeProvider>
    </DirectionProvider>
  </StrictMode>,
)
