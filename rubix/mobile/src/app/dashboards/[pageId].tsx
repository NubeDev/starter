// app/dashboards/[pageId].tsx — Block 5: render a server-driven
// dashboard page via the native SDUI stack.
//
// The pageId in the URL is the IR `page_ref` (e.g.
// `dashboard.disk-overview`) — the same identifier the web app uses.
// On mount we record it as the active connection's last-opened page
// so a cold start resumes here (boot redirect in app/index.tsx).

import { useLocalSearchParams } from 'expo-router';
import { useEffect } from 'react';
import { Text, View } from 'react-native';

import { useLocalDb } from '../../local-db/provider';
import { setLastPage } from '../../local-db/state/last-page';
import { useConnection } from '../../connection/provider';
import { SduiPageNative } from '../../sdui/page';
import { useTheme } from '../../theme/provider';

export default function DashboardPage() {
  const { pageId } = useLocalSearchParams<{ pageId: string }>();
  const db = useLocalDb();
  const { active } = useConnection();
  const theme = useTheme();

  useEffect(() => {
    if (active && pageId) {
      void setLastPage(db, active.id, pageId);
    }
  }, [db, active, pageId]);

  if (!pageId) {
    return (
      <View style={{ flex: 1, padding: 24, backgroundColor: theme.background }}>
        <Text style={{ color: theme.foreground }}>Missing page id.</Text>
      </View>
    );
  }

  return <SduiPageNative pageRef={pageId} />;
}
