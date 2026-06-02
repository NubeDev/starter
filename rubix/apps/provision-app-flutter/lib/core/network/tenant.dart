import 'package:flutter/foundation.dart';

/// One tenant/org the signed-in user may pick at login. Returned by the agent's
/// `/auth/token` 409 `tenant_required` response when the account belongs to more
/// than one org. `tenantId` is echoed back as `TokenRequest.tenant_id` on retry.
@immutable
class TenantMembership {
  const TenantMembership({required this.tenantId, required this.role});

  final String tenantId;
  final String role;

  factory TenantMembership.fromJson(Map<String, dynamic> json) =>
      TenantMembership(
        tenantId: (json['tenant_id'] as String?) ?? '',
        role: (json['role'] as String?) ?? '',
      );
}

/// The super-admin sentinel — an Admin can pass this as the tenant to see every
/// org's resources at once (matches the backend's `SUPER_ADMIN_TENANT`).
const superAdminTenant = '*';

/// Thrown by the transport when `/auth/token` returns 409 `tenant_required`:
/// the user belongs to multiple orgs and must pick one. The Connect screen
/// catches this, shows an org picker, then retries login with the chosen
/// `tenant_id`.
class TenantRequiredException implements Exception {
  const TenantRequiredException(this.memberships);
  final List<TenantMembership> memberships;

  @override
  String toString() => 'tenant_required';
}
