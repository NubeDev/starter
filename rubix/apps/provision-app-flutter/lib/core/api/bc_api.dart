import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:provision_app/core/network/network_providers.dart';
import 'package:provision_app/core/network/transport.dart';
import 'package:provision_app/core/api/bc_types.dart';
import 'package:provision_app/core/api/refresh.dart';

/// Typed, named wrappers over the `bc_*` tools — the Dart port of the React
/// app's `api/bc.ts`. Each call forwards to [RubixTransport.dispatch] so callers
/// never hand-spell tool ids. Reads go through `warehouse_query` (always fresh);
/// mutations bump the shared refresh signal so sibling list views re-fetch.
class BcApi {
  BcApi(this._transport, this._bumpRefresh);

  final RubixTransport _transport;
  final void Function() _bumpRefresh;

  /// The owning extension id — tool ids are `${EXTENSION_ID}.${name}`.
  static const extensionId = 'com.nubeio.rubixos';
  String _tool(String name) => '$extensionId.$name';

  /// Every write goes through here so the shared refresh signal bumps the moment
  /// the server confirms.
  Future<T> _mutate<T>(
    String toolId,
    Object? params,
    T Function(Map<String, dynamic>) parse,
  ) async {
    final res =
        await _transport.dispatch<Map<String, dynamic>>(toolId, params, fresh: true);
    _bumpRefresh();
    return parse(res);
  }

  /// A list read: `warehouse_query` envelope, always fresh.
  Future<List<R>> _query<R>(
    String template,
    R Function(Map<String, dynamic>) parse, {
    Map<String, dynamic> params = const {},
  }) async {
    final res = await _transport.dispatch<Map<String, dynamic>>(
      _tool('warehouse_query'),
      {'template': template, 'params': params},
      fresh: true,
    );
    final rows = (res['rows'] as List?) ?? const [];
    return rows
        .cast<Map<String, dynamic>>()
        .map(parse)
        .toList(growable: false);
  }

  // ── mutations ────────────────────────────────────────────────────────────

  /// read-only — no refresh bump
  Future<ScannedIdentity> decode(String barcode) async {
    final res = await _transport.dispatch<Map<String, dynamic>>(
      _tool('bc_decode'),
      {'barcode': barcode},
    );
    return ScannedIdentity.fromJson(res);
  }

  Future<ProvisionResult> provision(ProvisionInput input) =>
      _mutate(_tool('bc_provision'), input.toJson(), ProvisionResult.fromJson);

  /// Assign a (pending) device to a page — existing pageId or a newPage name.
  Future<AssignPageResult> assignPage({
    required String deviceId,
    String? pageId,
    String? newPage,
  }) {
    final params = <String, dynamic>{
      'device_id': deviceId,
      if (pageId != null) 'page_id': pageId,
      if (newPage != null) 'new_page': {'name': newPage},
    };
    return _mutate(
      _tool('bc_device_assign_page'),
      params,
      AssignPageResult.fromJson,
    );
  }

  Future<void> deviceUpdate(Map<String, dynamic> row) async {
    await _mutate(_tool('bc_device_update'), {'row': row}, (j) => j);
  }

  Future<void> decommission(List<String> deviceIds, {bool hard = false}) async {
    await _mutate(
      _tool('bc_device_decommission'),
      {'device_ids': deviceIds, 'hard': hard},
      (j) => j,
    );
  }

  Future<void> siteCreate({required String siteId, required String name}) async {
    await _mutate(
      _tool('bc_site_create'),
      {'row': {'site_id': siteId, 'name': name}},
      (j) => j,
    );
  }

  Future<void> locationCreate({
    required String locationId,
    required String siteId,
    required String name,
  }) async {
    await _mutate(
      _tool('bc_location_create'),
      {'row': {'location_id': locationId, 'site_id': siteId, 'name': name}},
      (j) => j,
    );
  }

  Future<void> pageCreate({
    required String pageId,
    required String siteId,
    required String name,
  }) async {
    await _mutate(
      _tool('bc_page_create'),
      {'row': {'page_id': pageId, 'site_id': siteId, 'name': name}},
      (j) => j,
    );
  }

  Future<void> templateUpsert(String yaml) async {
    await _mutate(_tool('bc_template_upsert'), {'yaml': yaml}, (j) => j);
  }

  Future<LabelRender> labelRender(String deviceId) async {
    final res = await _transport.dispatch<Map<String, dynamic>>(
      _tool('bc_label_render'),
      {'device_id': deviceId},
    );
    return LabelRender.fromJson(res);
  }

  // ── reads ──────────────────────────────────────────────────────────────

  Future<List<DeviceRow>> devicesList({
    String? siteId,
    String? status,
    int? limit,
  }) =>
      _query(_tool('bc_devices_list'), DeviceRow.fromJson, params: {
        if (siteId != null) 'site_id': siteId,
        if (status != null) 'status': status,
        if (limit != null) 'limit': limit,
      });

  Future<List<SiteRow>> sitesList({int limit = 200}) =>
      _query(_tool('bc_sites_list'), SiteRow.fromJson, params: {'limit': limit});

  Future<List<LocationRow>> locationsList({String? siteId, int? limit}) =>
      _query(_tool('bc_locations_list'), LocationRow.fromJson, params: {
        if (siteId != null) 'site_id': siteId,
        if (limit != null) 'limit': limit,
      });

  Future<List<PageRow>> pagesList({String? siteId, int limit = 200}) =>
      _query(_tool('bc_pages_list'), PageRow.fromJson, params: {
        if (siteId != null) 'site_id': siteId,
        'limit': limit,
      });

  Future<List<TemplateRow>> templatesList({int limit = 200}) => _query(
        _tool('bc_templates_list'),
        TemplateRow.fromJson,
        params: {'limit': limit},
      );

  Future<List<TemplateYaml>> templateYaml(String template) => _query(
        _tool('bc_template_yaml'),
        TemplateYaml.fromJson,
        params: {'template': template},
      );

  Future<List<PointRow>> pointsByDevice(String deviceId) => _query(
        _tool('bc_points_by_device'),
        PointRow.fromJson,
        params: {'device_id': deviceId},
      );

  Future<List<WidgetRow>> widgetsByPage(String pageId) => _query(
        _tool('bc_widgets_by_page'),
        WidgetRow.fromJson,
        params: {'page_id': pageId},
      );
}

/// The app-wide [BcApi], wired to the transport and the refresh signal.
final bcApiProvider = Provider<BcApi>((ref) {
  final transport = ref.watch(transportProvider);
  return BcApi(transport, () => ref.read(refreshProvider.notifier).bump());
});
