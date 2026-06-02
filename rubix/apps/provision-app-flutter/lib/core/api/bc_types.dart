import 'package:freezed_annotation/freezed_annotation.dart';

part 'bc_types.freezed.dart';
part 'bc_types.g.dart';

/// Domain row/result shapes for the barcode provisioning feature. Ported from
/// the React app's `api/bc-types.ts`, which itself mirrors the extension's
/// `ui-src/provision/bc-types.ts` so the request/response contracts stay in
/// lockstep with the backend `bc_*` tools.
///
/// Backend rows are loosely typed (numbers arrive as `String | number`, many
/// fields are nullable). The JSON helpers below coerce defensively so a stray
/// string-encoded number never crashes the parse.

/// A widget enum from a template point — the curated renderer catalog key.
/// Kept as a String (not an enum) because the backend may add kinds the client
/// doesn't know; the renderer switchboard falls back to `stat`.
typedef WidgetKind = String;

@freezed
abstract class TemplatePoint with _$TemplatePoint {
  const factory TemplatePoint({
    required String key,
    required String name,
    required String widget,
  }) = _TemplatePoint;

  factory TemplatePoint.fromJson(Map<String, dynamic> json) =>
      _$TemplatePointFromJson(json);
}

@freezed
abstract class ScannedTemplate with _$ScannedTemplate {
  const factory ScannedTemplate({
    @JsonKey(name: 'display_name') required String displayName,
    required String icon,
    required String category,
    @Default(<TemplatePoint>[]) List<TemplatePoint> points,
    @JsonKey(name: 'widget_group') @Default('') String widgetGroup,
  }) = _ScannedTemplate;

  factory ScannedTemplate.fromJson(Map<String, dynamic> json) =>
      _$ScannedTemplateFromJson(json);
}

/// Result of `bc_decode`.
@freezed
abstract class ScannedIdentity with _$ScannedIdentity {
  const factory ScannedIdentity({
    required String id,
    required String model,
    required String network,
    @Default('') String address,
    @JsonKey(name: 'default_ip') @Default('') String defaultIp,
    @Default('') String hw,
    required ScannedTemplate template,
    @JsonKey(name: 'known_models') @Default(<String>[]) List<String> knownModels,
  }) = _ScannedIdentity;

  factory ScannedIdentity.fromJson(Map<String, dynamic> json) =>
      _$ScannedIdentityFromJson(json);
}

/// Input to `bc_provision`. Hand-rolled toJson (the wire shape uses `new_*`
/// objects and drops nulls) — see `buildProvisionInput`.
@immutable
class ProvisionInput {
  const ProvisionInput({
    required this.barcode,
    this.siteId,
    this.locationId,
    this.newLocation,
    this.pageId,
    this.newPage,
    this.name,
    this.trend,
    this.alarm,
  });

  final String barcode;
  final String? siteId;
  final String? locationId;
  final String? newLocation;
  final String? pageId;
  final String? newPage;
  final String? name;
  final bool? trend;
  final bool? alarm;

  Map<String, dynamic> toJson() => {
        'barcode': barcode,
        if (siteId != null) 'site_id': siteId,
        if (trend != null) 'trend': trend,
        if (alarm != null) 'alarm': alarm,
        if (name != null) 'name': name,
        if (locationId != null) 'location_id': locationId,
        if (newLocation != null) 'new_location': {'name': newLocation},
        if (pageId != null) 'page_id': pageId,
        if (newPage != null) 'new_page': {'name': newPage},
      };
}

/// Result of `bc_provision`.
@freezed
abstract class ProvisionResult with _$ProvisionResult {
  const factory ProvisionResult({
    @JsonKey(name: 'device_id') required String deviceId,
    @Default(0) int points,
    @Default(0) int widgets,
    @Default(0) int alarms,
    @JsonKey(name: 'page_id') @Default('') String pageId,
    @Default(<String>[]) List<String> warnings,
  }) = _ProvisionResult;

  factory ProvisionResult.fromJson(Map<String, dynamic> json) =>
      _$ProvisionResultFromJson(json);
}

/// Result of `bc_device_assign_page`.
@freezed
abstract class AssignPageResult with _$AssignPageResult {
  const factory AssignPageResult({
    @JsonKey(name: 'device_id') required String deviceId,
    @JsonKey(name: 'page_id') required String pageId,
    @Default(0) int widgets,
    @Default('') String status,
  }) = _AssignPageResult;

