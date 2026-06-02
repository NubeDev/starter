import 'package:flutter/material.dart';

/// Device/connection STATUS — the repurposed "mood" layer. A live status tints
/// the app accent on top of the chosen theme, so the whole UI reflects the
/// agent connection state at a glance. null status = use the theme accent.
/// Ported from the React app's `theme/statuses.ts`.
enum StatusKey { online, pairing, fault, offline }

@immutable
class AppStatus {
  const AppStatus({required this.key, required this.label, required this.accent});
  final StatusKey key;
  final String label;
  final Color accent;
}

const statuses = <StatusKey, AppStatus>{
  StatusKey.online:
      AppStatus(key: StatusKey.online, label: 'Connected', accent: Color(0xFF36E2C4)),
  StatusKey.pairing:
      AppStatus(key: StatusKey.pairing, label: 'Provisioning', accent: Color(0xFFFFC24B)),
  StatusKey.fault:
      AppStatus(key: StatusKey.fault, label: 'Fault', accent: Color(0xFFFF5A52)),
  StatusKey.offline:
      AppStatus(key: StatusKey.offline, label: 'Offline', accent: Color(0xFF7C8A8A)),
};
