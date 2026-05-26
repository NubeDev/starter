import 'package:test/test.dart';
import 'package:rubix_api/rubix_api.dart';

// tests for TokenRequest
void main() {
  final instance = TokenRequestBuilder();
  // TODO add properties to the builder and call build()

  group(TokenRequest, () {
    // User's email — same identifier as `POST /auth/login`.
    // String email
    test('to test the property `email`', () async {
      // TODO
    });

    // Plaintext password.
    // String password
    test('to test the property `password`', () async {
      // TODO
    });

    // Optional tenant binding. When omitted, the route resolves the tenant from the user's memberships (requires [`AuthState::with_tenants`]). See design doc §payload.
    // String tenantId
    test('to test the property `tenantId`', () async {
      // TODO
    });

  });
}
