/// Interface for platform-branched token storage.
abstract class TokenStore {
  /// Returns the stored token, or null if none.
  Future<String?> read();

  /// Stores a token with its expiry.
  Future<void> write(String token, {required DateTime expiresAt});

  /// Removes the stored token.
  Future<void> clear();
}
