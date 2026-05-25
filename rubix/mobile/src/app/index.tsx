// app/index.tsx — boot redirect.
//
// Per APP-SHELL.md §Provider stack: with no active connection, send
// the operator to `/connections/new`. With one active, jump to the
// last opened dashboard for that connection if known, else
// `/dashboards`. Block 4 ships the redirect to `/dashboards` index
// (a stub) — the per-page `<SduiPage>` renderer is Block 5 territory.

import { Redirect } from 'expo-router';
import { useEffect, useState } from 'react';
import { ActivityIndicator, View } from 'react-native';

import { useConnection } from '../connection/provider';
import { useLocalDb } from '../local-db/provider';
import { getLastPage } from '../local-db/state/last-page';

export default function IndexRedirect() {
  const { active, ready } = useConnection();
  const db = useLocalDb();
  const [target, setTarget] = useState<string | null>(null);

  useEffect(() => {
    if (!ready) return;
    if (!active) {
      setTarget('/connections/new');
      return;
    }
    getLastPage(db, active.id)
      .then((page) =>
        setTarget(page ? `/dashboards/${page}` : '/dashboards/dashboard.disk-overview'),
      )
      .catch(() => setTarget('/dashboards/dashboard.disk-overview'));
  }, [ready, active, db]);

  if (!target) {
    return (
      <View style={{ flex: 1, alignItems: 'center', justifyContent: 'center' }}>
        <ActivityIndicator />
      </View>
    );
  }
  return <Redirect href={target as never} />;
}
