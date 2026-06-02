import 'package:flutter/widgets.dart';
import 'package:provision_app/core/api/bc_types.dart';
import 'package:provision_app/core/theme/app_theme.dart';

/// Map a device status string → a status color token. Keeps the dot color
/// logic in one place for the list and detail views. `pending` shares the amber
/// (warning) token with `pairing`. Ported from the React `statusDot.ts`; the
/// hex literals line up exactly with the [RubixTokens] palette.
Color statusColor(String status) {
  switch (status) {
    case 'online':
    case 'provisioned':
    case 'active':
      return RubixTokens.primary; // 0xFF36E2C4
    case 'pairing':
    case 'pending':
      return RubixTokens.coral; // 0xFFFFC24B
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
