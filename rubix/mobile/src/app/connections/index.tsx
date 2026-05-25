// app/connections/index.tsx — list saved servers.

import { Link, useRouter } from 'expo-router';
import { useCallback, useEffect, useState } from 'react';
import {
  ActivityIndicator,
  Alert,
  FlatList,
  Pressable,
  RefreshControl,
  Text,
  View,
} from 'react-native';
import { FormattedMessage, useIntl } from 'react-intl';

import { useLocalDb } from '../../local-db/provider';
import { listConnections } from '../../local-db/connection/list';
import { deleteConnection } from '../../local-db/connection/delete';
import { expoSecureTokenStore } from '../../local-db/token/expo-secure-store';
import type { Connection } from '../../local-db/connection/types';
import { useConnection } from '../../connection/provider';
import { useTheme } from '../../theme/provider';

export default function ConnectionsIndex() {
  const db = useLocalDb();
  const theme = useTheme();
  const intl = useIntl();
  const router = useRouter();
  const { setActiveId, active } = useConnection();
  const [rows, setRows] = useState<Connection[] | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  const refresh = useCallback(async () => {
    setRefreshing(true);
    try {
      setRows(await listConnections(db));
    } finally {
      setRefreshing(false);
    }
  }, [db]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function activate(id: string) {
    await setActiveId(id);
    router.replace('/');
  }

  function confirmDelete(c: Connection) {
    Alert.alert(
      intl.formatMessage({ id: 'connections.delete' }),
      intl.formatMessage({ id: 'connections.delete.confirm' }, { label: c.label }),
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: intl.formatMessage({ id: 'connections.delete' }),
          style: 'destructive',
          onPress: async () => {
            await deleteConnection(db, expoSecureTokenStore, c.id);
            if (active?.id === c.id) {
              await setActiveId(null);
            }
            await refresh();
          },
        },
      ],
    );
  }

  if (!rows) {
    return (
      <View style={{ flex: 1, alignItems: 'center', justifyContent: 'center' }}>
        <ActivityIndicator />
      </View>
    );
  }

  return (
    <View style={{ flex: 1, backgroundColor: theme.background, padding: 16 }}>
      <View
        style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}
      >
        <Text style={{ fontSize: 24, fontWeight: '600', color: theme.foreground }}>
          <FormattedMessage id="connections.title" />
        </Text>
        <Link href="/connections/new" asChild>
          <Pressable
            style={{
              backgroundColor: theme.accent,
              paddingVertical: 8,
              paddingHorizontal: 12,
              borderRadius: 6,
            }}
            accessibilityRole="button"
          >
            <Text style={{ color: '#fff', fontWeight: '500' }}>
              <FormattedMessage id="connections.add" />
            </Text>
          </Pressable>
        </Link>
      </View>
      <FlatList
        data={rows}
        keyExtractor={(c) => c.id}
        refreshControl={<RefreshControl refreshing={refreshing} onRefresh={refresh} />}
        ListEmptyComponent={
          <Text style={{ color: theme.foreground, opacity: 0.7 }}>
            <FormattedMessage id="connections.empty" />
          </Text>
        }
        renderItem={({ item }) => (
          <View
            style={{
              padding: 12,
              borderWidth: 1,
              borderColor: active?.id === item.id ? theme.accent : theme.border,
              borderRadius: 8,
              marginBottom: 8,
            }}
          >
            <Text style={{ fontWeight: '600', color: theme.foreground }}>{item.label}</Text>
            <Text style={{ color: theme.foreground, opacity: 0.7, marginTop: 2 }}>
              {item.baseUrl}
            </Text>
            <View style={{ flexDirection: 'row', marginTop: 8, gap: 12 }}>
              <Pressable onPress={() => activate(item.id)} accessibilityRole="button">
                <Text style={{ color: theme.accent }}>
                  <FormattedMessage id="connections.activate" />
                </Text>
              </Pressable>
              <Link href={`/connections/${item.id}` as never} asChild>
                <Pressable accessibilityRole="button">
                  <Text style={{ color: theme.accent }}>Edit</Text>
                </Pressable>
              </Link>
              <Pressable onPress={() => confirmDelete(item)} accessibilityRole="button">
                <Text style={{ color: '#B91C1C' }}>
                  <FormattedMessage id="connections.delete" />
                </Text>
              </Pressable>
            </View>
          </View>
        )}
      />
    </View>
  );
}
