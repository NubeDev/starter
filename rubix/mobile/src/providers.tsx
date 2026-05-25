// providers.tsx — the 8-provider stack from APP-SHELL.md §Provider stack.
//
// Composition only — no logic. If this file grows past ~40 lines the
// providers are doing too much; push the leaf-pulling out into the
// provider modules themselves and re-import.

import { type ReactNode, useMemo } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

import { LocalDbProvider } from './local-db/provider';
import { ConnectionProvider } from './connection/provider';
import { ThemeProvider } from './theme/provider';
import { I18nProvider } from './i18n/provider';
import { ClientStrap } from './connection/client-strap';

export function Providers({ children }: { children: ReactNode }) {
  // Single QueryClient for the app lifetime. Per-connection cache
  // isolation is via key namespacing (`starterQueryKey`), not
  // `queryClient.clear()`, per APP-SHELL.md.
  const queryClient = useMemo(() => new QueryClient(), []);
  return (
    <QueryClientProvider client={queryClient}>
      <LocalDbProvider>
        <ConnectionProvider>
          <I18nProvider>
            <ThemeProvider>
              <ClientStrap>{children}</ClientStrap>
            </ThemeProvider>
          </I18nProvider>
        </ConnectionProvider>
      </LocalDbProvider>
    </QueryClientProvider>
  );
}
