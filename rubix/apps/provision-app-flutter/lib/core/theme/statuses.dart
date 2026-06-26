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
  // DNA kit semantic dots: connected=green, warning=amber, offline=red,
  // idle=grey.
  StatusKey.online:
      AppStatus(key: StatusKey.online, label: 'Connected', accent: Color(0xFF5DCAA5)),
  StatusKey.pairing:
      AppStatus(key: StatusKey.pairing, label: 'Provisioning', accent: Color(0xFFC9A24A)),
  StatusKey.fault:
      AppStatus(key: StatusKey.fault, label: 'Offline', accent: Color(0xFFE24B4A)),
  StatusKey.offline:
      AppStatus(key: StatusKey.offline, label: 'Idle', accent: Color(0xFF7E8888)),
};
