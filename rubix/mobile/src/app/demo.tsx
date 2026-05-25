// app/demo.tsx — NativeWind v4 / gluestack-v3-style spike.
//
// Purpose: prove the NativeWind pipeline (the engine gluestack-ui v3
// sits on) renders end-to-end on Expo SDK 54 + Hermes + Expo Go. The
// screen is intentionally visual: KPI cards, gradient hero, action
// chips, mock chart bars — i.e. the kind of thing a real product
// dashboard looks like.
//
// Everything here uses className utilities only. No new providers, no
// new theme system — tokens come from tailwind.config.js which is
// the spike's stand-in for a generated `starter-theme-tokens` bridge.
//
// To open: from the app's index page or via Expo dev URL, navigate to
// `/demo`.

import { Ionicons } from '@expo/vector-icons';
import { Pressable, ScrollView, View, Text } from 'react-native';

export default function Demo() {
  return (
    <ScrollView className="flex-1 bg-background">
      <View className="px-5 pt-6 pb-24">
        {/* Hero card */}
        <View
          className="rounded-3xl p-6 mb-6 bg-primary"
          style={{
            shadowColor: '#7C3AED',
            shadowOffset: { width: 0, height: 12 },
            shadowOpacity: 0.25,
            shadowRadius: 20,
            elevation: 10,
          }}
        >
          <View className="flex-row items-center justify-between mb-4">
            <View className="flex-row items-center gap-2">
              <View className="w-9 h-9 rounded-2xl bg-white/20 items-center justify-center">
                <Ionicons name="server" size={18} color="#fff" />
              </View>
              <Text className="text-white/90 font-medium">Rubix · prod-01</Text>
            </View>
            <Ionicons name="ellipsis-horizontal" size={20} color="#fff" />
          </View>
          <Text className="text-white text-3xl font-bold tracking-tight">
            All systems normal
          </Text>
          <Text className="text-white/80 mt-1">12 services · 0 incidents</Text>

          <View className="flex-row gap-2 mt-5">
            <Chip label="2h" active />
            <Chip label="24h" />
            <Chip label="7d" />
            <Chip label="30d" />
          </View>
        </View>

        {/* KPI grid */}
        <View className="flex-row gap-3 mb-6">
          <Kpi icon="pulse" label="CPU" value="34%" trend="+2%" tone="success" />
          <Kpi icon="hardware-chip-outline" label="Memory" value="61%" trend="-1%" tone="warning" />
        </View>
        <View className="flex-row gap-3 mb-6">
          <Kpi icon="cloud-download-outline" label="Net In" value="128 MB/s" trend="+12%" tone="info" />
          <Kpi icon="cloud-upload-outline" label="Net Out" value="42 MB/s" trend="-3%" tone="info" />
        </View>

        {/* Chart card */}
        <View className="rounded-3xl border border-border bg-card p-5 mb-6">
          <View className="flex-row items-center justify-between mb-4">
            <View>
              <Text className="text-foreground text-base font-semibold">
                Request rate
              </Text>
              <Text className="text-muted-foreground text-xs">
                last 12 hours · rps
              </Text>
            </View>
            <View className="px-2.5 py-1 rounded-full bg-accent">
              <Text className="text-accent-foreground text-xs font-semibold">
                +18%
              </Text>
            </View>
          </View>
          <Bars />
        </View>

        {/* Activity */}
        <Text className="text-foreground text-base font-semibold mb-3">
          Recent activity
        </Text>
        <View className="rounded-3xl border border-border bg-card overflow-hidden">
          <Activity
            icon="checkmark-circle"
            tone="success"
            title="Deploy succeeded"
            sub="rubix-agent · 2m ago"
          />
          <Divider />
          <Activity
            icon="alert-circle"
            tone="warning"
            title="Latency spike resolved"
            sub="api-gateway · 14m ago"
          />
          <Divider />
          <Activity
            icon="person-add"
            tone="info"
            title="New operator: kim@nube.io"
            sub="auth · 1h ago"
          />
        </View>

        {/* CTA */}
        <Pressable
          className="mt-8 h-12 rounded-2xl bg-foreground items-center justify-center active:opacity-80"
          android_ripple={{ color: 'rgba(255,255,255,0.1)' }}
        >
          <Text className="text-white font-semibold">Open full dashboard</Text>
        </Pressable>
      </View>
    </ScrollView>
  );
}

