import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:provision_app/core/api/bc_api.dart';
import 'package:provision_app/core/api/bc_types.dart';
import 'package:provision_app/core/api/refresh.dart';

/// One row in the Home "Recent" list — a recently provisioned device with its
/// site/location resolved to display names (the device row only carries IDs).
class RecentDevice {
  const RecentDevice({
    required this.device,
    required this.placeLabel,
    required this.provisionedAt,
  });

  final DeviceRow device;

  /// "Da Nang · Roof" — site · location, or just the site, or '' if neither
  /// resolves to a name.
  final String placeLabel;

  /// Raw ISO `provisioned_at`, or null. Rendered via `relativeTimeLabel`.
  final String? provisionedAt;
}

/// Everything the Home screen needs in one shot. Counts are the LENGTH of the
/// full list fetches below (there is no dedicated count endpoint on the agent),
/// so each count is one list round-trip — cheap for the small inventories this
/// app targets, but a true `count(*)` would be leaner at scale.
class HomeData {
  const HomeData({
    required this.deviceCount,
    required this.siteCount,
    required this.templateCount,
    required this.recent,
  });

  final int deviceCount;
  final int siteCount;
  final int templateCount;
  final List<RecentDevice> recent;
}

/// Loads Home's counts + recent devices. Re-runs whenever the shared refresh
/// signal bumps (same pattern as the list screens). The recent list is the real
/// `devices_list` sorted by `provisioned_at` desc — NOT a synthetic feed.
final homeDataProvider = FutureProvider.autoDispose<HomeData>((ref) async {
  ref.watch(refreshProvider);
  final bc = ref.read(bcApiProvider);

  // Fetch in parallel — counts come from list lengths; sites/locations also
  // feed the name resolution for recent rows.
  final results = await Future.wait([
    bc.devicesList(),
    bc.sitesList(),
    bc.templatesList(),
  ]);
  final devices = results[0] as List<DeviceRow>;
  final sites = results[1] as List<SiteRow>;
  final templates = results[2] as List<TemplateRow>;

  // Resolve location names lazily — only for the sites that actually appear in
  // the recent devices, to avoid fetching every site's locations.
  final siteName = {for (final s in sites) s.siteId: s.name};

  // Recent = devices with a provisioned_at, newest first, top 3.
  final dated = devices
      .where((d) => (d.provisionedAt ?? '').isNotEmpty)
      .toList()
    ..sort((a, b) => (b.provisionedAt ?? '').compareTo(a.provisionedAt ?? ''));
  final top = dated.take(3).toList();

  // Look up location names for just the sites referenced by the top rows.
  final neededSites = {
    for (final d in top)
      if ((d.siteId ?? '').isNotEmpty) d.siteId!,
  };
  final locName = <String, String>{};
  await Future.wait([
    for (final siteId in neededSites)
      bc.locationsList(siteId: siteId).then((locs) {
        for (final l in locs) {
          locName[l.locationId] = l.name;
        }
      }).catchError((_) => null),
  ]);

  String placeLabel(DeviceRow d) {
    final site = siteName[d.siteId];
    final loc = locName[d.locationId];
    return [
      if (site != null && site.isNotEmpty) site,
      if (loc != null && loc.isNotEmpty) loc,
    ].join(' · ');
  }

  return HomeData(
    deviceCount: devices.length,
    siteCount: sites.length,
    templateCount: templates.length,
    recent: [
      for (final d in top)
        RecentDevice(
          device: d,
          placeLabel: placeLabel(d),
          provisionedAt: d.provisionedAt,
        ),
    ],
  );
});
