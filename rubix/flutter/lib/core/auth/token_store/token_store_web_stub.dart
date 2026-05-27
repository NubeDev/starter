import 'package:rubix_flutter/core/auth/token_store/token_store.dart';

/// VM-side stub for [WebTokenStore]. Selected by the conditional
/// import in `token_store_providers.dart` when `dart:js_interop` is
/// not available (native, unit tests on the Flutter VM, etc).
///
/// Constructing one is a programmer error — `kIsWeb` would be false
/// in those contexts, so the providers should always pick the mobile
/// store. The throws are tripwires for that bug.
class WebTokenStore implements TokenStore {
  WebTokenStore() {
    throw StateError(
      'WebTokenStore stub instantiated outside the web build — '
      'check the kIsWeb branch in token_store_providers.dart',
    );
  }

  @override
  Future<String?> read() => throw UnimplementedError();

  @override
  Future<void> write(String token, {required DateTime expiresAt}) =>
      throw UnimplementedError();

  @override
  Future<void> clear() => throw UnimplementedError();
}
