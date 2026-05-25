// app/connections/index.tsx — list saved servers.
//
// Built on `@nube/starter-ui-kit-native` primitives so the look
// matches the rest of the app (Card, Button, Badge, Text).

import { Link, useRouter } from 'expo-router';
import { useCallback, useEffect, useState } from 'react';
import { Alert, FlatList, RefreshControl, View } from 'react-native';
import { Ionicons } from '@expo/vector-icons';
import { FormattedMessage, useIntl } from 'react-intl';

import {
  Badge,
  Button,
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
  Row,
  Spinner,
  Text as KitText,
  useTheme as useKitTheme,
} from '@nube/starter-ui-kit-native';

import { useLocalDb } from '../../local-db/provider';
import { listConnections } from '../../local-db/connection/list';
import { deleteConnection } from '../../local-db/connection/delete';
import { expoSecureTokenStore } from '../../local-db/token/expo-secure-store';
import type { Connection } from '../../local-db/connection/types';
import { useConnection } from '../../connection/provider';

export default function ConnectionsIndex() {
  const db = useLocalDb();
  const t = useKitTheme();
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
      <View
        style={{
          flex: 1,
          alignItems: 'center',
          justifyContent: 'center',
          backgroundColor: t.color('background'),
        }}
      >
        <Spinner />
      </View>
    );
  }

  return (
    <View style={{ flex: 1, backgroundColor: t.color('background'), padding: 16 }}>
      <Row style={{ justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
        <KitText variant="title" weight="semibold">
          <FormattedMessage id="connections.title" />
        </KitText>
        <Link href="/connections/new" asChild>
          <Button
            accessibilityLabel={intl.formatMessage({ id: 'connections.add' })}
            size="sm"
          >
            <Row style={{ alignItems: 'center', gap: 6 }}>
              <Ionicons name="add" size={18} color="#fff" />
              <KitText weight="medium" style={{ color: '#fff' }}>
                {intl.formatMessage({ id: 'connections.add' })}
              </KitText>
            </Row>
          </Button>
        </Link>
      </Row>

      <FlatList
        data={rows}
        keyExtractor={(c) => c.id}
        contentContainerStyle={{ gap: 12, paddingBottom: 24 }}
        refreshControl={<RefreshControl refreshing={refreshing} onRefresh={refresh} />}
        ListEmptyComponent={
          <Card>
            <CardContent>
              <Row style={{ alignItems: 'center', gap: 8, marginBottom: 8 }}>
                <Ionicons
                  name="server-outline"
                  size={20}
                  color={t.color('muted-foreground')}
                />
                <KitText color="muted">
                  <FormattedMessage id="connections.empty" />
                </KitText>
              </Row>
            </CardContent>
          </Card>
        }
        renderItem={({ item }) => {
          const isActive = active?.id === item.id;
          return (
            <Card>
              <CardHeader>
                <Row style={{ alignItems: 'center', justifyContent: 'space-between' }}>
                  <CardTitle>{item.label}</CardTitle>
                  {isActive ? (
                    <Badge variant="default" accessibilityLabel="Active">
                      {intl.formatMessage({ id: 'connections.active' })}
                    </Badge>
                  ) : null}
                </Row>
              </CardHeader>
              <CardContent>
                <KitText variant="caption" color="muted" numberOfLines={1}>
                  {item.baseUrl}
                </KitText>
              </CardContent>
              <CardFooter>
                <Row style={{ gap: 8, flex: 1, justifyContent: 'flex-end' }}>
                  {!isActive && (
                    <Button
                      size="sm"
                      variant="default"
                      onPress={() => activate(item.id)}
                      accessibilityLabel={intl.formatMessage({ id: 'connections.activate' })}
                    >
                      {intl.formatMessage({ id: 'connections.activate' })}
                    </Button>
                  )}
                  <Link href={`/connections/${item.id}` as never} asChild>
                    <Button
                      size="sm"
                      variant="outline"
                      accessibilityLabel="Edit"
                    >
                      Edit
                    </Button>
                  </Link>
                  <Button
                    size="sm"
                    variant="destructive"
                    onPress={() => confirmDelete(item)}
                    accessibilityLabel={intl.formatMessage({ id: 'connections.delete' })}
                  >
                    {intl.formatMessage({ id: 'connections.delete' })}
                  </Button>
                </Row>
              </CardFooter>
            </Card>
          );
        }}
      />
    </View>
  );
}
