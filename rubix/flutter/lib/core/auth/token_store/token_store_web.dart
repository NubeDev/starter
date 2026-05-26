import 'package:rubix_flutter/core/auth/token_store/token_store.dart';

/// Web token store — in-memory only. Token is lost on tab reload by design.
class WebTokenStore implements TokenStore {
  String? _token;

  @override
  Future<String?> read() async => _token;

  @override
  Future<void> write(String token, {required DateTime expiresAt}) async {
    _token = token;
  }

  @override
  Future<void> clear() async {
    _token = null;
  }
}
