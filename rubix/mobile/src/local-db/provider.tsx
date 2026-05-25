// provider.tsx — opens the local SQLite db once, exposes it via context.
//
// First entry in the provider stack after QueryClientProvider per
// APP-SHELL.md §Provider stack. While the db opens we show a tiny
// splash; if migrations fail we show the error so the operator can
// reinstall (no auto-wipe — that loses connections).

import React, { createContext, useContext, useEffect, useState } from 'react';
import { ActivityIndicator, Text, View } from 'react-native';

import { LocalDbError } from './errors';
import { type Database, openDb } from './open';

interface LocalDbCtx {
  readonly db: Database;
}

const Ctx = createContext<LocalDbCtx | null>(null);

export function LocalDbProvider({ children }: { children: React.ReactNode }) {
  const [db, setDb] = useState<Database | null>(null);
  const [error, setError] = useState<unknown>(null);

  useEffect(() => {
    let cancelled = false;
    openDb()
      .then((handle) => {
        if (!cancelled) setDb(handle);
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(e);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (error) {
    const msg =
      error instanceof LocalDbError
        ? `${error.kind}: ${error.message}`
        : String(error);
    return (
      <View style={{ flex: 1, alignItems: 'center', justifyContent: 'center', padding: 24 }}>
        <Text style={{ fontWeight: '600', marginBottom: 8 }}>Local database error</Text>
        <Text style={{ textAlign: 'center' }}>{msg}</Text>
      </View>
    );
  }

  if (!db) {
    return (
      <View style={{ flex: 1, alignItems: 'center', justifyContent: 'center' }}>
        <ActivityIndicator />
      </View>
    );
  }

  return <Ctx.Provider value={{ db }}>{children}</Ctx.Provider>;
}

export function useLocalDb(): Database {
  const ctx = useContext(Ctx);
  if (!ctx) {
    throw new Error('useLocalDb: missing <LocalDbProvider>');
  }
  return ctx.db;
}
