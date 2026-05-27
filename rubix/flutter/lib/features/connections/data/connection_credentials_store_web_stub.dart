import 'package:rubix_flutter/features/connections/data/connection_credentials_store.dart';

/// VM-side stub. The provider guards with kIsWeb so this is never
/// constructed off-web — the throws are tripwires for that bug.
class WebConnectionCredentialsStore implements ConnectionCredentialsStore {
  WebConnectionCredentialsStore() {
    throw StateError(
      'WebConnectionCredentialsStore stub instantiated outside the web '
      'build — check the kIsWeb branch in '
      'connection_credentials_store.dart',
    );
  }

  @override
  Future<ConnectionCredentials?> read(int id) => throw UnimplementedError();

  @override
  Future<void> write(int id, ConnectionCredentials creds) =>
      throw UnimplementedError();

  @override
  Future<void> delete(int id) => throw UnimplementedError();
}
