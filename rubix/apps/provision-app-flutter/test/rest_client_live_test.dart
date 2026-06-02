@Tags(['live'])
library;

import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:provision_app/core/api/bc_api.dart';
import 'package:provision_app/core/network/credential_store.dart';
import 'package:provision_app/core/network/transport.dart';

/// LIVE integration test for the REST client — drives the REAL [RubixTransport]
/// and [BcApi] against a running rubix-agent, with no UI and no Flutter plugins
/// in the way. This is the ground truth for "can the app actually read data".
///
/// Run against the default local agent:
///   flutter test test/rest_client_live_test.dart --tags live
///
/// Point at another agent / creds with --dart-define:
///   flutter test test/rest_client_live_test.dart --tags live \
///     --dart-define=AGENT_URL=http://192.168.1.50:8088 \
///     --dart-define=AGENT_EMAIL=op@example.com \
///     --dart-define=AGENT_PASSWORD=rubix-dev-passwd
///
/// It is tagged `live` so the normal `flutter test` run skips it (a CI box has
/// no agent). The matching `dart_test.yaml` declares the tag.

const _baseUrl =
    String.fromEnvironment('AGENT_URL', defaultValue: 'http://127.0.0.1:8088');
const _email =
    String.fromEnvironment('AGENT_EMAIL', defaultValue: 'op@example.com');
const _password = String.fromEnvironment(
  'AGENT_PASSWORD',
  defaultValue: 'rubix-dev-passwd',
);

/// In-memory [CredentialStore] so the test needs no keychain / shared_prefs
/// plugins (those only exist in a running app). Same surface the transport uses.
class _MemoryStore extends CredentialStore {
  String _base = '';
  String _token = '';

  @override
  Future<String> readBaseUrl() async => _base;
  @override
  Future<void> writeBaseUrl(String value) async => _base = value;
  @override
  Future<String> readToken() async => _token;
  @override
  Future<void> writeToken(String value) async => _token = value;
}

void main() {
  late RubixTransport transport;
  late BcApi bc;

  setUpAll(() {
    transport = RubixTransport(_MemoryStore(), dio: Dio());
    // BcApi with a no-op refresh callback — the test drives reads directly.
    bc = BcApi(transport, () {});
  });

  test('agent is reachable (ping /healthz)', () async {
    final ping = await transport.ping(_baseUrl);
    // ignore: avoid_print
    print('PING $_baseUrl -> ok=${ping.ok} msg="${ping.message}"');
    expect(
      ping.ok,
      isTrue,
      reason: 'Agent not reachable at $_baseUrl — is it running? (${ping.message})',
    );
  });

  test('login mints a token and resolves the principal', () async {
    final user = await transport.login(_baseUrl, _email, _password);
    // ignore: avoid_print
    print('LOGIN ok -> email=${user.email} extra=${user.extra}');
    expect(user.email, isNotEmpty);

    final me = await transport.me();
    expect(me, isNotNull, reason: '/auth/me returned null after login');
  });

  test('bc_sites_list returns rows for this token', () async {
    final sites = await bc.sitesList();
    // ignore: avoid_print
    print('SITES count=${sites.length}');
    for (final s in sites.take(20)) {
      // ignore: avoid_print
      print('  site ${s.siteId}  "${s.name}"');
    }
    // The whole point of the bug report: assert we actually SEE data.
    expect(
      sites,
      isNotEmpty,
      reason: 'bc_sites_list returned 0 rows for $_email at $_baseUrl — the '
          'token resolved to a tenant with no sites (or the wrong tenant).',
    );
  });

  test('bc_devices_list returns rows for this token', () async {
    final devices = await bc.devicesList();
    // ignore: avoid_print
    print('DEVICES count=${devices.length}');
    for (final d in devices.take(20)) {
      // ignore: avoid_print
      print('  device ${d.deviceId}  "${d.name ?? '-'}"  '
          'status=${d.status}  site=${d.siteId ?? '-'}');
    }
    expect(
      devices,
      isNotEmpty,
      reason: 'bc_devices_list returned 0 rows for $_email at $_baseUrl.',
    );
  });

  test('raw warehouse_query envelope shape is {count, rows, template}',
      () async {
    // Hit dispatch directly to prove the envelope the client parses matches the
    // agent — the exact spot a silent parse mismatch would zero out the list.
    final raw = await transport.dispatch<Map<String, dynamic>>(
      '${BcApi.extensionId}.warehouse_query',
      {
        'template': '${BcApi.extensionId}.bc_sites_list',
        'params': {'limit': 5},
      },
      fresh: true,
    );
    // ignore: avoid_print
    print('RAW envelope keys=${raw.keys.toList()} count=${raw['count']}');
    expect(raw.containsKey('rows'), isTrue,
        reason: 'envelope missing "rows" — BcApi._query reads res["rows"]');
    expect(raw['rows'], isA<List<dynamic>>());
  });
}
