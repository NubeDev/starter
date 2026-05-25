// app/connections/new.tsx — add a server.
//
// Posts a /healthz probe before storing (network sanity check). The
// connection row is created either way — operators legitimately add
// servers that aren't reachable yet (mobile data turned off, VPN
// dropped) — but we surface the probe result inline so they know.
//
// The screen also offers a LAN scanner: tap "Scan LAN" and the app
// fetches `/healthz` on every host in the device's /24 subnet on
// port 8088 (configurable). Hits stream in as the sweep progresses;
// tapping one fills in the server URL.

import { useRouter } from 'expo-router';
import { useState } from 'react';
import {
  ActivityIndicator,
  KeyboardAvoidingView,
  Platform,
  Pressable,
  ScrollView,
  Text,
  TextInput,
  View,
} from 'react-native';
import { FormattedMessage, useIntl } from 'react-intl';

import { useLocalDb } from '../../local-db/provider';
import { createConnection } from '../../local-db/connection/create';
import { touchConnection } from '../../local-db/connection/touch';
import { useConnection } from '../../connection/provider';
import { useLanScan } from '../../connection/use-lan-scan';
import type { ScanHit } from '../../connection/scan';
import { useTheme } from '../../theme/provider';

export default function NewConnection() {
  const db = useLocalDb();
  const theme = useTheme();
  const intl = useIntl();
  const router = useRouter();
  const { setActiveId } = useConnection();
  const [label, setLabel] = useState('');
  const [baseUrl, setBaseUrl] = useState('');
  const [colour, setColour] = useState('');
  const [notes, setNotes] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [warning, setWarning] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const scan = useLanScan();
  const [scanPort, setScanPort] = useState('8088');

  function applyHit(hit: ScanHit): void {
    setBaseUrl(hit.baseUrl);
    if (!label.trim()) setLabel(`rubix @ ${hit.ip}`);
  }

  async function save() {
    setSubmitting(true);
    setWarning(null);
    setError(null);
    try {
      const trimmedUrl = baseUrl.trim().replace(/\/+$/, '');
      const result = await createConnection(db, {
        label: label.trim(),
        baseUrl: trimmedUrl,
        colour: colour.trim() || undefined,
        notes: notes.trim() || undefined,
      });
      if (result.status === 'duplicate-base-url') {
        setWarning(intl.formatMessage({ id: 'connections.new.duplicate_warning' }));
      }
      // Best-effort probe.
      try {
        const resp = await fetch(`${trimmedUrl}/healthz`);
        if (resp.ok) {
          const body = (await resp.json().catch(() => null)) as { version?: string } | null;
          await touchConnection(db, result.connection.id, body?.version ?? null);
        }
      } catch {
        /* ignore — operator can retry from list */
      }
      await setActiveId(result.connection.id);
      router.replace('/');
    } catch (e) {
      setError(String(e));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <KeyboardAvoidingView
      behavior={Platform.OS === 'ios' ? 'padding' : undefined}
      style={{ flex: 1, backgroundColor: theme.background }}
    >
      <ScrollView contentContainerStyle={{ padding: 24 }}>
        <Text
          style={{ fontSize: 24, fontWeight: '600', color: theme.foreground, marginBottom: 16 }}
          accessibilityRole="header"
        >
          <FormattedMessage id="connections.new.title" />
        </Text>
        <ScannerSection
          theme={theme}
          scan={scan}
          scanPort={scanPort}
          setScanPort={setScanPort}
          onApply={applyHit}
        />
        <Field label={intl.formatMessage({ id: 'connections.new.label' })} value={label} onChangeText={setLabel} theme={theme} />
        <Field
          label={intl.formatMessage({ id: 'connections.new.base_url' })}
          value={baseUrl}
          onChangeText={setBaseUrl}
          autoCapitalize="none"
          keyboardType="url"
          placeholder="https://rubix.example.com"
          theme={theme}
        />
        <Field label={intl.formatMessage({ id: 'connections.new.colour' })} value={colour} onChangeText={setColour} autoCapitalize="none" theme={theme} />
        <Field label={intl.formatMessage({ id: 'connections.new.notes' })} value={notes} onChangeText={setNotes} multiline theme={theme} />
        {warning && (
          <Text style={{ marginTop: 12, color: '#92400E' }} accessibilityRole="alert">
            {warning}
          </Text>
        )}
        {error && (
          <Text style={{ marginTop: 12, color: '#B91C1C' }} accessibilityRole="alert">
            {error}
          </Text>
        )}
        <Pressable
          onPress={save}
          disabled={submitting || !label || !baseUrl}
          style={{
            marginTop: 24,
            backgroundColor: theme.accent,
            paddingVertical: 14,
            borderRadius: 8,
            alignItems: 'center',
            opacity: submitting || !label || !baseUrl ? 0.6 : 1,
          }}
          accessibilityRole="button"
        >
          {submitting ? (
            <ActivityIndicator color="#fff" />
          ) : (
            <Text style={{ color: '#fff', fontWeight: '600' }}>
              <FormattedMessage id="connections.new.save" />
            </Text>
          )}
        </Pressable>
      </ScrollView>
    </KeyboardAvoidingView>
  );
}

function Field(
  props: React.ComponentProps<typeof TextInput> & {
    label: string;
    theme: { foreground: string; border: string };
  },
) {
  const { label, theme, ...rest } = props;
  return (
    <View style={{ marginTop: 12 }}>
      <Text style={{ marginBottom: 4, color: theme.foreground }}>{label}</Text>
      <TextInput
        {...rest}
        style={{
          borderWidth: 1,
          borderColor: theme.border,
          borderRadius: 8,
          padding: 12,
          color: theme.foreground,
          minHeight: rest.multiline ? 80 : undefined,
          textAlignVertical: rest.multiline ? 'top' : undefined,
        }}
      />
    </View>
  );
}

interface ScannerSectionProps {
  theme: { foreground: string; border: string; accent: string; background: string };
  scan: ReturnType<typeof useLanScan>;
  scanPort: string;
  setScanPort: (v: string) => void;
  onApply: (hit: ScanHit) => void;
}

function ScannerSection({ theme, scan, scanPort, setScanPort, onApply }: ScannerSectionProps) {
  const intl = useIntl();

  function startScan(): void {
    const parsed = Number.parseInt(scanPort, 10);
    const port = Number.isFinite(parsed) && parsed > 0 && parsed < 65536 ? parsed : 8088;
    void scan.start({ port });
  }

  return (
    <View
      style={{
        marginTop: 8,
        marginBottom: 4,
        padding: 12,
        borderWidth: 1,
        borderColor: theme.border,
        borderRadius: 8,
      }}
    >
      <Text style={{ color: theme.foreground, fontWeight: '600', marginBottom: 4 }}>
        <FormattedMessage id="connections.new.scan.title" />
      </Text>
      <Text style={{ color: theme.foreground, opacity: 0.7, fontSize: 12, marginBottom: 8 }}>
        <FormattedMessage id="connections.new.scan.hint" />
      </Text>
      <View style={{ flexDirection: 'row', alignItems: 'center', gap: 8 }}>
        <View style={{ flex: 1 }}>
          <Text style={{ fontSize: 12, color: theme.foreground, marginBottom: 4 }}>
            <FormattedMessage id="connections.new.scan.port" />
          </Text>
          <TextInput
            value={scanPort}
            onChangeText={setScanPort}
            keyboardType="number-pad"
            editable={!scan.scanning}
            style={{
              borderWidth: 1,
              borderColor: theme.border,
              borderRadius: 8,
              padding: 10,
              color: theme.foreground,
            }}
          />
        </View>
        <Pressable
          onPress={scan.scanning ? scan.cancel : startScan}
          style={{
            marginTop: 18,
            backgroundColor: scan.scanning ? '#9CA3AF' : theme.accent,
            paddingVertical: 12,
            paddingHorizontal: 16,
            borderRadius: 8,
          }}
          accessibilityRole="button"
          accessibilityLabel={intl.formatMessage({
            id: scan.scanning
              ? 'connections.new.scan.cancel'
              : 'connections.new.scan.start',
          })}
        >
          <Text style={{ color: '#fff', fontWeight: '600' }}>
            <FormattedMessage
              id={
                scan.scanning
                  ? 'connections.new.scan.cancel'
                  : 'connections.new.scan.start'
              }
            />
          </Text>
        </Pressable>
      </View>
      {scan.scanning && (
        <View style={{ flexDirection: 'row', alignItems: 'center', marginTop: 10 }}>
          <ActivityIndicator color={theme.accent} />
          <Text style={{ marginLeft: 8, color: theme.foreground }}>
            <FormattedMessage
              id="connections.new.scan.progress"
              values={{ done: scan.done, total: scan.total }}
            />
            {scan.localIp ? `  ·  ${scan.localIp}` : ''}
          </Text>
        </View>
      )}
      {scan.error && (
        <Text style={{ marginTop: 8, color: '#B91C1C' }} accessibilityRole="alert">
          {scan.error}
        </Text>
      )}
      {scan.hits.length > 0 && (
        <View style={{ marginTop: 10 }}>
          {scan.hits.map((hit) => (
            <Pressable
              key={hit.baseUrl}
              onPress={() => onApply(hit)}
              accessibilityRole="button"
              accessibilityLabel={intl.formatMessage(
                { id: 'connections.new.scan.use' },
                { url: hit.baseUrl },
              )}
              style={{
                paddingVertical: 10,
                paddingHorizontal: 12,
                borderRadius: 6,
                borderWidth: 1,
                borderColor: theme.border,
                marginTop: 6,
                backgroundColor: theme.background,
              }}
            >
              <Text style={{ color: theme.foreground, fontWeight: '600' }}>{hit.baseUrl}</Text>
              {hit.version && (
                <Text style={{ color: theme.foreground, opacity: 0.7, fontSize: 12 }}>
                  v{hit.version}
                </Text>
              )}
            </Pressable>
          ))}
        </View>
      )}
      {!scan.scanning && scan.hits.length === 0 && scan.done > 0 && !scan.error && (
        <Text style={{ marginTop: 8, color: theme.foreground, opacity: 0.7, fontSize: 12 }}>
          <FormattedMessage id="connections.new.scan.no_results" />
        </Text>
      )}
    </View>
  );
}
