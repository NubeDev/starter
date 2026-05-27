import 'dart:convert';

import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

import 'package:rubix_flutter/features/connections/data/connection_credentials_store_web_stub.dart'
    if (dart.library.js_interop)
        'package:rubix_flutter/features/connections/data/connection_credentials_store_web.dart';

class ConnectionCredentials {
  const ConnectionCredentials({required this.email, required this.password});
  final String email;
  final String password;
}

/// Per-connection credentials. Web uses plain localStorage (matches
/// the token store split); native uses flutter_secure_storage.
///
/// The web fork is deliberate — `flutter_secure_storage`'s web
/// fallback encrypts under a key it also stores in browser storage,
/// and any drift between the two (incognito reload, partial cache
/// clear, version bump) returns null on read even though the user
/// did save creds. localStorage round-trips reliably, which is what
/// this internal-tool flow needs.
abstract class ConnectionCredentialsStore {
  Future<ConnectionCredentials?> read(int id);
  Future<void> write(int id, ConnectionCredentials creds);
  Future<void> delete(int id);
}

String credsKey(int id) => 'conn_creds_$id';

ConnectionCredentials credsFromJson(String raw) {
  final json = jsonDecode(raw) as Map<String, dynamic>;
  return ConnectionCredentials(
    email: json['email'] as String,
    password: json['password'] as String,
  );
}

String credsToJson(ConnectionCredentials creds) =>
    jsonEncode({'email': creds.email, 'password': creds.password});

class MobileConnectionCredentialsStore implements ConnectionCredentialsStore {
  MobileConnectionCredentialsStore(this._storage);

  final FlutterSecureStorage _storage;

  @override
  Future<ConnectionCredentials?> read(int id) async {
    final raw = await _storage.read(key: credsKey(id));
    if (raw == null) return null;
    return credsFromJson(raw);
  }

  @override
  Future<void> write(int id, ConnectionCredentials creds) =>
      _storage.write(key: credsKey(id), value: credsToJson(creds));

  @override
  Future<void> delete(int id) => _storage.delete(key: credsKey(id));
}

final connectionCredentialsStoreProvider =
    Provider<ConnectionCredentialsStore>((ref) {
  if (kIsWeb) {
    return WebConnectionCredentialsStore();
  }
  return MobileConnectionCredentialsStore(const FlutterSecureStorage());
});
