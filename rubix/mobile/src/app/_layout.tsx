// app/_layout.tsx — Expo Router root. Mounts the provider stack and
// the Stack navigator; all screens live as siblings to this file.
//
// SSE polyfill: rubix's starter SSE hook reads
// `(globalThis as { EventSource? }).EventSource` (APP-SHELL.md
// §Required RN runtime deps). `react-native-sse` is not a drop-in
// polyfill so we install it explicitly at boot.

import 'react-native-gesture-handler';
// NativeWind global stylesheet — must load before any className= is
// evaluated. Imported here at the root layout so every screen has the
// utility classes ready.
import '../../global.css';
// Side-effect import: registers all 16 native SDUI renderers
// (RenderPage, RenderRow, RenderCol, RenderKpi, RenderChart, …) with
// the shared registry exported from `@nube/starter-ui-sdui-react/
// headless`. Must run before the SDUI provider mounts; importing it
// at the top of the root layout is the earliest module-graph point.
import '@nube/starter-ui-sdui-native';
import { Stack } from 'expo-router';
import EventSource from 'react-native-sse';

import { AppBar } from '../nav/app-bar';
import { Providers } from '../providers';

// Globally available EventSource so the upstream SSE hook works in RN.
(globalThis as unknown as { EventSource: typeof EventSource }).EventSource =
  EventSource as unknown as typeof EventSource;

export default function RootLayout() {
  return (
    <Providers>
      <Stack screenOptions={{ header: (props) => <AppBar {...props} /> }}>
        {/* Login + boot redirect: no chrome. */}
        <Stack.Screen name="login" options={{ headerShown: false }} />
        <Stack.Screen name="index" options={{ headerShown: false }} />
        {/* Top-level screens. */}
        <Stack.Screen name="dashboards/index" options={{ title: 'Dashboards' }} />
        <Stack.Screen name="dashboards/[pageId]" options={{ title: 'Dashboard' }} />
        <Stack.Screen name="connections/index" options={{ title: 'Servers' }} />
        <Stack.Screen name="connections/new" options={{ title: 'Add server' }} />
        <Stack.Screen name="demo" options={{ title: 'Demo' }} />
      </Stack>
    </Providers>
  );
}
