// connection/provider.tsx — owns the active connection's clients.
//
// Reads the active id from the local DB on mount, builds fresh
// `StarterClient` + `RubixClient` against the connection's `baseUrl`,
// rehydrates the bearer from `expo-secure-store` (if any), and
// publishes the id into the zustand atom that `starterQueryKey`
// consumers read. When the operator switches connection (or there is
// none on a fresh install) we rebuild everything so transport
// configuration cannot leak across instances.

import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react';
import type { RubixClient } from '@nube/rubix-client-ts';

import { useLocalDb } from '../local-db/provider';
import { getActiveConnection, getActiveConnectionId } from '../local-db/connection/active';
import { setActiveConnection } from '../local-db/connection/set-active';
import type { Connection } from '../local-db/connection/types';
import { expoSecureTokenStore } from '../local-db/token/expo-secure-store';
import { getToken } from '../local-db/token/get';

import { makeRubixClient } from '../lib/client';
import { _setActiveIdInternal } from './active-id-store';

interface ConnectionCtx {
  /** Currently active connection, or `null` if none selected. */
  readonly active: Connection | null;
  /** Fresh `RubixClient` bound to `active`. `null` while no active. */
  readonly client: RubixClient | null;
  /** Boot has settled (db read attempted). */
  readonly ready: boolean;
  /** Switch the active connection. Pass `null` to clear. */
  setActiveId(id: string | null): Promise<void>;
  /** Force a rebuild — used after the operator edits `base_url`. */
  refresh(): Promise<void>;
}

const Ctx = createContext<ConnectionCtx | null>(null);

export function ConnectionProvider({ children }: { children: React.ReactNode }) {
  const db = useLocalDb();
  const [active, setActive] = useState<Connection | null>(null);
  const [client, setClient] = useState<RubixClient | null>(null);
  const [ready, setReady] = useState(false);

  const buildFromDb = useCallback(async (): Promise<void> => {
    const conn = await getActiveConnection(db);
    if (!conn) {
      setActive(null);
      setClient(null);
      _setActiveIdInternal(null);
      return;
    }
    const rubix = makeRubixClient(conn.baseUrl);
    const token = await getToken(expoSecureTokenStore, conn.id);
    if (token) {
      rubix.starter.headers['Authorization'] = `Bearer ${token}`;
    }
    setActive(conn);
    setClient(rubix);
    _setActiveIdInternal(conn.id);
  }, [db]);

  useEffect(() => {
    let cancelled = false;
    buildFromDb()
      .catch(() => {
        // Boot must continue even if the active row is corrupt — the
        // operator can still reach /connections and pick a new one.
        setActive(null);
        setClient(null);
      })
      .finally(() => {
        if (!cancelled) setReady(true);
      });
    return () => {
      cancelled = true;
    };
  }, [buildFromDb]);

  const setActiveId = useCallback(
    async (id: string | null): Promise<void> => {
      await setActiveConnection(db, id);
      await buildFromDb();
    },
    [db, buildFromDb],
  );

  const value = useMemo<ConnectionCtx>(
    () => ({ active, client, ready, setActiveId, refresh: buildFromDb }),
    [active, client, ready, setActiveId, buildFromDb],
  );

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useConnection(): ConnectionCtx {
  const ctx = useContext(Ctx);
  if (!ctx) {
    throw new Error('useConnection: missing <ConnectionProvider>');
  }
  return ctx;
}

/** Read just the active connection id; cheaper than the full client. */
export function useActiveConnection(): Connection | null {
  return useConnection().active;
}

/** Read the bound `RubixClient`. Throws if none is active — call sites
 *  inside `/connections/*` and `/login` should branch on
 *  `useConnection().active` first. */
export function useActiveClient(): RubixClient {
  const { client } = useConnection();
  if (!client) {
    throw new Error('useActiveClient: no active connection');
  }
  return client;
}

/** Initialise the active-id store from the db at app boot, before the
 *  provider tree mounts. Currently a no-op — ConnectionProvider already
 *  syncs on mount — but kept as a single import surface so a future
 *  pre-render hydration path has a stable entry point. */
export async function bootHydrateActiveId(db: Parameters<typeof getActiveConnectionId>[0]): Promise<void> {
  const id = await getActiveConnectionId(db);
  _setActiveIdInternal(id);
}
