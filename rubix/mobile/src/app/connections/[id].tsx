// app/connections/[id].tsx — edit one connection.

import { useLocalSearchParams, useRouter } from 'expo-router';
import { useEffect, useState } from 'react';
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
import { getConnection } from '../../local-db/connection/get';
import { updateConnection } from '../../local-db/connection/update';
import { useConnection } from '../../connection/provider';
import type { Connection } from '../../local-db/connection/types';
import { useTheme } from '../../theme/provider';

export default function EditConnection() {
  const { id } = useLocalSearchParams<{ id: string }>();
  const db = useLocalDb();
  const theme = useTheme();
  const intl = useIntl();
  const router = useRouter();
  const { active, refresh } = useConnection();
  const [row, setRow] = useState<Connection | null>(null);
  const [label, setLabel] = useState('');
  const [baseUrl, setBaseUrl] = useState('');
  const [colour, setColour] = useState('');
  const [notes, setNotes] = useState('');
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (!id) return;
    void getConnection(db, id).then((c) => {
      if (!c) return;
      setRow(c);
      setLabel(c.label);
      setBaseUrl(c.baseUrl);
      setColour(c.colour);
      setNotes(c.notes);
    });
  }, [db, id]);

  if (!row) {
    return (
      <View style={{ flex: 1, alignItems: 'center', justifyContent: 'center' }}>
        <ActivityIndicator />
      </View>
    );
  }

  async function save() {
    setSubmitting(true);
    try {
      await updateConnection(db, row!.id, { label, baseUrl, colour, notes });
      if (active?.id === row!.id) {
        // Rebuild the client so a baseUrl change takes effect immediately.
        await refresh();
      }
      router.back();
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
          <FormattedMessage id="connections.edit.title" values={{ label: row.label }} />
        </Text>
        <Field label={intl.formatMessage({ id: 'connections.new.label' })} value={label} onChangeText={setLabel} theme={theme} />
        <Field
          label={intl.formatMessage({ id: 'connections.new.base_url' })}
          value={baseUrl}
          onChangeText={setBaseUrl}
          autoCapitalize="none"
          keyboardType="url"
          theme={theme}
        />
        <Field label={intl.formatMessage({ id: 'connections.new.colour' })} value={colour} onChangeText={setColour} autoCapitalize="none" theme={theme} />
        <Field label={intl.formatMessage({ id: 'connections.new.notes' })} value={notes} onChangeText={setNotes} multiline theme={theme} />
        <Pressable
          onPress={save}
          disabled={submitting}
          style={{
            marginTop: 24,
            backgroundColor: theme.accent,
            paddingVertical: 14,
            borderRadius: 8,
            alignItems: 'center',
            opacity: submitting ? 0.6 : 1,
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
