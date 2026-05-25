// app/dashboards/index.tsx — placeholder for Block 5.
//
// In Block 5 this becomes the SDUI-rendered dashboard list (resolves
// `/api/v1/ui/pages` through `<SduiPage>`). For Block 4 we show the
// active connection's identity so the operator confirms login worked.

import { Link } from 'expo-router';
import { Pressable, Text, View } from 'react-native';
import { FormattedMessage } from 'react-intl';

import { useConnection } from '../../connection/provider';
import { useTheme } from '../../theme/provider';

export default function DashboardsIndex() {
  const { active } = useConnection();
  const theme = useTheme();
  return (
    <View
      style={{
        flex: 1,
        padding: 24,
        backgroundColor: theme.background,
        justifyContent: 'center',
      }}
    >
      <Text
        style={{ fontSize: 24, fontWeight: '600', color: theme.foreground }}
        accessibilityRole="header"
      >
        <FormattedMessage id="dashboards.title" />
      </Text>
      {active && (
        <Text style={{ marginTop: 8, color: theme.foreground, opacity: 0.7 }}>
          {active.label} · {active.baseUrl}
        </Text>
      )}
      <Text style={{ marginTop: 24, color: theme.foreground }}>
        <FormattedMessage id="dashboards.empty" />
      </Text>
      <View style={{ flexDirection: 'row', gap: 12, marginTop: 24 }}>
        <Link href="/dashboards/dashboard.disk-overview" asChild>
          <Pressable accessibilityRole="button">
            <Text style={{ color: theme.accent }}>disk-overview</Text>
          </Pressable>
        </Link>
        <Link href="/connections" asChild>
          <Pressable accessibilityRole="button">
            <Text style={{ color: theme.accent }}>
              <FormattedMessage id="connections.title" />
            </Text>
          </Pressable>
        </Link>
        <Link href="/settings" asChild>
          <Pressable accessibilityRole="button">
            <Text style={{ color: theme.accent }}>
              <FormattedMessage id="settings.title" />
            </Text>
          </Pressable>
        </Link>
        <Link href="/login" asChild>
          <Pressable accessibilityRole="button">
            <Text style={{ color: theme.accent }}>
              <FormattedMessage id="auth.login.submit" />
            </Text>
          </Pressable>
        </Link>
      </View>
    </View>
  );
}
