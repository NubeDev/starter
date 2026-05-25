// sdui/provider.tsx — mounts <SduiProvider> with a transport built
// from the active connection's StarterClient. Re-keys on connection
// change so the transport instance never outlives its client.
//
// The native renderer side-effect import (registering RenderPage,
// RenderRow, … with the shared registry) lives in app/_layout.tsx so
// it runs once at app boot before this provider mounts.

import { useMemo, type ReactNode } from 'react';
import { SduiProvider } from '@nube/starter-ui-sdui-react/headless';

import { useConnection } from '../connection/provider';
import { makeSduiTransport } from './transport';

export function MobileSduiProvider({ children }: { children: ReactNode }) {
  const { client, active } = useConnection();

  // No active connection → render children without an SDUI provider.
  // The route guard ensures no SDUI consumer is reachable in that
  // state (login + /connections/* don't touch <SduiPage>).
  if (!client || !active) {
    return <>{children}</>;
  }

  return <SduiProviderForClient client={client.starter}>{children}</SduiProviderForClient>;
}

function SduiProviderForClient({
  client,
  children,
}: {
  client: import('@nube/starter-client-ts').StarterClient;
  children: ReactNode;
}) {
  // Memo keyed by `client` identity — ConnectionProvider constructs a
  // new StarterClient on every active-id switch, so a new transport
  // (and a new context value) propagates automatically.
  const transport = useMemo(() => makeSduiTransport(client), [client]);
  return <SduiProvider transport={transport}>{children}</SduiProvider>;
}
