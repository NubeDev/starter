// The (barcode, place, trend, alarm) positional signature mirrors the React
// `buildProvisionInput` exactly — keep it positional for call-site parity.
// ignore_for_file: avoid_positional_boolean_parameters

import 'package:provision_app/core/api/bc_types.dart';
import 'package:provision_app/features/place/domain/placement.dart';

/// Translate the UI placement + toggles into the `bc_provision` payload. New
/// location/page names become `new_*` objects; chosen ids pass straight
/// through. Ported from the React `buildProvisionInput.ts`.
ProvisionInput buildProvisionInput(
  String barcode,
  Placement place,
  bool trend,
  bool alarm, {
  String? name,
}) {
  return ProvisionInput(
    barcode: barcode,
    siteId: place.siteId,
    trend: trend,
    alarm: alarm,
    name: name,
    locationId: place.locationId.isNotEmpty ? place.locationId : null,
    newLocation: place.locationId.isEmpty && place.newLocation.trim().isNotEmpty
        ? place.newLocation.trim()
        : null,
    pageId: place.pageId.isNotEmpty ? place.pageId : null,
    newPage: place.pageId.isEmpty && place.newPage.trim().isNotEmpty
        ? place.newPage.trim()
        : null,
  );
}
