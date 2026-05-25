// app/settings.tsx — minimal settings (logout, switch connection).

import { Link, useRouter } from 'expo-router';
import { Pressable, Text, View } from 'react-native';
import { FormattedMessage } from 'react-intl';

import { useConnection } from '../connection/provider';
import { expoSecureTokenStore } from '../local-db/token/expo-secure-store';
import { clearBearer } from '../auth/install';
import { useTheme } from '../theme/provider';

export default function Settings() {
  const { active, client, refresh } = useConnection();
  const theme = useTheme();
  const router = useRouter();

  async function logout() {
    if (!active || !client) return;
    await clearBearer({
      client: client.starter,
      secureStore: expoSecureTokenStore,
      connectionId: active.id,
    });
    await refresh();
    router.replace('/login');
  }

  return (
    <View style={{ flex: 1, padding: 24, backgroundColor: theme.background }}>
      <Text
        style={{ fontSize: 24, fontWeight: '600', color: theme.foreground, marginBottom: 16 }}
        accessibilityRole="header"
      >
        <FormattedMessage id="settings.title" />
      </Text>
      {active && (
        <>
          <Text style={{ color: theme.foreground, marginBottom: 16 }}>
            {active.label} · {active.baseUrl}
          </Text>
          <Pressable
            onPress={logout}
            style={{
              borderWidth: 1,
              borderColor: '#B91C1C',
              paddingVertical: 12,
              paddingHorizontal: 16,
              borderRadius: 8,
              alignItems: 'center',
              marginBottom: 12,
            }}
            accessibilityRole="button"
          >
            <Text style={{ color: '#B91C1C', fontWeight: '500' }}>
              <FormattedMessage id="settings.logout" values={{ label: active.label }} />
            </Text>
          </Pressable>
        </>
      )}
      <Link href="/connections" asChild>
        <Pressable
          style={{
            paddingVertical: 12,
            paddingHorizontal: 16,
            borderRadius: 8,
            alignItems: 'center',
            borderWidth: 1,
            borderColor: theme.border,
          }}
          accessibilityRole="button"
        >
          <Text style={{ color: theme.foreground }}>
            <FormattedMessage id="connections.title" />
          </Text>
        </Pressable>
      </Link>
    </View>
  );
}
