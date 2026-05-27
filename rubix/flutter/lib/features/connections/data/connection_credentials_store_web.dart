import 'package:rubix_flutter/features/connections/data/connection_credentials_store.dart';
import 'package:web/web.dart' as web;

class WebConnectionCredentialsStore implements ConnectionCredentialsStore {
  @override
  Future<ConnectionCredentials?> read(int id) async {
    final raw = web.window.localStorage.getItem(credsKey(id));
    if (raw == null) return null;
    return credsFromJson(raw);
  }

  @override
  Future<void> write(int id, ConnectionCredentials creds) async {
    web.window.localStorage.setItem(credsKey(id), credsToJson(creds));
  }

  @override
  Future<void> delete(int id) async {
    web.window.localStorage.removeItem(credsKey(id));
  }
}
