// Test-only helpers — kept out of the public barrel.
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { PageStateProvider } from "../headless/page-state.js";
import { SduiProvider } from "../headless/sdui-provider.js";
import type { SduiTransport } from "../headless/transport/index.js";

export function nullTransport(): SduiTransport {
  return {
    resolve: async () => ({ render: { ir_version: 5, root: { type: "page" } } }),
    action: async () => ({ type: "noop" }),
    table: async () => ({ rows: [] }),
    subscribe: () => () => {},
  };
}

export function renderHarness(el: ReactElement, pageState: Record<string, unknown> = {}): string {
  const qc = new QueryClient();
  return renderToStaticMarkup(
    <QueryClientProvider client={qc}>
      <SduiProvider transport={nullTransport()}>
        <PageStateProvider initial={pageState}>{el}</PageStateProvider>
      </SduiProvider>
    </QueryClientProvider>,
  );
}
