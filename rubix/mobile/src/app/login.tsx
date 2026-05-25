// app/login.tsx — per-connection login screen.
//
// Two-step login per APP-SHELL.md §Strategy:
//
//   1. POST credentials to <baseUrl>/api/v1/auth/token via
//      `issueTokenForConnection`. Server returns
//      { token, expires_at, token_type }.
//   2. Persist via `expoSecureTokenStore`, then install on the
//      in-memory `StarterClient` via `tokenStrategy.login`.
//
// The screen reads the active connection from `ConnectionProvider`. If
// none is active it sends the operator back to `/connections/new` —
// you cannot log in before you have picked a server.

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

import { useConnection } from '../connection/provider';
import { expoSecureTokenStore } from '../local-db/token/expo-secure-store';
import { loginWithCredentials } from '../auth/strategy';
import { DEV_LOGIN_DEFAULTS, PREFILL_LOGIN_IN_DEV } from '../auth/dev-defaults';
import { takePendingRoute } from '../state/pending-route';
import { useTheme } from '../theme/provider';

export default function LoginScreen() {
  const { active, client } = useConnection();
  const router = useRouter();
  const intl = useIntl();
  const theme = useTheme();
  const [email, setEmail] = useState(PREFILL_LOGIN_IN_DEV ? DEV_LOGIN_DEFAULTS.email : '');
  const [password, setPassword] = useState(
    PREFILL_LOGIN_IN_DEV ? DEV_LOGIN_DEFAULTS.password : '',
  );
  const [tenantId, setTenantId] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [errorKey, setErrorKey] = useState<string | null>(null);
  const [errorDetail, setErrorDetail] = useState<string | null>(null);

  if (!active || !client) {
    // Defensive — `<Stack>` should never route here when there is no
    // active connection. If we still landed, send the operator back
    // to the safe entry point.
    return null;
  }

  async function submit() {
    setSubmitting(true);
    setErrorKey(null);
    setErrorDetail(null);
    try {
      await loginWithCredentials({
        client: client!.starter,
        secureStore: expoSecureTokenStore,
        connectionId: active!.id,
        baseUrl: active!.baseUrl,
        email: email.trim(),
        password,
        tenantId: tenantId.trim() || undefined,
      });
      // Restore the route the operator was on when 401'd, if any.
      const pending = takePendingRoute();
      router.replace((pending?.pathname ?? '/') as never);
    } catch (e: unknown) {
      const err = e as { status?: number; body?: { error?: string }; message?: string };
      if (err.status === 401) {
        setErrorKey('auth.login.error.bad_credentials');
      } else if (err.status === 400 && err.body?.error === 'password_not_set') {
        setErrorKey('auth.login.error.password_not_set');
      } else if (err.status === 400 && err.body?.error === 'missing_tenant_id') {
        setErrorKey('auth.login.error.missing_tenant_id');
      } else if (err.status === 409 && err.body?.error === 'tenant_required') {
        setErrorKey('auth.login.error.tenant_required');
      } else if (err.status === undefined) {
        setErrorKey('auth.login.error.network');
      } else {
        setErrorKey('auth.login.error.unknown');
        setErrorDetail(err.message ?? String(err.status));
      }
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <KeyboardAvoidingView
      behavior={Platform.OS === 'ios' ? 'padding' : undefined}
      style={{ flex: 1, backgroundColor: theme.background }}
    >
      <ScrollView
        contentContainerStyle={{ flexGrow: 1, padding: 24, justifyContent: 'center' }}
      >
        <Text
          style={{
            fontSize: 28,
            fontWeight: '600',
            color: theme.foreground,
            marginBottom: 8,
          }}
          accessibilityRole="header"
        >
          <FormattedMessage id="auth.login.title" />
        </Text>
        <Text style={{ color: theme.foreground, opacity: 0.7, marginBottom: 24 }}>
          {active.label} · {active.baseUrl}
        </Text>
        <Field
          label={intl.formatMessage({ id: 'auth.login.email' })}
          value={email}
          onChangeText={setEmail}
          autoCapitalize="none"
          autoComplete="email"
          keyboardType="email-address"
          theme={theme}
        />
        <Field
          label={intl.formatMessage({ id: 'auth.login.password' })}
          value={password}
          onChangeText={setPassword}
          secureTextEntry
          autoComplete="password"
          theme={theme}
        />
        <Field
          label={intl.formatMessage({ id: 'auth.login.tenant' })}
          value={tenantId}
          onChangeText={setTenantId}
          autoCapitalize="none"
          theme={theme}
        />
        {errorKey && (
          <View
            accessibilityRole="alert"
            style={{
              marginTop: 12,
              padding: 12,
              borderRadius: 8,
              borderWidth: 1,
              borderColor: '#FCA5A5',
              backgroundColor: '#FEF2F2',
            }}
          >
            <Text style={{ color: '#7F1D1D' }}>
              <FormattedMessage
                id={errorKey}
                values={{ detail: errorDetail ?? '' }}
              />
            </Text>
          </View>
        )}
        <Pressable
          onPress={submit}
          disabled={submitting || !email || !password}
          style={{
            marginTop: 24,
            backgroundColor: theme.accent,
            paddingVertical: 14,
            borderRadius: 8,
            alignItems: 'center',
            opacity: submitting || !email || !password ? 0.6 : 1,
          }}
          accessibilityRole="button"
        >
          {submitting ? (
            <ActivityIndicator color="#fff" />
          ) : (
            <Text style={{ color: '#fff', fontWeight: '600' }}>
              <FormattedMessage id="auth.login.submit" />
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
        }}
        placeholderTextColor={theme.border}
      />
    </View>
  );
}
