import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:rubix_flutter/core/auth/token_store/token_store.dart';

/// Mobile token store backed by Keychain (iOS) / Keystore (Android).
class MobileTokenStore implements TokenStore {
  MobileTokenStore(this._storage);

  final FlutterSecureStorage _storage;

  static const _keyToken = 'rubix_auth_token';
  static const _keyExpiresAt = 'rubix_auth_expires_at';

  @override
  Future<String?> read() => _storage.read(key: _keyToken);

  @override
  Future<void> write(String token, {required DateTime expiresAt}) async {
    await _storage.write(key: _keyToken, value: token);
    await _storage.write(
      key: _keyExpiresAt,
      value: expiresAt.toIso8601String(),
    );
  }

  @override
  Future<void> clear() async {
    await _storage.delete(key: _keyToken);
    await _storage.delete(key: _keyExpiresAt);
  }
}
