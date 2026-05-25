// Slim Vite host for `@nube/starter-ui-ch-explorer`.
//
// The library is intentionally headless — no router, no QueryClient,
// no transport, no theme. This file is what `clickhouse-explorer-in-
// rubix-shell.md` PR 4 calls "the demo host": it wires the three
// providers the library expects (TanStack Query, StarterClient,
// tailwind tokens) and mounts `<Explorer />` at `/`. Designed to be
// the smallest viable surface so the explorer keeps shipping as a
// standalone "ClickHouse over starter-server" demo even though the
// primary consumer is now the rubix admin shell at
// `/admin/warehouse` → Explorer tab.
//
// Design notes: rubix/docs/design/warehouse/explorer/README.md.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StarterClient } from "@nube/starter-client-ts";
import { StarterClientProvider } from "@nube/starter-client-react";
import { Explorer } from "@nube/starter-ui-ch-explorer";

import "./globals.css";

// Same base URL the ch-explorer binary listens on. The library's
// fetchers call `/api/warehouse/ch/*` against this client.
const starter = new StarterClient({
  baseUrl: window.location.origin,
  fetch: globalThis.fetch.bind(globalThis),
});

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30_000,
      gcTime: 5 * 60_000,
      retry: 1,
    },
  },
});

const root = document.getElementById("root");
if (!root) throw new Error("missing #root in index.html");

createRoot(root).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <StarterClientProvider client={starter}>
        <main className="mx-auto max-w-7xl p-6">
          <Explorer />
        </main>
      </StarterClientProvider>
    </QueryClientProvider>
  </StrictMode>,
);
