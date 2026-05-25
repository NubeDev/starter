// nav/app-bar.tsx — single reusable header for the Expo Router stack.
//
// Uses `@nube/starter-ui-kit-native` design tokens directly via
// `useTheme()` so the bar matches the same palette as Card, Button,
// Input, etc. and respects the layout-prefs store the web app also
// writes to.
//
// Icons: `@expo/vector-icons/Ionicons` (ships with Expo Go, bitmap
// font only — no native module).

import * as React from 'react';
import { useRouter } from 'expo-router';
import { Ionicons } from '@expo/vector-icons';
import { Platform, Pressable, StatusBar, StyleSheet, View } from 'react-native';

import { Text as KitText, useTheme as useKitTheme } from '@nube/starter-ui-kit-native';

const HEIGHT = 52;

interface AppBarProps {
  navigation: { goBack(): void };
  options: { title?: string; headerTitle?: string | unknown };
  back?: unknown;
}

export function AppBar({ navigation, options, back }: AppBarProps) {
  const t = useKitTheme();
  const router = useRouter();
  const title =
    typeof options.headerTitle === 'string'
      ? options.headerTitle
      : (options.title ?? '');

  const topPad = Platform.OS === 'android' ? (StatusBar.currentHeight ?? 0) : 44;
  const fg = t.color('foreground');
  const bg = t.color('background');
  const border = t.color('border');
  const accent = t.color('primary');

  return (
    <View
      style={{
        paddingTop: topPad,
        backgroundColor: bg,
        borderBottomWidth: StyleSheet.hairlineWidth,
        borderBottomColor: border,
      }}
    >
      <View
        style={{
          height: HEIGHT,
          flexDirection: 'row',
          alignItems: 'center',
          paddingHorizontal: 4,
        }}
      >
        {back ? (
          <IconBtn
            name="chevron-back"
            onPress={() => navigation.goBack()}
            label="Back"
            color={accent}
            size={26}
          />
        ) : (
          <View style={{ width: 12 }} />
        )}
        <View style={{ flex: 1, alignItems: 'center' }}>
          <KitText variant="body" weight="semibold" numberOfLines={1}>
            {title}
          </KitText>
        </View>
        <IconBtn
          name="home-outline"
          onPress={() => router.replace('/')}
          label="Home"
          color={fg}
        />
        <IconBtn
          name="sparkles-outline"
          onPress={() => router.push('/demo')}
          label="Demo"
          color={accent}
        />
        <IconBtn
          name="server-outline"
          onPress={() => router.push('/connections')}
          label="Servers"
          color={fg}
        />
        <IconBtn
          name="settings-outline"
          onPress={() => router.push('/settings')}
          label="Settings"
          color={fg}
        />
      </View>
    </View>
  );
}

function IconBtn(props: {
  name: React.ComponentProps<typeof Ionicons>['name'];
  onPress: () => void;
  label: string;
  color: string;
  size?: number;
}) {
  const { name, onPress, label, color, size = 22 } = props;
  return (
    <Pressable
      onPress={onPress}
      accessibilityRole="button"
      accessibilityLabel={label}
      hitSlop={8}
      style={({ pressed }) => ({
        width: 44,
        height: HEIGHT,
        alignItems: 'center',
        justifyContent: 'center',
        opacity: pressed ? 0.5 : 1,
      })}
    >
      <Ionicons name={name} size={size} color={color} />
    </Pressable>
  );
}
