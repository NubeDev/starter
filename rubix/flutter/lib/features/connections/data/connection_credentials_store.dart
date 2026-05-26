import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

class ConnectionCredentials {
  const ConnectionCredentials({required this.email, required this.password});
  final String email;
  final String password;
}

/// Per-connection credentials in flutter_secure_storage.
///
/// Each connection's creds live under key `conn_creds_<id>` as JSON.
/// On web the package falls back to localStorage; for an internal tool
/// that is acceptable — the alternative is making the user retype
/// credentials on every page reload.
class ConnectionCredentialsStore {
  ConnectionCredentialsStore(this._storage);

  final FlutterSecureStorage _storage;

  static String _key(int id) => 'conn_creds_$id';

  Future<ConnectionCredentials?> read(int id) async {
    final raw = await _storage.read(key: _key(id));
    if (raw == null) return null;
    final json = jsonDecode(raw) as Map<String, dynamic>;
    return ConnectionCredentials(
      email: json['email'] as String,
      password: json['password'] as String,
    );
  }

  Future<void> write(int id, ConnectionCredentials creds) async {
    await _storage.write(
      key: _key(id),
      value: jsonEncode({
        'email': creds.email,
        'password': creds.password,
      }),
    );
  }

  Future<void> delete(int id) => _storage.delete(key: _key(id));
}

final connectionCredentialsStoreProvider =
    Provider<ConnectionCredentialsStore>((ref) {
  return ConnectionCredentialsStore(const FlutterSecureStorage());
});
