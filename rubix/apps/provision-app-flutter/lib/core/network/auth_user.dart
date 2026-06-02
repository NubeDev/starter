import 'package:flutter/foundation.dart';

/// The signed-in principal. The agent's `/auth/me` returns at least an email;
/// other fields are kept in [extra] so the UI can show them without a rigid
/// schema (mirrors the React `AuthUser` index signature).
@immutable
class AuthUser {
  const AuthUser({required this.email, this.extra = const {}});

  final String email;
  final Map<String, dynamic> extra;

  factory AuthUser.fromJson(Map<String, dynamic> json) {
    // `/auth/me` may wrap the principal under `user`, or return it flat.
    final raw = json['user'] is Map<String, dynamic>
        ? json['user'] as Map<String, dynamic>
        : json;
    return AuthUser(
      email: (raw['email'] as String?) ?? 'Connected',
      extra: raw,
    );
  }
}
