import 'package:test/test.dart';
import 'package:rubix_api/rubix_api.dart';

// tests for TenantRequiredResponse
void main() {
  final instance = TenantRequiredResponseBuilder();
  // TODO add properties to the builder and call build()

  group(TenantRequiredResponse, () {
    // Always `\"tenant_required\"`. Discriminator string.
    // String error
    test('to test the property `error`', () async {
      // TODO
    });

    // One entry per membership row for the authenticated user.
    // BuiltList<TenantMembershipEntry> memberships
    test('to test the property `memberships`', () async {
      // TODO
    });

  });
}