function Chip({ label, active }: { label: string; active?: boolean }) {
  return (
    <View
      className={
        'px-3 py-1.5 rounded-full ' +
        (active ? 'bg-white' : 'bg-white/15')
      }
    >
      <Text
        className={
          'text-xs font-semibold ' +
          (active ? 'text-primary' : 'text-white/90')
        }
      >
        {label}
      </Text>
    </View>
  );
}

type Tone = 'success' | 'warning' | 'info';
function toneClasses(t: Tone) {
  if (t === 'success') return { bg: 'bg-success/10', fg: 'text-success' };
  if (t === 'warning') return { bg: 'bg-warning/10', fg: 'text-warning' };
  return { bg: 'bg-accent', fg: 'text-accent-foreground' };
}

function Kpi(props: {
  icon: React.ComponentProps<typeof Ionicons>['name'];
  label: string;
  value: string;
  trend: string;
  tone: Tone;
}) {
  const t = toneClasses(props.tone);
  const trendUp = props.trend.startsWith('+');
  return (
    <View className="flex-1 rounded-3xl border border-border bg-card p-4">
      <View className="flex-row items-center justify-between mb-3">
        <View className={`w-9 h-9 rounded-2xl items-center justify-center ${t.bg}`}>
          <Ionicons name={props.icon} size={18} color={trendColor(props.tone)} />
        </View>
        <View className="flex-row items-center gap-0.5">
          <Ionicons
            name={trendUp ? 'arrow-up' : 'arrow-down'}
            size={12}
            color={trendUp ? '#16A34A' : '#DC2626'}
          />
          <Text
            className={`text-xs font-semibold ${trendUp ? 'text-success' : 'text-destructive'}`}
          >
            {props.trend.replace(/^[+-]/, '')}
          </Text>
        </View>
      </View>
      <Text className="text-muted-foreground text-xs mb-1">{props.label}</Text>
      <Text className="text-foreground text-2xl font-bold tracking-tight">
        {props.value}
      </Text>
    </View>
  );
}

function trendColor(tone: Tone): string {
  if (tone === 'success') return '#16A34A';
  if (tone === 'warning') return '#F59E0B';
  return '#5B21B6';
}

function Bars() {
  // Static demo bars — height ratios chosen to look "alive". A real
  // version would plug into Victory Native or our own chart kit.
  const heights = [28, 44, 36, 60, 52, 78, 64, 92, 72, 88, 70, 96];
  return (
    <View className="flex-row items-end gap-1.5 h-32">
      {heights.map((h, i) => (
        <View
          key={i}
          style={{ height: h, flex: 1 }}
          className={`rounded-t-xl ${i === heights.length - 1 ? 'bg-primary' : 'bg-primary/30'}`}
        />
      ))}
    </View>
  );
}

function Divider() {
  return <View className="h-px bg-border ml-14" />;
}

function Activity(props: {
  icon: React.ComponentProps<typeof Ionicons>['name'];
  tone: Tone;
  title: string;
  sub: string;
}) {
  const t = toneClasses(props.tone);
  return (
    <View className="flex-row items-center p-4 gap-3">
      <View className={`w-10 h-10 rounded-2xl items-center justify-center ${t.bg}`}>
        <Ionicons name={props.icon} size={20} color={trendColor(props.tone)} />
      </View>
      <View className="flex-1">
        <Text className="text-foreground font-semibold">{props.title}</Text>
        <Text className="text-muted-foreground text-xs mt-0.5">{props.sub}</Text>
      </View>
      <Ionicons name="chevron-forward" size={18} color="#94A3B8" />
    </View>
  );
}
