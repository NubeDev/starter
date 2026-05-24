import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider, createRouter } from '@tanstack/react-router'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ReactQueryDevtools } from '@tanstack/react-query-devtools'
import { AuthProvider, StarterClientProvider } from '@nube/starter-client-react'
import { StarterError } from '@nube/starter-client-ts'
import { RubixClientProvider } from '@nube/rubix-client-react'
import { routeTree } from './routeTree.gen'
import { DirectionProvider } from '@nube/starter-ui-core/layout'
import { ThemeProvider } from '@/components/theme/theme-provider'
import { I18nProvider } from '@/i18n/provider'
import { getRubixClient, getStarterClient } from '@/lib/client'
import { LoginRoute } from '@/routes/login'
import '@xyflow/react/dist/style.css'
import '@nube/starter-ui-flow/styles.css'
import './styles/theme.css'

const router = createRouter({ routeTree, defaultPreload: 'intent' })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

// Single ambient QueryClient for the whole app. Defaults mirror what
// `@nube/starter-client-react`'s `QueryProvider` sets — staleTime 30s,
// gcTime 5min, retry skip on auth errors — but we construct it
// directly here so we can hand it to additional providers later
// without re-wrapping.
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30_000,
      gcTime: 5 * 60_000,
      retry: (failureCount, error) => {
        if (
          error instanceof StarterError &&
          (error.status === 401 || error.status === 403)
        ) {
          return false
        }
        return failureCount < 2
      },
    },
  },
})

// SCOPE OQ-4: a single layout-level auth guard sits at the app root
// via `AuthProvider.unauthenticatedSlot` — when `me()` returns 401
// the provider swaps the entire children subtree out for
// `<LoginRoute />` instead of rendering the router. Authed routes
// therefore stay guard-free: there's no per-route boilerplate to
// drift out of sync.
createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <StarterClientProvider client={getStarterClient()}>
        <RubixClientProvider client={getRubixClient()}>
          <DirectionProvider>
            <ThemeProvider>
              <I18nProvider>
                <AuthProvider unauthenticatedSlot={<LoginRoute />}>
                  <RouterProvider router={router} />
                </AuthProvider>
              </I18nProvider>
            </ThemeProvider>
          </DirectionProvider>
        </RubixClientProvider>
      </StarterClientProvider>
      {/* SCOPE OQ-3: devtools mount only in dev — Vite tree-shakes
          the import out of production bundles via the DEV flag. */}
      {import.meta.env.DEV ? <ReactQueryDevtools initialIsOpen={false} /> : null}
    </QueryClientProvider>
  </StrictMode>,
)
