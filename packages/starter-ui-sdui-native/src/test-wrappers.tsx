// Lightweight provider stack for renderer tests that touch
// `usePageState` / `useSduiAction` / `useSduiContext`. We don't
// need the real transport — stub everything to a noop and let the
// renderer dispatch without hitting the network.

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import * as React from "react";
import {
  PageStateProvider,
  SduiProvider,
  type SduiTransport,
} from "@nube/starter-ui-sdui-react/headless";

const noopTransport: SduiTransport = {
  resolve: async () => ({ render: { ir_version: 5, root: { type: "page" } } }),
  action: async () => ({ type: "noop" }),
  table: async () => ({ rows: [] }),
  subscribe: () => () => {},
};

export function Providers({
  children,
  initialState,
  customRenderers,
}: {
  children: React.ReactNode;
  initialState?: Record<string, unknown>;
  customRenderers?: Record<string, React.ComponentType<{ node: unknown }>>;
}) {
  const [qc] = React.useState(() => new QueryClient());
  return (
    <QueryClientProvider client={qc}>
      <SduiProvider
        transport={noopTransport}
        customRenderers={customRenderers as never}
      >
        <PageStateProvider initial={initialState}>{children}</PageStateProvider>
      </SduiProvider>
    </QueryClientProvider>
  );
}
