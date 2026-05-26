import 'package:test/test.dart';
import 'package:rubix_api/rubix_api.dart';

// tests for TokenResponse
void main() {
  final instance = TokenResponseBuilder();
  // TODO add properties to the builder and call build()

  group(TokenResponse, () {
    // Absolute UTC expiry (RFC3339). Advisory in v1 — clients react to 401 rather than pre-emptively refreshing.
    // DateTime expiresAt
    test('to test the property `expiresAt`', () async {
      // TODO
    });

    // The plaintext bearer (`sak_<id>.<secret>`). Shown once; the server stores only the argon2id hash of the secret.
    // String token
    test('to test the property `token`', () async {
      // TODO
    });

    // Always `\"Bearer\"`. Reserved for the future refresh-token flow.
    // String tokenType
    test('to test the property `tokenType`', () async {
      // TODO
    });

  });
}