  factory AssignPageResult.fromJson(Map<String, dynamic> json) =>
      _$AssignPageResultFromJson(json);
}

@freezed
abstract class DeviceRow with _$DeviceRow {
  const factory DeviceRow({
    @JsonKey(name: 'device_id') required String deviceId,
    @Default('') String template,
    String? name,
    String? network,
    String? address,
    @JsonKey(name: 'site_id') String? siteId,
    @JsonKey(name: 'location_id') String? locationId,
    @JsonKey(name: 'page_id') String? pageId,
    @Default('') String status,
    @JsonKey(name: 'provisioned_at') String? provisionedAt,
  }) = _DeviceRow;

  factory DeviceRow.fromJson(Map<String, dynamic> json) =>
      _$DeviceRowFromJson(json);
}

@freezed
abstract class SiteRow with _$SiteRow {
  const factory SiteRow({
    @JsonKey(name: 'site_id') required String siteId,
    @Default('') String name,
  }) = _SiteRow;

  factory SiteRow.fromJson(Map<String, dynamic> json) =>
      _$SiteRowFromJson(json);
}

@freezed
abstract class LocationRow with _$LocationRow {
  const factory LocationRow({
    @JsonKey(name: 'location_id') required String locationId,
    @JsonKey(name: 'site_id') @Default('') String siteId,
    @Default('') String name,
  }) = _LocationRow;

  factory LocationRow.fromJson(Map<String, dynamic> json) =>
      _$LocationRowFromJson(json);
}

@freezed
abstract class PageRow with _$PageRow {
  const factory PageRow({
    @JsonKey(name: 'page_id') required String pageId,
    @JsonKey(name: 'site_id') String? siteId,
    @Default('') String name,
  }) = _PageRow;

  factory PageRow.fromJson(Map<String, dynamic> json) =>
      _$PageRowFromJson(json);
}

@freezed
abstract class TemplateRow with _$TemplateRow {
  const factory TemplateRow({
    required String template,
    @JsonKey(fromJson: _stringify) @Default('') String version,
    @JsonKey(name: 'display_name') @Default('') String displayName,
    @Default('') String network,
    @Default('') String category,
    @Default('') String icon,
  }) = _TemplateRow;

  factory TemplateRow.fromJson(Map<String, dynamic> json) =>
      _$TemplateRowFromJson(json);
}

@freezed
abstract class TemplateYaml with _$TemplateYaml {
  const factory TemplateYaml({
    required String template,
    @Default('') String yaml,
  }) = _TemplateYaml;

  factory TemplateYaml.fromJson(Map<String, dynamic> json) =>
      _$TemplateYamlFromJson(json);
}

@freezed
abstract class PointRow with _$PointRow {
  const factory PointRow({
    @JsonKey(name: 'point_id') required String pointId,
    @JsonKey(name: 'device_id') @Default('') String deviceId,
    @JsonKey(name: 'point_key') @Default('') String pointKey,
    @Default('') String name,
    String? unit,
    @Default('') String kind,
    @Default('stat') String widget,
    @Default(false) bool writable,
    @JsonKey(name: 'trend_on') @Default(false) bool trendOn,
    @JsonKey(name: 'alarm_on') @Default(false) bool alarmOn,
  }) = _PointRow;

  factory PointRow.fromJson(Map<String, dynamic> json) =>
      _$PointRowFromJson(json);
}

@freezed
abstract class WidgetRow with _$WidgetRow {
  const factory WidgetRow({
    @JsonKey(name: 'widget_id') required String widgetId,
    @JsonKey(name: 'page_id') @Default('') String pageId,
    @JsonKey(name: 'device_id') @Default('') String deviceId,
    @JsonKey(name: 'point_id') String? pointId,
    @Default('stat') String widget,
    String? role,
    String? title,
  }) = _WidgetRow;

  factory WidgetRow.fromJson(Map<String, dynamic> json) =>
      _$WidgetRowFromJson(json);
}

@freezed
abstract class LabelRender with _$LabelRender {
  const factory LabelRender({
    @JsonKey(name: 'device_id') required String deviceId,
    @Default('') String serial,
    @JsonKey(name: 'qr_url') @Default('') String qrUrl,
    @Default('') String code128,
    @JsonKey(name: 'display_name') @Default('') String displayName,
  }) = _LabelRender;

  factory LabelRender.fromJson(Map<String, dynamic> json) =>
      _$LabelRenderFromJson(json);
}

/// version arrives as a String or an int — normalise to String.
String _stringify(Object? v) => v?.toString() ?? '';
