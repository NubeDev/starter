import 'package:web/web.dart' as web;
import 'package:rubix_flutter/core/auth/token_store/token_store.dart';

/// Web token store backed by `localStorage`.
///
/// Persists across tab reloads. Expiry is stored alongside the token
/// so the Dio refresh interceptor can renew proactively instead of
/// waiting for a 401 from the server.
class WebTokenStore implements TokenStore {
  static const _keyToken = 'rubix_auth_token';
  static const _keyExpiresAt = 'rubix_auth_expires_at';

  @override
  Future<String?> read() async {
    return web.window.localStorage.getItem(_keyToken);
  }

  @override
  Future<void> write(String token, {required DateTime expiresAt}) async {
    web.window.localStorage.setItem(_keyToken, token);
    web.window.localStorage.setItem(
      _keyExpiresAt,
      expiresAt.toIso8601String(),
    );
  }

  @override
  Future<void> clear() async {
    web.window.localStorage.removeItem(_keyToken);
    web.window.localStorage.removeItem(_keyExpiresAt);
  }
}
