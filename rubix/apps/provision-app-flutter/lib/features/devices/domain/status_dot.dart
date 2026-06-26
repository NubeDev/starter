import 'package:flutter/widgets.dart';
import 'package:provision_app/core/api/bc_types.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/look.dart';

/// Map a device status string → a status color. Keeps the dot color logic in
/// one place for the list and detail views. `pending` shares the secondary
/// accent with `pairing`. The "live" and "pending" colors track the active
/// theme via [look]; `fault`/`offline` are fixed status semantics. Ported from
/// the React `statusDot.ts`.
Color statusColor(String status, Look look) {
  switch (status) {
    case 'online':
    case 'provisioned':
    case 'active':
      return look.accent;
    case 'pairing':
    case 'pending':
      return look.accent2;
    case 'fault':
    case 'error':
      return RubixTokens.fault; // 0xFFFF5A52
    default:
      return RubixTokens.offline; // 0xFF7C8A8A
  }
}

/// A device is placeable on a page when it's commissioned but not yet on one —
/// status `pending` or simply missing a page_id.
bool isPlaceable(DeviceRow d) => d.status == 'pending' || d.pageId == null;
