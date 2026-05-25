// sdui/page.tsx — the mobile twin of headless `<SduiPage>`.
//
// The headless implementation (packages/starter-ui-sdui-react/src/
// headless/sdui-page.tsx) renders web `<div>` placeholders for the
// transient loading / error / dry-run states. The happy path delegates
// to `<Render>`, which is platform-neutral — registered renderers from
// `@nube/starter-ui-sdui-native` take over from there. We can't reuse
// the web `<div>` branches under React Native, so this file mirrors
// the same control flow with RN primitives.
//
// If the headless package later grows platform-neutral default UI for
// loading / error / dry-run (e.g. a render-prop or null-default),
// delete this file and import `SduiPage` directly.

import { useMemo } from 'react';
import { ActivityIndicator, ScrollView, Text, View } from 'react-native';
import {
  PageStateProvider,
  Render,
  listRenderers,
  useSduiResolve,
  useSduiSubscriptions,
  type SduiPageProps,
} from '@nube/starter-ui-sdui-react/headless';
import type {
  ClientCapabilities,
  UiResolveResponseOk,
} from '@nube/starter-ui-ir';
import { IR_VERSION } from '@nube/starter-ui-ir';

import { useTheme } from '../theme/provider';

export function SduiPageNative(props: SduiPageProps) {
  return (
    <PageStateProvider initial={props.initialPageState}>
      <SduiPageNativeInner {...props} />
    </PageStateProvider>
  );
}

function SduiPageNativeInner({
  pageRef,
  targetRef,
  stack,
  capabilities,
}: SduiPageProps) {
  const theme = useTheme();
  const caps = useMemo<ClientCapabilities>(
    () =>
      capabilities ?? {
        ir_versions: [IR_VERSION],
        custom_renderers: listRenderers(),
      },
    [capabilities],
  );

  const query = useSduiResolve({ pageRef, targetRef, stack, capabilities: caps });
  const data = query.data;
  const ok = data && isOk(data) ? data : undefined;

  useSduiSubscriptions(ok?.subscriptions, [
    'sdui',
    'resolve',
    {
      page_ref: pageRef,
      target_ref: targetRef,
      stack,
      capabilities: caps,
    },
  ]);

  if (query.isError) {
    return (
      <View style={{ flex: 1, padding: 16, backgroundColor: theme.background }}>
        <Text style={{ color: '#B91C1C' }} accessibilityRole="alert">
          {query.error?.message ?? 'resolve failed'}
        </Text>
      </View>
    );
  }
  if (!data) {
    return (
      <View
        style={{
          flex: 1,
          alignItems: 'center',
          justifyContent: 'center',
          backgroundColor: theme.background,
        }}
      >
        <ActivityIndicator color={theme.accent} />
      </View>
    );
  }
  if (!ok) {
    const errors =
      (data as { errors?: { location: string; message: string }[] }).errors ?? [];
    return (
      <ScrollView
        style={{ flex: 1, backgroundColor: theme.background }}
        contentContainerStyle={{ padding: 16 }}
      >
        <Text style={{ fontWeight: '600', color: theme.foreground, marginBottom: 8 }}>
          Page is in dry-run state:
        </Text>
        {errors.map((e, i) => (
          <Text key={i} style={{ color: theme.foreground, marginTop: 4 }}>
            {e.location}: {e.message}
          </Text>
        ))}
      </ScrollView>
    );
  }

  return (
    <ScrollView
      style={{ flex: 1, backgroundColor: theme.background }}
      contentContainerStyle={{ padding: 12 }}
    >
      <Render node={ok.render.root} />
    </ScrollView>
  );
}

function isOk(r: unknown): r is UiResolveResponseOk {
  return Boolean(r && typeof r === 'object' && 'render' in r);
}
