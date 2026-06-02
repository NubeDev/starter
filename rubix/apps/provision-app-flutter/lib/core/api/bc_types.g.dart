// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'bc_types.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

_TemplatePoint _$TemplatePointFromJson(Map<String, dynamic> json) =>
    _TemplatePoint(
      key: json['key'] as String,
      name: json['name'] as String,
      widget: json['widget'] as String,
    );

Map<String, dynamic> _$TemplatePointToJson(_TemplatePoint instance) =>
    <String, dynamic>{
      'key': instance.key,
      'name': instance.name,
      'widget': instance.widget,
    };

_ScannedTemplate _$ScannedTemplateFromJson(Map<String, dynamic> json) =>
    _ScannedTemplate(
      displayName: json['display_name'] as String,
      icon: json['icon'] as String,
      category: json['category'] as String,
      points:
          (json['points'] as List<dynamic>?)
              ?.map((e) => TemplatePoint.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const <TemplatePoint>[],
      widgetGroup: json['widget_group'] as String? ?? '',
    );

Map<String, dynamic> _$ScannedTemplateToJson(_ScannedTemplate instance) =>
    <String, dynamic>{
      'display_name': instance.displayName,
      'icon': instance.icon,
      'category': instance.category,
      'points': instance.points,
      'widget_group': instance.widgetGroup,
    };

_ScannedIdentity _$ScannedIdentityFromJson(Map<String, dynamic> json) =>
    _ScannedIdentity(
      id: json['id'] as String,
      model: json['model'] as String,
      network: json['network'] as String,
      address: json['address'] as String? ?? '',
      defaultIp: json['default_ip'] as String? ?? '',
      hw: json['hw'] as String? ?? '',
      template: ScannedTemplate.fromJson(
        json['template'] as Map<String, dynamic>,
      ),
      knownModels:
          (json['known_models'] as List<dynamic>?)
              ?.map((e) => e as String)
              .toList() ??
          const <String>[],
    );

Map<String, dynamic> _$ScannedIdentityToJson(_ScannedIdentity instance) =>
    <String, dynamic>{
      'id': instance.id,
      'model': instance.model,
      'network': instance.network,
      'address': instance.address,
      'default_ip': instance.defaultIp,
      'hw': instance.hw,
      'template': instance.template,
      'known_models': instance.knownModels,
    };

_ProvisionResult _$ProvisionResultFromJson(Map<String, dynamic> json) =>
    _ProvisionResult(
      deviceId: json['device_id'] as String,
      points: (json['points'] as num?)?.toInt() ?? 0,
      widgets: (json['widgets'] as num?)?.toInt() ?? 0,
      alarms: (json['alarms'] as num?)?.toInt() ?? 0,
      pageId: json['page_id'] as String? ?? '',
      warnings:
          (json['warnings'] as List<dynamic>?)
              ?.map((e) => e as String)
              .toList() ??
          const <String>[],
    );

Map<String, dynamic> _$ProvisionResultToJson(_ProvisionResult instance) =>
    <String, dynamic>{
      'device_id': instance.deviceId,
      'points': instance.points,
      'widgets': instance.widgets,
      'alarms': instance.alarms,
      'page_id': instance.pageId,
      'warnings': instance.warnings,
    };

_AssignPageResult _$AssignPageResultFromJson(Map<String, dynamic> json) =>
    _AssignPageResult(
      deviceId: json['device_id'] as String,
      pageId: json['page_id'] as String,
      widgets: (json['widgets'] as num?)?.toInt() ?? 0,
      status: json['status'] as String? ?? '',
    );

Map<String, dynamic> _$AssignPageResultToJson(_AssignPageResult instance) =>
    <String, dynamic>{
      'device_id': instance.deviceId,
      'page_id': instance.pageId,
      'widgets': instance.widgets,
      'status': instance.status,
    };

_DeviceRow _$DeviceRowFromJson(Map<String, dynamic> json) => _DeviceRow(
  deviceId: json['device_id'] as String,
  template: json['template'] as String? ?? '',
  name: json['name'] as String?,
  network: json['network'] as String?,
  address: json['address'] as String?,
  siteId: json['site_id'] as String?,
  locationId: json['location_id'] as String?,
  pageId: json['page_id'] as String?,
  status: json['status'] as String? ?? '',
  provisionedAt: json['provisioned_at'] as String?,
);

Map<String, dynamic> _$DeviceRowToJson(_DeviceRow instance) =>
    <String, dynamic>{
      'device_id': instance.deviceId,
      'template': instance.template,
      'name': instance.name,
      'network': instance.network,
      'address': instance.address,
      'site_id': instance.siteId,
      'location_id': instance.locationId,
      'page_id': instance.pageId,
      'status': instance.status,
      'provisioned_at': instance.provisionedAt,
    };

_SiteRow _$SiteRowFromJson(Map<String, dynamic> json) => _SiteRow(
  siteId: json['site_id'] as String,
  name: json['name'] as String? ?? '',
);

Map<String, dynamic> _$SiteRowToJson(_SiteRow instance) => <String, dynamic>{
  'site_id': instance.siteId,
  'name': instance.name,
};

_LocationRow _$LocationRowFromJson(Map<String, dynamic> json) => _LocationRow(
  locationId: json['location_id'] as String,
  siteId: json['site_id'] as String? ?? '',
  name: json['name'] as String? ?? '',
);

Map<String, dynamic> _$LocationRowToJson(_LocationRow instance) =>
    <String, dynamic>{
      'location_id': instance.locationId,
      'site_id': instance.siteId,
      'name': instance.name,
    };

_PageRow _$PageRowFromJson(Map<String, dynamic> json) => _PageRow(
  pageId: json['page_id'] as String,
  siteId: json['site_id'] as String?,
  name: json['name'] as String? ?? '',
);

Map<String, dynamic> _$PageRowToJson(_PageRow instance) => <String, dynamic>{
  'page_id': instance.pageId,
  'site_id': instance.siteId,
  'name': instance.name,
};

_TemplateRow _$TemplateRowFromJson(Map<String, dynamic> json) => _TemplateRow(
  template: json['template'] as String,
  version: json['version'] == null ? '' : _stringify(json['version']),
  displayName: json['display_name'] as String? ?? '',
  network: json['network'] as String? ?? '',
  category: json['category'] as String? ?? '',
  icon: json['icon'] as String? ?? '',
);

Map<String, dynamic> _$TemplateRowToJson(_TemplateRow instance) =>
    <String, dynamic>{
      'template': instance.template,
      'version': instance.version,
      'display_name': instance.displayName,
      'network': instance.network,
      'category': instance.category,
      'icon': instance.icon,
    };

_TemplateYaml _$TemplateYamlFromJson(Map<String, dynamic> json) =>
    _TemplateYaml(
      template: json['template'] as String,
      yaml: json['yaml'] as String? ?? '',
    );

Map<String, dynamic> _$TemplateYamlToJson(_TemplateYaml instance) =>
    <String, dynamic>{'template': instance.template, 'yaml': instance.yaml};

_PointRow _$PointRowFromJson(Map<String, dynamic> json) => _PointRow(
  pointId: json['point_id'] as String,
  deviceId: json['device_id'] as String? ?? '',
  pointKey: json['point_key'] as String? ?? '',
  name: json['name'] as String? ?? '',
  unit: json['unit'] as String?,
  kind: json['kind'] as String? ?? '',
  widget: json['widget'] as String? ?? 'stat',
  writable: json['writable'] as bool? ?? false,
  trendOn: json['trend_on'] as bool? ?? false,
  alarmOn: json['alarm_on'] as bool? ?? false,
);

Map<String, dynamic> _$PointRowToJson(_PointRow instance) => <String, dynamic>{
  'point_id': instance.pointId,
  'device_id': instance.deviceId,
  'point_key': instance.pointKey,
  'name': instance.name,
  'unit': instance.unit,
  'kind': instance.kind,
  'widget': instance.widget,
  'writable': instance.writable,
  'trend_on': instance.trendOn,
  'alarm_on': instance.alarmOn,
};

_WidgetRow _$WidgetRowFromJson(Map<String, dynamic> json) => _WidgetRow(
  widgetId: json['widget_id'] as String,
  pageId: json['page_id'] as String? ?? '',
  deviceId: json['device_id'] as String? ?? '',
  pointId: json['point_id'] as String?,
  widget: json['widget'] as String? ?? 'stat',
  role: json['role'] as String?,
  title: json['title'] as String?,
);

Map<String, dynamic> _$WidgetRowToJson(_WidgetRow instance) =>
    <String, dynamic>{
      'widget_id': instance.widgetId,
      'page_id': instance.pageId,
      'device_id': instance.deviceId,
      'point_id': instance.pointId,
      'widget': instance.widget,
      'role': instance.role,
      'title': instance.title,
    };

_LabelRender _$LabelRenderFromJson(Map<String, dynamic> json) => _LabelRender(
  deviceId: json['device_id'] as String,
  serial: json['serial'] as String? ?? '',
  qrUrl: json['qr_url'] as String? ?? '',
  code128: json['code128'] as String? ?? '',
  displayName: json['display_name'] as String? ?? '',
);

Map<String, dynamic> _$LabelRenderToJson(_LabelRender instance) =>
    <String, dynamic>{
      'device_id': instance.deviceId,
      'serial': instance.serial,
      'qr_url': instance.qrUrl,
      'code128': instance.code128,
      'display_name': instance.displayName,
    };
