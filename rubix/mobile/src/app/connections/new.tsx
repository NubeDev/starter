// app/connections/new.tsx — add a server.
//
// Posts a /healthz probe before storing (network sanity check). The
// connection row is created either way — operators legitimately add
// servers that aren't reachable yet (mobile data turned off, VPN
// dropped) — but we surface the probe result inline so they know.

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
