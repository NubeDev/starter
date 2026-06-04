import 'package:provision_app/core/api/bc_api.dart';
import 'package:provision_app/core/api/bc_types.dart';
import 'package:provision_app/core/network/transport.dart';

/// TEMPORARY demo data source — `--dart-define=DNA_DEMO=true`. Lets the gated
/// screens be previewed without a live agent. DELETE before merging.
class DemoBcApi extends BcApi {
  DemoBcApi(RubixTransport transport) : super(transport, () {});

  @override
  Future<List<DeviceRow>> devicesList({String? siteId, String? status, int? limit}) async => const [
        DeviceRow(deviceId: 'aidan', template: 'lora_droplet_sensor', name: 'aidan', network: 'lora', status: 'pending', siteId: 'site_dn', locationId: 'loc_roof', provisionedAt: '2026-06-04T11:36:00Z'),
        DeviceRow(deviceId: 'roof-temp-02', template: 'lora_droplet_sensor', name: 'roof-temp-02', network: 'lora', status: 'placed', pageId: 'page_roof', siteId: 'site_dn', locationId: 'loc_roof', provisionedAt: '2026-06-04T10:40:00Z'),
        DeviceRow(deviceId: 'lobby-rh', template: 'lora_droplet_sensor', name: 'lobby-rh', network: 'lora', status: 'placed', pageId: 'page_lobby', siteId: 'site_hq', locationId: 'loc_lobby', provisionedAt: '2026-06-04T08:40:00Z'),
      ];

  @override
  Future<List<SiteRow>> sitesList({int limit = 200}) async => const [
        SiteRow(siteId: 'site_dn', name: 'Da Nang'),
        SiteRow(siteId: 'site_hq', name: 'HQ'),
        SiteRow(siteId: 'site_x', name: 'Plant 3'),
      ];

  @override
  Future<List<LocationRow>> locationsList({String? siteId, int? limit}) async =>
      siteId == 'site_hq'
          ? const [LocationRow(locationId: 'loc_lobby', siteId: 'site_hq', name: 'Lobby')]
          : const [LocationRow(locationId: 'loc_roof', siteId: 'site_dn', name: 'Roof')];

  @override
  Future<List<TemplateRow>> templatesList({int limit = 200}) async => const [
        TemplateRow(template: 'lora_droplet_sensor', version: '1', displayName: 'LoRa Droplet Sensor', network: 'lora', category: 'sensor'),
        TemplateRow(template: 'lora_electrical_optical', version: '1', displayName: 'LoRa Electrical Optical Sensor', network: 'lora', category: 'sensor'),
      ];

  @override
  Future<List<PointRow>> pointsByDevice(String deviceId) async => const [
        PointRow(pointId: 'p_batt', name: 'Battery', widget: 'battery', unit: '%', trendOn: true, alarmOn: true),
      ];
}
