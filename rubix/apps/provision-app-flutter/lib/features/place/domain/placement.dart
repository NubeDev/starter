import 'package:flutter/foundation.dart';

/// The placement choice shared by the Scan flow and the Wizard. Mirrors the
/// React `placement.ts` (the extension's Placement shape): Site -> Location ->
/// Page (scoped to the chosen site).
///
/// A placement is provisionable once a SITE is chosen (`ready`). A page is
/// optional — a device with a site but no page is commissioned as `pending`
/// and can be placed on a page later from Devices.
@immutable
class Placement {
  const Placement({
    this.siteId = '',
    this.locationId = '',
    this.newLocation = '',
    this.pageId = '',
    this.newPage = '',
  });

  final String siteId;
  final String locationId;
  final String newLocation;
  final String pageId;
  final String newPage;

  /// The all-empty placement (the React `EMPTY_PLACEMENT`).
  static const empty = Placement();

  Placement copyWith({
    String? siteId,
    String? locationId,
    String? newLocation,
    String? pageId,
    String? newPage,
  }) {
    return Placement(
      siteId: siteId ?? this.siteId,
      locationId: locationId ?? this.locationId,
      newLocation: newLocation ?? this.newLocation,
      pageId: pageId ?? this.pageId,
      newPage: newPage ?? this.newPage,
    );
  }

  /// Provisionable once a site is chosen.
  bool get ready => siteId.isNotEmpty;

  /// Whether this placement also lands the device on a page (existing or new).
  bool get hasPage => pageId.isNotEmpty || newPage.trim().isNotEmpty;

  @override
  bool operator ==(Object other) =>
      other is Placement &&
      other.siteId == siteId &&
      other.locationId == locationId &&
      other.newLocation == newLocation &&
      other.pageId == pageId &&
      other.newPage == newPage;

  @override
  int get hashCode =>
      Object.hash(siteId, locationId, newLocation, pageId, newPage);
}
