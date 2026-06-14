import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// Persists the agent base URL (non-sensitive → shared_preferences) and the
/// `sak_` Bearer token (sensitive → secure storage / keychain). Replaces the
/// React webTransport's localStorage seeds (`rbx.provision.baseUrl` /
/// `rbx.provision.token`), with the token upgraded to the platform keychain
/// since Flutter has one.
class CredentialStore {
  CredentialStore([FlutterSecureStorage? secure])
      : _secure = secure ?? const FlutterSecureStorage();

  static const _baseKey = 'rbx.provision.baseUrl';
  static const _tokenKey = 'rbx.provision.token';

  final FlutterSecureStorage _secure;

  Future<String> readBaseUrl() async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getString(_baseKey) ?? '';
  }

  Future<void> writeBaseUrl(String value) async {
    final prefs = await SharedPreferences.getInstance();
    if (value.isEmpty) {
      await prefs.remove(_baseKey);
    } else {
      await prefs.setString(_baseKey, value);
    }
  }

  Future<String> readToken() async => await _secure.read(key: _tokenKey) ?? '';

  Future<void> writeToken(String value) async {
    if (value.isEmpty) {
      await _secure.delete(key: _tokenKey);
    } else {
      await _secure.write(key: _tokenKey, value: value);
    }
  }
}
