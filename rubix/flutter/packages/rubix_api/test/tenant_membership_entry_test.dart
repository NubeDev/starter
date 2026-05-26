import 'package:test/test.dart';
import 'package:rubix_api/rubix_api.dart';

// tests for TenantMembershipEntry
void main() {
  final instance = TenantMembershipEntryBuilder();
  // TODO add properties to the builder and call build()

  group(TenantMembershipEntry, () {
    // User's role within that tenant (`reader | writer | admin`).
    // String role
    test('to test the property `role`', () async {
      // TODO
    });

    // Tenant id to echo back in `TokenRequest.tenant_id`.
    // String tenantId
    test('to test the property `tenantId`', () async {
      // TODO
    });

  });
}
