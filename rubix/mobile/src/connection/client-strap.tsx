// connection/client-strap.tsx — mounts the per-connection client +
// auth providers conditionally on there being an active connection.
//
// `RubixClientProvider` cannot exist without a `RubixClient`, and we
// cannot construct one without a base URL. So while no connection is
// active (fresh install, between switches) we render children directly
// — the only screens reachable in that state are `/connections/new` and
// the post-install `/` redirect; neither touches `useRubixClient`.

import type { ReactNode } from 'react';

import { RubixClientProvider } from '@nube/rubix-client-react';

import { useConnection } from './provider';

export function ClientStrap({ children }: { children: ReactNode }) {
  const { client, ready } = useConnection();
  if (!ready) {
    // ConnectionProvider already shows a splash via LocalDbProvider's
    // pending state; nothing to do here.
    return null;
  }
  if (!client) {
    // No active connection: render children directly. The route guard
    // in `app/_layout.tsx` ensures only `/connections/*` is reachable.
    return <>{children}</>;
  }
  return <RubixClientProvider client={client}>{children}</RubixClientProvider>;
}
