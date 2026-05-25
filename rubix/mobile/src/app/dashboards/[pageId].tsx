// app/dashboards/[pageId].tsx — single dashboard route.
//
// Block 4: shows the requested page id so deep-linking + last-page
// resume can be verified. Block 5 swaps the body for `<SduiPage pageRef={pageId}/>`.

import { useLocalSearchParams } from 'expo-router';
import { useEffect } from 'react';
import { Text, View } from 'react-native';

import { useLocalDb } from '../../local-db/provider';
import { setLastPage } from '../../local-db/state/last-page';
import { useConnection } from '../../connection/provider';
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

  return (
    <View style={{ flex: 1, backgroundColor: theme.background, padding: 24, justifyContent: 'center' }}>
      <Text style={{ fontSize: 20, fontWeight: '600', color: theme.foreground }}>
        Dashboard
      </Text>
      <Text style={{ marginTop: 8, color: theme.foreground, opacity: 0.7 }}>{pageId}</Text>
      <Text style={{ marginTop: 24, color: theme.foreground, opacity: 0.6 }}>
        SDUI renderer lands in Block 5.
      </Text>
    </View>
  );
}
