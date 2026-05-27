/// Picks the right data-store implementation at startup:
/// - native/desktop/mobile → talks to Drift directly (in-process).
/// - web → talks to the `rubix_server` shelf process over REST,
///   because the browser can't open the on-disk SQLite file.
library;

import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:rubix_data/rubix_data.dart';
import 'package:rubix_flutter/app.dart';
import 'package:rubix_flutter/core/storage/daos/connection_dao.dart';
import 'package:rubix_flutter/core/storage/daos/settings_dao.dart';
import 'package:rubix_flutter/features/connections/data/local_connections_repository.dart';
import 'package:rubix_flutter/features/connections/data/rest_connections_repository.dart';
import 'package:rubix_flutter/features/settings/data/local_settings_repository.dart';
import 'package:rubix_flutter/features/settings/data/rest_settings_repository.dart';

/// Base URL of the local Dart REST backend used by the web build.
/// Override with `--dart-define=RUBIX_SERVER_URL=...`.
const String _webServerUrl = String.fromEnvironment(
  'RUBIX_SERVER_URL',
  defaultValue: 'http://localhost:8787',
);

/// Shared-secret token the server requires on every `/api/*` call.
/// The Makefile reads `~/.rubix/server.token` and injects the same
/// value into both processes via `--dart-define`. Defaults to empty
/// so the app fails loudly at boot if the user forgot the flag.
const String _webServerToken = String.fromEnvironment(
  'RUBIX_SERVER_TOKEN',
  defaultValue: '',
);

/// Single shared Dio for the web data-layer — same base URL and auth
/// for both connections + settings impls, so the bearer-token
/// interceptor only attaches once.
Dio _buildWebDio() {
  final dio = Dio(BaseOptions(baseUrl: _webServerUrl));
  if (_webServerToken.isNotEmpty) {
    dio.options.headers['Authorization'] = 'Bearer $_webServerToken';
  }
  return dio;
}

final connectionsRepositoryProvider = Provider<ConnectionsRepository>((ref) {
  if (kIsWeb) {
    return RestConnectionsRepository(baseUrl: _webServerUrl, dio: _buildWebDio());
  }
  return LocalConnectionsRepository(
    ConnectionDao(ref.watch(appDatabaseProvider)),
  );
});

final settingsRepositoryProvider = Provider<SettingsRepository>((ref) {
  if (kIsWeb) {
    return RestSettingsRepository(baseUrl: _webServerUrl, dio: _buildWebDio());
  }
  return LocalSettingsRepository(SettingsDao(ref.watch(appDatabaseProvider)));
});
