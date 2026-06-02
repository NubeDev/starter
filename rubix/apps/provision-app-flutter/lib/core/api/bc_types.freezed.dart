// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'bc_types.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;

/// @nodoc
mixin _$TemplatePoint {

 String get key; String get name; String get widget;
/// Create a copy of TemplatePoint
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TemplatePointCopyWith<TemplatePoint> get copyWith => _$TemplatePointCopyWithImpl<TemplatePoint>(this as TemplatePoint, _$identity);

  /// Serializes this TemplatePoint to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TemplatePoint&&(identical(other.key, key) || other.key == key)&&(identical(other.name, name) || other.name == name)&&(identical(other.widget, widget) || other.widget == widget));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,key,name,widget);

@override
String toString() {
  return 'TemplatePoint(key: $key, name: $name, widget: $widget)';
}


}

/// @nodoc
abstract mixin class $TemplatePointCopyWith<$Res>  {
  factory $TemplatePointCopyWith(TemplatePoint value, $Res Function(TemplatePoint) _then) = _$TemplatePointCopyWithImpl;
@useResult
$Res call({
 String key, String name, String widget
});




}
/// @nodoc
class _$TemplatePointCopyWithImpl<$Res>
    implements $TemplatePointCopyWith<$Res> {
  _$TemplatePointCopyWithImpl(this._self, this._then);

  final TemplatePoint _self;
  final $Res Function(TemplatePoint) _then;

/// Create a copy of TemplatePoint
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? key = null,Object? name = null,Object? widget = null,}) {
  return _then(_self.copyWith(
key: null == key ? _self.key : key // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,widget: null == widget ? _self.widget : widget // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// Adds pattern-matching-related methods to [TemplatePoint].
extension TemplatePointPatterns on TemplatePoint {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _TemplatePoint value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _TemplatePoint() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _TemplatePoint value)  $default,){
final _that = this;
switch (_that) {
case _TemplatePoint():
return $default(_that);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _TemplatePoint value)?  $default,){
final _that = this;
switch (_that) {
case _TemplatePoint() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String key,  String name,  String widget)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _TemplatePoint() when $default != null:
return $default(_that.key,_that.name,_that.widget);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String key,  String name,  String widget)  $default,) {final _that = this;
switch (_that) {
case _TemplatePoint():
return $default(_that.key,_that.name,_that.widget);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String key,  String name,  String widget)?  $default,) {final _that = this;
switch (_that) {
case _TemplatePoint() when $default != null:
return $default(_that.key,_that.name,_that.widget);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _TemplatePoint implements TemplatePoint {
  const _TemplatePoint({required this.key, required this.name, required this.widget});
  factory _TemplatePoint.fromJson(Map<String, dynamic> json) => _$TemplatePointFromJson(json);

@override final  String key;
@override final  String name;
@override final  String widget;

/// Create a copy of TemplatePoint
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$TemplatePointCopyWith<_TemplatePoint> get copyWith => __$TemplatePointCopyWithImpl<_TemplatePoint>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$TemplatePointToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _TemplatePoint&&(identical(other.key, key) || other.key == key)&&(identical(other.name, name) || other.name == name)&&(identical(other.widget, widget) || other.widget == widget));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,key,name,widget);

@override
String toString() {
  return 'TemplatePoint(key: $key, name: $name, widget: $widget)';
}


}

/// @nodoc
abstract mixin class _$TemplatePointCopyWith<$Res> implements $TemplatePointCopyWith<$Res> {
  factory _$TemplatePointCopyWith(_TemplatePoint value, $Res Function(_TemplatePoint) _then) = __$TemplatePointCopyWithImpl;
@override @useResult
$Res call({
 String key, String name, String widget
});




}
/// @nodoc
class __$TemplatePointCopyWithImpl<$Res>
    implements _$TemplatePointCopyWith<$Res> {
  __$TemplatePointCopyWithImpl(this._self, this._then);

  final _TemplatePoint _self;
  final $Res Function(_TemplatePoint) _then;

/// Create a copy of TemplatePoint
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? key = null,Object? name = null,Object? widget = null,}) {
  return _then(_TemplatePoint(
key: null == key ? _self.key : key // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,widget: null == widget ? _self.widget : widget // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}


/// @nodoc
mixin _$ScannedTemplate {

@JsonKey(name: 'display_name') String get displayName; String get icon; String get category; List<TemplatePoint> get points;@JsonKey(name: 'widget_group') String get widgetGroup;
/// Create a copy of ScannedTemplate
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ScannedTemplateCopyWith<ScannedTemplate> get copyWith => _$ScannedTemplateCopyWithImpl<ScannedTemplate>(this as ScannedTemplate, _$identity);

  /// Serializes this ScannedTemplate to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ScannedTemplate&&(identical(other.displayName, displayName) || other.displayName == displayName)&&(identical(other.icon, icon) || other.icon == icon)&&(identical(other.category, category) || other.category == category)&&const DeepCollectionEquality().equals(other.points, points)&&(identical(other.widgetGroup, widgetGroup) || other.widgetGroup == widgetGroup));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,displayName,icon,category,const DeepCollectionEquality().hash(points),widgetGroup);

@override
String toString() {
  return 'ScannedTemplate(displayName: $displayName, icon: $icon, category: $category, points: $points, widgetGroup: $widgetGroup)';
}


}

/// @nodoc
abstract mixin class $ScannedTemplateCopyWith<$Res>  {
  factory $ScannedTemplateCopyWith(ScannedTemplate value, $Res Function(ScannedTemplate) _then) = _$ScannedTemplateCopyWithImpl;
@useResult
$Res call({
@JsonKey(name: 'display_name') String displayName, String icon, String category, List<TemplatePoint> points,@JsonKey(name: 'widget_group') String widgetGroup
});




}
/// @nodoc
class _$ScannedTemplateCopyWithImpl<$Res>
    implements $ScannedTemplateCopyWith<$Res> {
  _$ScannedTemplateCopyWithImpl(this._self, this._then);

  final ScannedTemplate _self;
  final $Res Function(ScannedTemplate) _then;

/// Create a copy of ScannedTemplate
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? displayName = null,Object? icon = null,Object? category = null,Object? points = null,Object? widgetGroup = null,}) {
  return _then(_self.copyWith(
displayName: null == displayName ? _self.displayName : displayName // ignore: cast_nullable_to_non_nullable
as String,icon: null == icon ? _self.icon : icon // ignore: cast_nullable_to_non_nullable
as String,category: null == category ? _self.category : category // ignore: cast_nullable_to_non_nullable
as String,points: null == points ? _self.points : points // ignore: cast_nullable_to_non_nullable
as List<TemplatePoint>,widgetGroup: null == widgetGroup ? _self.widgetGroup : widgetGroup // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// Adds pattern-matching-related methods to [ScannedTemplate].
extension ScannedTemplatePatterns on ScannedTemplate {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _ScannedTemplate value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _ScannedTemplate() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _ScannedTemplate value)  $default,){
final _that = this;
switch (_that) {
case _ScannedTemplate():
return $default(_that);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _ScannedTemplate value)?  $default,){
final _that = this;
switch (_that) {
case _ScannedTemplate() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function(@JsonKey(name: 'display_name')  String displayName,  String icon,  String category,  List<TemplatePoint> points, @JsonKey(name: 'widget_group')  String widgetGroup)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _ScannedTemplate() when $default != null:
return $default(_that.displayName,_that.icon,_that.category,_that.points,_that.widgetGroup);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function(@JsonKey(name: 'display_name')  String displayName,  String icon,  String category,  List<TemplatePoint> points, @JsonKey(name: 'widget_group')  String widgetGroup)  $default,) {final _that = this;
switch (_that) {
case _ScannedTemplate():
return $default(_that.displayName,_that.icon,_that.category,_that.points,_that.widgetGroup);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function(@JsonKey(name: 'display_name')  String displayName,  String icon,  String category,  List<TemplatePoint> points, @JsonKey(name: 'widget_group')  String widgetGroup)?  $default,) {final _that = this;
switch (_that) {
case _ScannedTemplate() when $default != null:
return $default(_that.displayName,_that.icon,_that.category,_that.points,_that.widgetGroup);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _ScannedTemplate implements ScannedTemplate {
  const _ScannedTemplate({@JsonKey(name: 'display_name') required this.displayName, required this.icon, required this.category, final  List<TemplatePoint> points = const <TemplatePoint>[], @JsonKey(name: 'widget_group') this.widgetGroup = ''}): _points = points;
  factory _ScannedTemplate.fromJson(Map<String, dynamic> json) => _$ScannedTemplateFromJson(json);

@override@JsonKey(name: 'display_name') final  String displayName;
@override final  String icon;
@override final  String category;
 final  List<TemplatePoint> _points;
@override@JsonKey() List<TemplatePoint> get points {
  if (_points is EqualUnmodifiableListView) return _points;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_points);
}

@override@JsonKey(name: 'widget_group') final  String widgetGroup;

/// Create a copy of ScannedTemplate
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$ScannedTemplateCopyWith<_ScannedTemplate> get copyWith => __$ScannedTemplateCopyWithImpl<_ScannedTemplate>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$ScannedTemplateToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _ScannedTemplate&&(identical(other.displayName, displayName) || other.displayName == displayName)&&(identical(other.icon, icon) || other.icon == icon)&&(identical(other.category, category) || other.category == category)&&const DeepCollectionEquality().equals(other._points, _points)&&(identical(other.widgetGroup, widgetGroup) || other.widgetGroup == widgetGroup));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,displayName,icon,category,const DeepCollectionEquality().hash(_points),widgetGroup);

@override
String toString() {
  return 'ScannedTemplate(displayName: $displayName, icon: $icon, category: $category, points: $points, widgetGroup: $widgetGroup)';
}


}

/// @nodoc
abstract mixin class _$ScannedTemplateCopyWith<$Res> implements $ScannedTemplateCopyWith<$Res> {
  factory _$ScannedTemplateCopyWith(_ScannedTemplate value, $Res Function(_ScannedTemplate) _then) = __$ScannedTemplateCopyWithImpl;
@override @useResult
$Res call({
@JsonKey(name: 'display_name') String displayName, String icon, String category, List<TemplatePoint> points,@JsonKey(name: 'widget_group') String widgetGroup
});




}
/// @nodoc
class __$ScannedTemplateCopyWithImpl<$Res>
    implements _$ScannedTemplateCopyWith<$Res> {
  __$ScannedTemplateCopyWithImpl(this._self, this._then);

  final _ScannedTemplate _self;
  final $Res Function(_ScannedTemplate) _then;

/// Create a copy of ScannedTemplate
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? displayName = null,Object? icon = null,Object? category = null,Object? points = null,Object? widgetGroup = null,}) {
  return _then(_ScannedTemplate(
displayName: null == displayName ? _self.displayName : displayName // ignore: cast_nullable_to_non_nullable
as String,icon: null == icon ? _self.icon : icon // ignore: cast_nullable_to_non_nullable
as String,category: null == category ? _self.category : category // ignore: cast_nullable_to_non_nullable
as String,points: null == points ? _self._points : points // ignore: cast_nullable_to_non_nullable
as List<TemplatePoint>,widgetGroup: null == widgetGroup ? _self.widgetGroup : widgetGroup // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}


/// @nodoc
mixin _$ScannedIdentity {

 String get id; String get model; String get network; String get address;@JsonKey(name: 'default_ip') String get defaultIp; String get hw; ScannedTemplate get template;@JsonKey(name: 'known_models') List<String> get knownModels;
/// Create a copy of ScannedIdentity
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ScannedIdentityCopyWith<ScannedIdentity> get copyWith => _$ScannedIdentityCopyWithImpl<ScannedIdentity>(this as ScannedIdentity, _$identity);

  /// Serializes this ScannedIdentity to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ScannedIdentity&&(identical(other.id, id) || other.id == id)&&(identical(other.model, model) || other.model == model)&&(identical(other.network, network) || other.network == network)&&(identical(other.address, address) || other.address == address)&&(identical(other.defaultIp, defaultIp) || other.defaultIp == defaultIp)&&(identical(other.hw, hw) || other.hw == hw)&&(identical(other.template, template) || other.template == template)&&const DeepCollectionEquality().equals(other.knownModels, knownModels));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,id,model,network,address,defaultIp,hw,template,const DeepCollectionEquality().hash(knownModels));

@override
String toString() {
  return 'ScannedIdentity(id: $id, model: $model, network: $network, address: $address, defaultIp: $defaultIp, hw: $hw, template: $template, knownModels: $knownModels)';
}


}

/// @nodoc
abstract mixin class $ScannedIdentityCopyWith<$Res>  {
  factory $ScannedIdentityCopyWith(ScannedIdentity value, $Res Function(ScannedIdentity) _then) = _$ScannedIdentityCopyWithImpl;
@useResult
$Res call({
 String id, String model, String network, String address,@JsonKey(name: 'default_ip') String defaultIp, String hw, ScannedTemplate template,@JsonKey(name: 'known_models') List<String> knownModels
});


$ScannedTemplateCopyWith<$Res> get template;

}
/// @nodoc
class _$ScannedIdentityCopyWithImpl<$Res>
    implements $ScannedIdentityCopyWith<$Res> {
  _$ScannedIdentityCopyWithImpl(this._self, this._then);

  final ScannedIdentity _self;
  final $Res Function(ScannedIdentity) _then;

/// Create a copy of ScannedIdentity
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? id = null,Object? model = null,Object? network = null,Object? address = null,Object? defaultIp = null,Object? hw = null,Object? template = null,Object? knownModels = null,}) {
  return _then(_self.copyWith(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,model: null == model ? _self.model : model // ignore: cast_nullable_to_non_nullable
as String,network: null == network ? _self.network : network // ignore: cast_nullable_to_non_nullable
as String,address: null == address ? _self.address : address // ignore: cast_nullable_to_non_nullable
as String,defaultIp: null == defaultIp ? _self.defaultIp : defaultIp // ignore: cast_nullable_to_non_nullable
as String,hw: null == hw ? _self.hw : hw // ignore: cast_nullable_to_non_nullable
as String,template: null == template ? _self.template : template // ignore: cast_nullable_to_non_nullable
as ScannedTemplate,knownModels: null == knownModels ? _self.knownModels : knownModels // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}
/// Create a copy of ScannedIdentity
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$ScannedTemplateCopyWith<$Res> get template {
  
  return $ScannedTemplateCopyWith<$Res>(_self.template, (value) {
    return _then(_self.copyWith(template: value));
  });
}
}


/// Adds pattern-matching-related methods to [ScannedIdentity].
extension ScannedIdentityPatterns on ScannedIdentity {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _ScannedIdentity value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _ScannedIdentity() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _ScannedIdentity value)  $default,){
final _that = this;
switch (_that) {
case _ScannedIdentity():
return $default(_that);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _ScannedIdentity value)?  $default,){
final _that = this;
switch (_that) {
case _ScannedIdentity() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String id,  String model,  String network,  String address, @JsonKey(name: 'default_ip')  String defaultIp,  String hw,  ScannedTemplate template, @JsonKey(name: 'known_models')  List<String> knownModels)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _ScannedIdentity() when $default != null:
return $default(_that.id,_that.model,_that.network,_that.address,_that.defaultIp,_that.hw,_that.template,_that.knownModels);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String id,  String model,  String network,  String address, @JsonKey(name: 'default_ip')  String defaultIp,  String hw,  ScannedTemplate template, @JsonKey(name: 'known_models')  List<String> knownModels)  $default,) {final _that = this;
switch (_that) {
case _ScannedIdentity():
return $default(_that.id,_that.model,_that.network,_that.address,_that.defaultIp,_that.hw,_that.template,_that.knownModels);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String id,  String model,  String network,  String address, @JsonKey(name: 'default_ip')  String defaultIp,  String hw,  ScannedTemplate template, @JsonKey(name: 'known_models')  List<String> knownModels)?  $default,) {final _that = this;
switch (_that) {
case _ScannedIdentity() when $default != null:
return $default(_that.id,_that.model,_that.network,_that.address,_that.defaultIp,_that.hw,_that.template,_that.knownModels);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _ScannedIdentity implements ScannedIdentity {
  const _ScannedIdentity({required this.id, required this.model, required this.network, this.address = '', @JsonKey(name: 'default_ip') this.defaultIp = '', this.hw = '', required this.template, @JsonKey(name: 'known_models') final  List<String> knownModels = const <String>[]}): _knownModels = knownModels;
  factory _ScannedIdentity.fromJson(Map<String, dynamic> json) => _$ScannedIdentityFromJson(json);

@override final  String id;
@override final  String model;
@override final  String network;
@override@JsonKey() final  String address;
@override@JsonKey(name: 'default_ip') final  String defaultIp;
@override@JsonKey() final  String hw;
@override final  ScannedTemplate template;
 final  List<String> _knownModels;
@override@JsonKey(name: 'known_models') List<String> get knownModels {
  if (_knownModels is EqualUnmodifiableListView) return _knownModels;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_knownModels);
}


/// Create a copy of ScannedIdentity
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$ScannedIdentityCopyWith<_ScannedIdentity> get copyWith => __$ScannedIdentityCopyWithImpl<_ScannedIdentity>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$ScannedIdentityToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _ScannedIdentity&&(identical(other.id, id) || other.id == id)&&(identical(other.model, model) || other.model == model)&&(identical(other.network, network) || other.network == network)&&(identical(other.address, address) || other.address == address)&&(identical(other.defaultIp, defaultIp) || other.defaultIp == defaultIp)&&(identical(other.hw, hw) || other.hw == hw)&&(identical(other.template, template) || other.template == template)&&const DeepCollectionEquality().equals(other._knownModels, _knownModels));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,id,model,network,address,defaultIp,hw,template,const DeepCollectionEquality().hash(_knownModels));

@override
String toString() {
  return 'ScannedIdentity(id: $id, model: $model, network: $network, address: $address, defaultIp: $defaultIp, hw: $hw, template: $template, knownModels: $knownModels)';
}


}

/// @nodoc
abstract mixin class _$ScannedIdentityCopyWith<$Res> implements $ScannedIdentityCopyWith<$Res> {
  factory _$ScannedIdentityCopyWith(_ScannedIdentity value, $Res Function(_ScannedIdentity) _then) = __$ScannedIdentityCopyWithImpl;
@override @useResult
$Res call({
 String id, String model, String network, String address,@JsonKey(name: 'default_ip') String defaultIp, String hw, ScannedTemplate template,@JsonKey(name: 'known_models') List<String> knownModels
});


@override $ScannedTemplateCopyWith<$Res> get template;

}
/// @nodoc
class __$ScannedIdentityCopyWithImpl<$Res>
    implements _$ScannedIdentityCopyWith<$Res> {
  __$ScannedIdentityCopyWithImpl(this._self, this._then);

  final _ScannedIdentity _self;
  final $Res Function(_ScannedIdentity) _then;

/// Create a copy of ScannedIdentity
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? id = null,Object? model = null,Object? network = null,Object? address = null,Object? defaultIp = null,Object? hw = null,Object? template = null,Object? knownModels = null,}) {
  return _then(_ScannedIdentity(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,model: null == model ? _self.model : model // ignore: cast_nullable_to_non_nullable
as String,network: null == network ? _self.network : network // ignore: cast_nullable_to_non_nullable
as String,address: null == address ? _self.address : address // ignore: cast_nullable_to_non_nullable
as String,defaultIp: null == defaultIp ? _self.defaultIp : defaultIp // ignore: cast_nullable_to_non_nullable
as String,hw: null == hw ? _self.hw : hw // ignore: cast_nullable_to_non_nullable
as String,template: null == template ? _self.template : template // ignore: cast_nullable_to_non_nullable
as ScannedTemplate,knownModels: null == knownModels ? _self._knownModels : knownModels // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}

/// Create a copy of ScannedIdentity
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$ScannedTemplateCopyWith<$Res> get template {
  
  return $ScannedTemplateCopyWith<$Res>(_self.template, (value) {
    return _then(_self.copyWith(template: value));
  });
}
}


/// @nodoc
mixin _$ProvisionResult {

@JsonKey(name: 'device_id') String get deviceId; int get points; int get widgets; int get alarms;@JsonKey(name: 'page_id') String get pageId; List<String> get warnings;
/// Create a copy of ProvisionResult
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ProvisionResultCopyWith<ProvisionResult> get copyWith => _$ProvisionResultCopyWithImpl<ProvisionResult>(this as ProvisionResult, _$identity);

  /// Serializes this ProvisionResult to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ProvisionResult&&(identical(other.deviceId, deviceId) || other.deviceId == deviceId)&&(identical(other.points, points) || other.points == points)&&(identical(other.widgets, widgets) || other.widgets == widgets)&&(identical(other.alarms, alarms) || other.alarms == alarms)&&(identical(other.pageId, pageId) || other.pageId == pageId)&&const DeepCollectionEquality().equals(other.warnings, warnings));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,deviceId,points,widgets,alarms,pageId,const DeepCollectionEquality().hash(warnings));

@override
String toString() {
  return 'ProvisionResult(deviceId: $deviceId, points: $points, widgets: $widgets, alarms: $alarms, pageId: $pageId, warnings: $warnings)';
}


}

/// @nodoc
abstract mixin class $ProvisionResultCopyWith<$Res>  {
  factory $ProvisionResultCopyWith(ProvisionResult value, $Res Function(ProvisionResult) _then) = _$ProvisionResultCopyWithImpl;
@useResult
$Res call({
@JsonKey(name: 'device_id') String deviceId, int points, int widgets, int alarms,@JsonKey(name: 'page_id') String pageId, List<String> warnings
});




}
/// @nodoc
class _$ProvisionResultCopyWithImpl<$Res>
    implements $ProvisionResultCopyWith<$Res> {
  _$ProvisionResultCopyWithImpl(this._self, this._then);

  final ProvisionResult _self;
  final $Res Function(ProvisionResult) _then;

/// Create a copy of ProvisionResult
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? deviceId = null,Object? points = null,Object? widgets = null,Object? alarms = null,Object? pageId = null,Object? warnings = null,}) {
  return _then(_self.copyWith(
deviceId: null == deviceId ? _self.deviceId : deviceId // ignore: cast_nullable_to_non_nullable
as String,points: null == points ? _self.points : points // ignore: cast_nullable_to_non_nullable
as int,widgets: null == widgets ? _self.widgets : widgets // ignore: cast_nullable_to_non_nullable
as int,alarms: null == alarms ? _self.alarms : alarms // ignore: cast_nullable_to_non_nullable
as int,pageId: null == pageId ? _self.pageId : pageId // ignore: cast_nullable_to_non_nullable
as String,warnings: null == warnings ? _self.warnings : warnings // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}

}


/// Adds pattern-matching-related methods to [ProvisionResult].
extension ProvisionResultPatterns on ProvisionResult {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _ProvisionResult value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _ProvisionResult() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _ProvisionResult value)  $default,){
final _that = this;
switch (_that) {
case _ProvisionResult():
return $default(_that);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _ProvisionResult value)?  $default,){
final _that = this;
switch (_that) {
case _ProvisionResult() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function(@JsonKey(name: 'device_id')  String deviceId,  int points,  int widgets,  int alarms, @JsonKey(name: 'page_id')  String pageId,  List<String> warnings)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _ProvisionResult() when $default != null:
return $default(_that.deviceId,_that.points,_that.widgets,_that.alarms,_that.pageId,_that.warnings);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function(@JsonKey(name: 'device_id')  String deviceId,  int points,  int widgets,  int alarms, @JsonKey(name: 'page_id')  String pageId,  List<String> warnings)  $default,) {final _that = this;
switch (_that) {
case _ProvisionResult():
return $default(_that.deviceId,_that.points,_that.widgets,_that.alarms,_that.pageId,_that.warnings);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function(@JsonKey(name: 'device_id')  String deviceId,  int points,  int widgets,  int alarms, @JsonKey(name: 'page_id')  String pageId,  List<String> warnings)?  $default,) {final _that = this;
switch (_that) {
case _ProvisionResult() when $default != null:
return $default(_that.deviceId,_that.points,_that.widgets,_that.alarms,_that.pageId,_that.warnings);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _ProvisionResult implements ProvisionResult {
  const _ProvisionResult({@JsonKey(name: 'device_id') required this.deviceId, this.points = 0, this.widgets = 0, this.alarms = 0, @JsonKey(name: 'page_id') this.pageId = '', final  List<String> warnings = const <String>[]}): _warnings = warnings;
  factory _ProvisionResult.fromJson(Map<String, dynamic> json) => _$ProvisionResultFromJson(json);

@override@JsonKey(name: 'device_id') final  String deviceId;
@override@JsonKey() final  int points;
@override@JsonKey() final  int widgets;
@override@JsonKey() final  int alarms;
@override@JsonKey(name: 'page_id') final  String pageId;
 final  List<String> _warnings;
@override@JsonKey() List<String> get warnings {
  if (_warnings is EqualUnmodifiableListView) return _warnings;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_warnings);
}


/// Create a copy of ProvisionResult
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$ProvisionResultCopyWith<_ProvisionResult> get copyWith => __$ProvisionResultCopyWithImpl<_ProvisionResult>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$ProvisionResultToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _ProvisionResult&&(identical(other.deviceId, deviceId) || other.deviceId == deviceId)&&(identical(other.points, points) || other.points == points)&&(identical(other.widgets, widgets) || other.widgets == widgets)&&(identical(other.alarms, alarms) || other.alarms == alarms)&&(identical(other.pageId, pageId) || other.pageId == pageId)&&const DeepCollectionEquality().equals(other._warnings, _warnings));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,deviceId,points,widgets,alarms,pageId,const DeepCollectionEquality().hash(_warnings));

@override
String toString() {
  return 'ProvisionResult(deviceId: $deviceId, points: $points, widgets: $widgets, alarms: $alarms, pageId: $pageId, warnings: $warnings)';
}


}

/// @nodoc
abstract mixin class _$ProvisionResultCopyWith<$Res> implements $ProvisionResultCopyWith<$Res> {
  factory _$ProvisionResultCopyWith(_ProvisionResult value, $Res Function(_ProvisionResult) _then) = __$ProvisionResultCopyWithImpl;
@override @useResult
$Res call({
@JsonKey(name: 'device_id') String deviceId, int points, int widgets, int alarms,@JsonKey(name: 'page_id') String pageId, List<String> warnings
});




}
/// @nodoc
class __$ProvisionResultCopyWithImpl<$Res>
    implements _$ProvisionResultCopyWith<$Res> {
  __$ProvisionResultCopyWithImpl(this._self, this._then);

  final _ProvisionResult _self;
  final $Res Function(_ProvisionResult) _then;

/// Create a copy of ProvisionResult
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? deviceId = null,Object? points = null,Object? widgets = null,Object? alarms = null,Object? pageId = null,Object? warnings = null,}) {
  return _then(_ProvisionResult(
deviceId: null == deviceId ? _self.deviceId : deviceId // ignore: cast_nullable_to_non_nullable
as String,points: null == points ? _self.points : points // ignore: cast_nullable_to_non_nullable
as int,widgets: null == widgets ? _self.widgets : widgets // ignore: cast_nullable_to_non_nullable
as int,alarms: null == alarms ? _self.alarms : alarms // ignore: cast_nullable_to_non_nullable
as int,pageId: null == pageId ? _self.pageId : pageId // ignore: cast_nullable_to_non_nullable
as String,warnings: null == warnings ? _self._warnings : warnings // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}


}


/// @nodoc
mixin _$AssignPageResult {

@JsonKey(name: 'device_id') String get deviceId;@JsonKey(name: 'page_id') String get pageId; int get widgets; String get status;
/// Create a copy of AssignPageResult
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AssignPageResultCopyWith<AssignPageResult> get copyWith => _$AssignPageResultCopyWithImpl<AssignPageResult>(this as AssignPageResult, _$identity);

  /// Serializes this AssignPageResult to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AssignPageResult&&(identical(other.deviceId, deviceId) || other.deviceId == deviceId)&&(identical(other.pageId, pageId) || other.pageId == pageId)&&(identical(other.widgets, widgets) || other.widgets == widgets)&&(identical(other.status, status) || other.status == status));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,deviceId,pageId,widgets,status);

@override
String toString() {
  return 'AssignPageResult(deviceId: $deviceId, pageId: $pageId, widgets: $widgets, status: $status)';
}


}

/// @nodoc
abstract mixin class $AssignPageResultCopyWith<$Res>  {
  factory $AssignPageResultCopyWith(AssignPageResult value, $Res Function(AssignPageResult) _then) = _$AssignPageResultCopyWithImpl;
@useResult
$Res call({
@JsonKey(name: 'device_id') String deviceId,@JsonKey(name: 'page_id') String pageId, int widgets, String status
});




}
/// @nodoc
class _$AssignPageResultCopyWithImpl<$Res>
    implements $AssignPageResultCopyWith<$Res> {
  _$AssignPageResultCopyWithImpl(this._self, this._then);

  final AssignPageResult _self;
  final $Res Function(AssignPageResult) _then;

/// Create a copy of AssignPageResult
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? deviceId = null,Object? pageId = null,Object? widgets = null,Object? status = null,}) {
  return _then(_self.copyWith(
deviceId: null == deviceId ? _self.deviceId : deviceId // ignore: cast_nullable_to_non_nullable
as String,pageId: null == pageId ? _self.pageId : pageId // ignore: cast_nullable_to_non_nullable
as String,widgets: null == widgets ? _self.widgets : widgets // ignore: cast_nullable_to_non_nullable
as int,status: null == status ? _self.status : status // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// Adds pattern-matching-related methods to [AssignPageResult].
extension AssignPageResultPatterns on AssignPageResult {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _AssignPageResult value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _AssignPageResult() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _AssignPageResult value)  $default,){
final _that = this;
switch (_that) {
case _AssignPageResult():
return $default(_that);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _AssignPageResult value)?  $default,){
final _that = this;
switch (_that) {
case _AssignPageResult() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function(@JsonKey(name: 'device_id')  String deviceId, @JsonKey(name: 'page_id')  String pageId,  int widgets,  String status)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _AssignPageResult() when $default != null:
return $default(_that.deviceId,_that.pageId,_that.widgets,_that.status);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function(@JsonKey(name: 'device_id')  String deviceId, @JsonKey(name: 'page_id')  String pageId,  int widgets,  String status)  $default,) {final _that = this;
switch (_that) {
case _AssignPageResult():
return $default(_that.deviceId,_that.pageId,_that.widgets,_that.status);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function(@JsonKey(name: 'device_id')  String deviceId, @JsonKey(name: 'page_id')  String pageId,  int widgets,  String status)?  $default,) {final _that = this;
switch (_that) {
case _AssignPageResult() when $default != null:
return $default(_that.deviceId,_that.pageId,_that.widgets,_that.status);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _AssignPageResult implements AssignPageResult {
  const _AssignPageResult({@JsonKey(name: 'device_id') required this.deviceId, @JsonKey(name: 'page_id') required this.pageId, this.widgets = 0, this.status = ''});
  factory _AssignPageResult.fromJson(Map<String, dynamic> json) => _$AssignPageResultFromJson(json);

@override@JsonKey(name: 'device_id') final  String deviceId;
@override@JsonKey(name: 'page_id') final  String pageId;
@override@JsonKey() final  int widgets;
@override@JsonKey() final  String status;

/// Create a copy of AssignPageResult
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$AssignPageResultCopyWith<_AssignPageResult> get copyWith => __$AssignPageResultCopyWithImpl<_AssignPageResult>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$AssignPageResultToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _AssignPageResult&&(identical(other.deviceId, deviceId) || other.deviceId == deviceId)&&(identical(other.pageId, pageId) || other.pageId == pageId)&&(identical(other.widgets, widgets) || other.widgets == widgets)&&(identical(other.status, status) || other.status == status));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,deviceId,pageId,widgets,status);

@override
String toString() {
  return 'AssignPageResult(deviceId: $deviceId, pageId: $pageId, widgets: $widgets, status: $status)';
}


}

/// @nodoc
abstract mixin class _$AssignPageResultCopyWith<$Res> implements $AssignPageResultCopyWith<$Res> {
  factory _$AssignPageResultCopyWith(_AssignPageResult value, $Res Function(_AssignPageResult) _then) = __$AssignPageResultCopyWithImpl;
@override @useResult
$Res call({
@JsonKey(name: 'device_id') String deviceId,@JsonKey(name: 'page_id') String pageId, int widgets, String status
});




}
/// @nodoc
class __$AssignPageResultCopyWithImpl<$Res>
    implements _$AssignPageResultCopyWith<$Res> {
  __$AssignPageResultCopyWithImpl(this._self, this._then);

  final _AssignPageResult _self;
  final $Res Function(_AssignPageResult) _then;

/// Create a copy of AssignPageResult
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? deviceId = null,Object? pageId = null,Object? widgets = null,Object? status = null,}) {
  return _then(_AssignPageResult(
deviceId: null == deviceId ? _self.deviceId : deviceId // ignore: cast_nullable_to_non_nullable
as String,pageId: null == pageId ? _self.pageId : pageId // ignore: cast_nullable_to_non_nullable
as String,widgets: null == widgets ? _self.widgets : widgets // ignore: cast_nullable_to_non_nullable
as int,status: null == status ? _self.status : status // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}


/// @nodoc
mixin _$DeviceRow {

@JsonKey(name: 'device_id') String get deviceId; String get template; String? get name; String? get network; String? get address;@JsonKey(name: 'site_id') String? get siteId;@JsonKey(name: 'location_id') String? get locationId;@JsonKey(name: 'page_id') String? get pageId; String get status;@JsonKey(name: 'provisioned_at') String? get provisionedAt;
/// Create a copy of DeviceRow
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$DeviceRowCopyWith<DeviceRow> get copyWith => _$DeviceRowCopyWithImpl<DeviceRow>(this as DeviceRow, _$identity);

  /// Serializes this DeviceRow to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DeviceRow&&(identical(other.deviceId, deviceId) || other.deviceId == deviceId)&&(identical(other.template, template) || other.template == template)&&(identical(other.name, name) || other.name == name)&&(identical(other.network, network) || other.network == network)&&(identical(other.address, address) || other.address == address)&&(identical(other.siteId, siteId) || other.siteId == siteId)&&(identical(other.locationId, locationId) || other.locationId == locationId)&&(identical(other.pageId, pageId) || other.pageId == pageId)&&(identical(other.status, status) || other.status == status)&&(identical(other.provisionedAt, provisionedAt) || other.provisionedAt == provisionedAt));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,deviceId,template,name,network,address,siteId,locationId,pageId,status,provisionedAt);

@override
String toString() {
  return 'DeviceRow(deviceId: $deviceId, template: $template, name: $name, network: $network, address: $address, siteId: $siteId, locationId: $locationId, pageId: $pageId, status: $status, provisionedAt: $provisionedAt)';
}


}

/// @nodoc
abstract mixin class $DeviceRowCopyWith<$Res>  {
  factory $DeviceRowCopyWith(DeviceRow value, $Res Function(DeviceRow) _then) = _$DeviceRowCopyWithImpl;
@useResult
$Res call({
@JsonKey(name: 'device_id') String deviceId, String template, String? name, String? network, String? address,@JsonKey(name: 'site_id') String? siteId,@JsonKey(name: 'location_id') String? locationId,@JsonKey(name: 'page_id') String? pageId, String status,@JsonKey(name: 'provisioned_at') String? provisionedAt
});




}
/// @nodoc
class _$DeviceRowCopyWithImpl<$Res>
    implements $DeviceRowCopyWith<$Res> {
  _$DeviceRowCopyWithImpl(this._self, this._then);

  final DeviceRow _self;
  final $Res Function(DeviceRow) _then;

/// Create a copy of DeviceRow
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? deviceId = null,Object? template = null,Object? name = freezed,Object? network = freezed,Object? address = freezed,Object? siteId = freezed,Object? locationId = freezed,Object? pageId = freezed,Object? status = null,Object? provisionedAt = freezed,}) {
  return _then(_self.copyWith(
deviceId: null == deviceId ? _self.deviceId : deviceId // ignore: cast_nullable_to_non_nullable
as String,template: null == template ? _self.template : template // ignore: cast_nullable_to_non_nullable
as String,name: freezed == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String?,network: freezed == network ? _self.network : network // ignore: cast_nullable_to_non_nullable
as String?,address: freezed == address ? _self.address : address // ignore: cast_nullable_to_non_nullable
as String?,siteId: freezed == siteId ? _self.siteId : siteId // ignore: cast_nullable_to_non_nullable
as String?,locationId: freezed == locationId ? _self.locationId : locationId // ignore: cast_nullable_to_non_nullable
as String?,pageId: freezed == pageId ? _self.pageId : pageId // ignore: cast_nullable_to_non_nullable
as String?,status: null == status ? _self.status : status // ignore: cast_nullable_to_non_nullable
as String,provisionedAt: freezed == provisionedAt ? _self.provisionedAt : provisionedAt // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}

}


/// Adds pattern-matching-related methods to [DeviceRow].
extension DeviceRowPatterns on DeviceRow {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _DeviceRow value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _DeviceRow() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _DeviceRow value)  $default,){
final _that = this;
switch (_that) {
case _DeviceRow():
return $default(_that);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _DeviceRow value)?  $default,){
final _that = this;
switch (_that) {
case _DeviceRow() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function(@JsonKey(name: 'device_id')  String deviceId,  String template,  String? name,  String? network,  String? address, @JsonKey(name: 'site_id')  String? siteId, @JsonKey(name: 'location_id')  String? locationId, @JsonKey(name: 'page_id')  String? pageId,  String status, @JsonKey(name: 'provisioned_at')  String? provisionedAt)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _DeviceRow() when $default != null:
return $default(_that.deviceId,_that.template,_that.name,_that.network,_that.address,_that.siteId,_that.locationId,_that.pageId,_that.status,_that.provisionedAt);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function(@JsonKey(name: 'device_id')  String deviceId,  String template,  String? name,  String? network,  String? address, @JsonKey(name: 'site_id')  String? siteId, @JsonKey(name: 'location_id')  String? locationId, @JsonKey(name: 'page_id')  String? pageId,  String status, @JsonKey(name: 'provisioned_at')  String? provisionedAt)  $default,) {final _that = this;
switch (_that) {
case _DeviceRow():
return $default(_that.deviceId,_that.template,_that.name,_that.network,_that.address,_that.siteId,_that.locationId,_that.pageId,_that.status,_that.provisionedAt);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function(@JsonKey(name: 'device_id')  String deviceId,  String template,  String? name,  String? network,  String? address, @JsonKey(name: 'site_id')  String? siteId, @JsonKey(name: 'location_id')  String? locationId, @JsonKey(name: 'page_id')  String? pageId,  String status, @JsonKey(name: 'provisioned_at')  String? provisionedAt)?  $default,) {final _that = this;
switch (_that) {
case _DeviceRow() when $default != null:
return $default(_that.deviceId,_that.template,_that.name,_that.network,_that.address,_that.siteId,_that.locationId,_that.pageId,_that.status,_that.provisionedAt);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _DeviceRow implements DeviceRow {
  const _DeviceRow({@JsonKey(name: 'device_id') required this.deviceId, this.template = '', this.name, this.network, this.address, @JsonKey(name: 'site_id') this.siteId, @JsonKey(name: 'location_id') this.locationId, @JsonKey(name: 'page_id') this.pageId, this.status = '', @JsonKey(name: 'provisioned_at') this.provisionedAt});
  factory _DeviceRow.fromJson(Map<String, dynamic> json) => _$DeviceRowFromJson(json);

@override@JsonKey(name: 'device_id') final  String deviceId;
@override@JsonKey() final  String template;
@override final  String? name;
@override final  String? network;
@override final  String? address;
@override@JsonKey(name: 'site_id') final  String? siteId;
@override@JsonKey(name: 'location_id') final  String? locationId;
@override@JsonKey(name: 'page_id') final  String? pageId;
@override@JsonKey() final  String status;
@override@JsonKey(name: 'provisioned_at') final  String? provisionedAt;

/// Create a copy of DeviceRow
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$DeviceRowCopyWith<_DeviceRow> get copyWith => __$DeviceRowCopyWithImpl<_DeviceRow>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$DeviceRowToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _DeviceRow&&(identical(other.deviceId, deviceId) || other.deviceId == deviceId)&&(identical(other.template, template) || other.template == template)&&(identical(other.name, name) || other.name == name)&&(identical(other.network, network) || other.network == network)&&(identical(other.address, address) || other.address == address)&&(identical(other.siteId, siteId) || other.siteId == siteId)&&(identical(other.locationId, locationId) || other.locationId == locationId)&&(identical(other.pageId, pageId) || other.pageId == pageId)&&(identical(other.status, status) || other.status == status)&&(identical(other.provisionedAt, provisionedAt) || other.provisionedAt == provisionedAt));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,deviceId,template,name,network,address,siteId,locationId,pageId,status,provisionedAt);

@override
String toString() {
  return 'DeviceRow(deviceId: $deviceId, template: $template, name: $name, network: $network, address: $address, siteId: $siteId, locationId: $locationId, pageId: $pageId, status: $status, provisionedAt: $provisionedAt)';
}


}

/// @nodoc
abstract mixin class _$DeviceRowCopyWith<$Res> implements $DeviceRowCopyWith<$Res> {
  factory _$DeviceRowCopyWith(_DeviceRow value, $Res Function(_DeviceRow) _then) = __$DeviceRowCopyWithImpl;
@override @useResult
$Res call({
@JsonKey(name: 'device_id') String deviceId, String template, String? name, String? network, String? address,@JsonKey(name: 'site_id') String? siteId,@JsonKey(name: 'location_id') String? locationId,@JsonKey(name: 'page_id') String? pageId, String status,@JsonKey(name: 'provisioned_at') String? provisionedAt
});




}
/// @nodoc
class __$DeviceRowCopyWithImpl<$Res>
    implements _$DeviceRowCopyWith<$Res> {
  __$DeviceRowCopyWithImpl(this._self, this._then);

  final _DeviceRow _self;
  final $Res Function(_DeviceRow) _then;

/// Create a copy of DeviceRow
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? deviceId = null,Object? template = null,Object? name = freezed,Object? network = freezed,Object? address = freezed,Object? siteId = freezed,Object? locationId = freezed,Object? pageId = freezed,Object? status = null,Object? provisionedAt = freezed,}) {
  return _then(_DeviceRow(
deviceId: null == deviceId ? _self.deviceId : deviceId // ignore: cast_nullable_to_non_nullable
as String,template: null == template ? _self.template : template // ignore: cast_nullable_to_non_nullable
as String,name: freezed == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String?,network: freezed == network ? _self.network : network // ignore: cast_nullable_to_non_nullable
as String?,address: freezed == address ? _self.address : address // ignore: cast_nullable_to_non_nullable
as String?,siteId: freezed == siteId ? _self.siteId : siteId // ignore: cast_nullable_to_non_nullable
as String?,locationId: freezed == locationId ? _self.locationId : locationId // ignore: cast_nullable_to_non_nullable
as String?,pageId: freezed == pageId ? _self.pageId : pageId // ignore: cast_nullable_to_non_nullable
as String?,status: null == status ? _self.status : status // ignore: cast_nullable_to_non_nullable
as String,provisionedAt: freezed == provisionedAt ? _self.provisionedAt : provisionedAt // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}


}


/// @nodoc
mixin _$SiteRow {

@JsonKey(name: 'site_id') String get siteId; String get name;
/// Create a copy of SiteRow
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SiteRowCopyWith<SiteRow> get copyWith => _$SiteRowCopyWithImpl<SiteRow>(this as SiteRow, _$identity);

  /// Serializes this SiteRow to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SiteRow&&(identical(other.siteId, siteId) || other.siteId == siteId)&&(identical(other.name, name) || other.name == name));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,siteId,name);

@override
String toString() {
  return 'SiteRow(siteId: $siteId, name: $name)';
}


}

/// @nodoc
abstract mixin class $SiteRowCopyWith<$Res>  {
  factory $SiteRowCopyWith(SiteRow value, $Res Function(SiteRow) _then) = _$SiteRowCopyWithImpl;
@useResult
$Res call({
@JsonKey(name: 'site_id') String siteId, String name
});




}
/// @nodoc
class _$SiteRowCopyWithImpl<$Res>
    implements $SiteRowCopyWith<$Res> {
  _$SiteRowCopyWithImpl(this._self, this._then);

  final SiteRow _self;
  final $Res Function(SiteRow) _then;

/// Create a copy of SiteRow
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? siteId = null,Object? name = null,}) {
  return _then(_self.copyWith(
siteId: null == siteId ? _self.siteId : siteId // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// Adds pattern-matching-related methods to [SiteRow].
extension SiteRowPatterns on SiteRow {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _SiteRow value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _SiteRow() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _SiteRow value)  $default,){
final _that = this;
switch (_that) {
case _SiteRow():
return $default(_that);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _SiteRow value)?  $default,){
final _that = this;
switch (_that) {
case _SiteRow() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function(@JsonKey(name: 'site_id')  String siteId,  String name)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _SiteRow() when $default != null:
return $default(_that.siteId,_that.name);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function(@JsonKey(name: 'site_id')  String siteId,  String name)  $default,) {final _that = this;
switch (_that) {
case _SiteRow():
return $default(_that.siteId,_that.name);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function(@JsonKey(name: 'site_id')  String siteId,  String name)?  $default,) {final _that = this;
switch (_that) {
case _SiteRow() when $default != null:
return $default(_that.siteId,_that.name);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _SiteRow implements SiteRow {
  const _SiteRow({@JsonKey(name: 'site_id') required this.siteId, this.name = ''});
  factory _SiteRow.fromJson(Map<String, dynamic> json) => _$SiteRowFromJson(json);

@override@JsonKey(name: 'site_id') final  String siteId;
@override@JsonKey() final  String name;

/// Create a copy of SiteRow
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$SiteRowCopyWith<_SiteRow> get copyWith => __$SiteRowCopyWithImpl<_SiteRow>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$SiteRowToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _SiteRow&&(identical(other.siteId, siteId) || other.siteId == siteId)&&(identical(other.name, name) || other.name == name));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,siteId,name);

@override
String toString() {
  return 'SiteRow(siteId: $siteId, name: $name)';
}


}

/// @nodoc
abstract mixin class _$SiteRowCopyWith<$Res> implements $SiteRowCopyWith<$Res> {
  factory _$SiteRowCopyWith(_SiteRow value, $Res Function(_SiteRow) _then) = __$SiteRowCopyWithImpl;
@override @useResult
$Res call({
@JsonKey(name: 'site_id') String siteId, String name
});




}
/// @nodoc
class __$SiteRowCopyWithImpl<$Res>
    implements _$SiteRowCopyWith<$Res> {
  __$SiteRowCopyWithImpl(this._self, this._then);

  final _SiteRow _self;
  final $Res Function(_SiteRow) _then;

/// Create a copy of SiteRow
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? siteId = null,Object? name = null,}) {
  return _then(_SiteRow(
siteId: null == siteId ? _self.siteId : siteId // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}


/// @nodoc
mixin _$LocationRow {

@JsonKey(name: 'location_id') String get locationId;@JsonKey(name: 'site_id') String get siteId; String get name;
/// Create a copy of LocationRow
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LocationRowCopyWith<LocationRow> get copyWith => _$LocationRowCopyWithImpl<LocationRow>(this as LocationRow, _$identity);

  /// Serializes this LocationRow to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LocationRow&&(identical(other.locationId, locationId) || other.locationId == locationId)&&(identical(other.siteId, siteId) || other.siteId == siteId)&&(identical(other.name, name) || other.name == name));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,locationId,siteId,name);

@override
String toString() {
  return 'LocationRow(locationId: $locationId, siteId: $siteId, name: $name)';
}


}

/// @nodoc
abstract mixin class $LocationRowCopyWith<$Res>  {
  factory $LocationRowCopyWith(LocationRow value, $Res Function(LocationRow) _then) = _$LocationRowCopyWithImpl;
@useResult
$Res call({
@JsonKey(name: 'location_id') String locationId,@JsonKey(name: 'site_id') String siteId, String name
});




}
/// @nodoc
class _$LocationRowCopyWithImpl<$Res>
    implements $LocationRowCopyWith<$Res> {
  _$LocationRowCopyWithImpl(this._self, this._then);

  final LocationRow _self;
  final $Res Function(LocationRow) _then;

/// Create a copy of LocationRow
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? locationId = null,Object? siteId = null,Object? name = null,}) {
  return _then(_self.copyWith(
locationId: null == locationId ? _self.locationId : locationId // ignore: cast_nullable_to_non_nullable
as String,siteId: null == siteId ? _self.siteId : siteId // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// Adds pattern-matching-related methods to [LocationRow].
extension LocationRowPatterns on LocationRow {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _LocationRow value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _LocationRow() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _LocationRow value)  $default,){
final _that = this;
switch (_that) {
case _LocationRow():
return $default(_that);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _LocationRow value)?  $default,){
final _that = this;
switch (_that) {
case _LocationRow() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function(@JsonKey(name: 'location_id')  String locationId, @JsonKey(name: 'site_id')  String siteId,  String name)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _LocationRow() when $default != null:
return $default(_that.locationId,_that.siteId,_that.name);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function(@JsonKey(name: 'location_id')  String locationId, @JsonKey(name: 'site_id')  String siteId,  String name)  $default,) {final _that = this;
switch (_that) {
case _LocationRow():
return $default(_that.locationId,_that.siteId,_that.name);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function(@JsonKey(name: 'location_id')  String locationId, @JsonKey(name: 'site_id')  String siteId,  String name)?  $default,) {final _that = this;
switch (_that) {
case _LocationRow() when $default != null:
return $default(_that.locationId,_that.siteId,_that.name);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _LocationRow implements LocationRow {
  const _LocationRow({@JsonKey(name: 'location_id') required this.locationId, @JsonKey(name: 'site_id') this.siteId = '', this.name = ''});
  factory _LocationRow.fromJson(Map<String, dynamic> json) => _$LocationRowFromJson(json);

@override@JsonKey(name: 'location_id') final  String locationId;
@override@JsonKey(name: 'site_id') final  String siteId;
@override@JsonKey() final  String name;

/// Create a copy of LocationRow
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$LocationRowCopyWith<_LocationRow> get copyWith => __$LocationRowCopyWithImpl<_LocationRow>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$LocationRowToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _LocationRow&&(identical(other.locationId, locationId) || other.locationId == locationId)&&(identical(other.siteId, siteId) || other.siteId == siteId)&&(identical(other.name, name) || other.name == name));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,locationId,siteId,name);

@override
String toString() {
  return 'LocationRow(locationId: $locationId, siteId: $siteId, name: $name)';
}


}

/// @nodoc
abstract mixin class _$LocationRowCopyWith<$Res> implements $LocationRowCopyWith<$Res> {
  factory _$LocationRowCopyWith(_LocationRow value, $Res Function(_LocationRow) _then) = __$LocationRowCopyWithImpl;
@override @useResult
$Res call({
@JsonKey(name: 'location_id') String locationId,@JsonKey(name: 'site_id') String siteId, String name
});




}
/// @nodoc
class __$LocationRowCopyWithImpl<$Res>
    implements _$LocationRowCopyWith<$Res> {
  __$LocationRowCopyWithImpl(this._self, this._then);

  final _LocationRow _self;
  final $Res Function(_LocationRow) _then;

/// Create a copy of LocationRow
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? locationId = null,Object? siteId = null,Object? name = null,}) {
  return _then(_LocationRow(
locationId: null == locationId ? _self.locationId : locationId // ignore: cast_nullable_to_non_nullable
as String,siteId: null == siteId ? _self.siteId : siteId // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}


/// @nodoc
mixin _$PageRow {

@JsonKey(name: 'page_id') String get pageId;@JsonKey(name: 'site_id') String? get siteId; String get name;
/// Create a copy of PageRow
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PageRowCopyWith<PageRow> get copyWith => _$PageRowCopyWithImpl<PageRow>(this as PageRow, _$identity);

  /// Serializes this PageRow to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PageRow&&(identical(other.pageId, pageId) || other.pageId == pageId)&&(identical(other.siteId, siteId) || other.siteId == siteId)&&(identical(other.name, name) || other.name == name));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,pageId,siteId,name);

@override
String toString() {
  return 'PageRow(pageId: $pageId, siteId: $siteId, name: $name)';
}


}

/// @nodoc
abstract mixin class $PageRowCopyWith<$Res>  {
  factory $PageRowCopyWith(PageRow value, $Res Function(PageRow) _then) = _$PageRowCopyWithImpl;
@useResult
$Res call({
@JsonKey(name: 'page_id') String pageId,@JsonKey(name: 'site_id') String? siteId, String name
});




}
/// @nodoc
class _$PageRowCopyWithImpl<$Res>
    implements $PageRowCopyWith<$Res> {
  _$PageRowCopyWithImpl(this._self, this._then);

  final PageRow _self;
  final $Res Function(PageRow) _then;

/// Create a copy of PageRow
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? pageId = null,Object? siteId = freezed,Object? name = null,}) {
  return _then(_self.copyWith(
pageId: null == pageId ? _self.pageId : pageId // ignore: cast_nullable_to_non_nullable
as String,siteId: freezed == siteId ? _self.siteId : siteId // ignore: cast_nullable_to_non_nullable
as String?,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// Adds pattern-matching-related methods to [PageRow].
extension PageRowPatterns on PageRow {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _PageRow value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _PageRow() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _PageRow value)  $default,){
final _that = this;
switch (_that) {
case _PageRow():
return $default(_that);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _PageRow value)?  $default,){
final _that = this;
switch (_that) {
case _PageRow() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function(@JsonKey(name: 'page_id')  String pageId, @JsonKey(name: 'site_id')  String? siteId,  String name)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _PageRow() when $default != null:
return $default(_that.pageId,_that.siteId,_that.name);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function(@JsonKey(name: 'page_id')  String pageId, @JsonKey(name: 'site_id')  String? siteId,  String name)  $default,) {final _that = this;
switch (_that) {
case _PageRow():
return $default(_that.pageId,_that.siteId,_that.name);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function(@JsonKey(name: 'page_id')  String pageId, @JsonKey(name: 'site_id')  String? siteId,  String name)?  $default,) {final _that = this;
switch (_that) {
case _PageRow() when $default != null:
return $default(_that.pageId,_that.siteId,_that.name);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _PageRow implements PageRow {
  const _PageRow({@JsonKey(name: 'page_id') required this.pageId, @JsonKey(name: 'site_id') this.siteId, this.name = ''});
  factory _PageRow.fromJson(Map<String, dynamic> json) => _$PageRowFromJson(json);

@override@JsonKey(name: 'page_id') final  String pageId;
@override@JsonKey(name: 'site_id') final  String? siteId;
@override@JsonKey() final  String name;

/// Create a copy of PageRow
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$PageRowCopyWith<_PageRow> get copyWith => __$PageRowCopyWithImpl<_PageRow>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$PageRowToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _PageRow&&(identical(other.pageId, pageId) || other.pageId == pageId)&&(identical(other.siteId, siteId) || other.siteId == siteId)&&(identical(other.name, name) || other.name == name));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,pageId,siteId,name);

@override
String toString() {
  return 'PageRow(pageId: $pageId, siteId: $siteId, name: $name)';
}


}

/// @nodoc
abstract mixin class _$PageRowCopyWith<$Res> implements $PageRowCopyWith<$Res> {
  factory _$PageRowCopyWith(_PageRow value, $Res Function(_PageRow) _then) = __$PageRowCopyWithImpl;
@override @useResult
$Res call({
@JsonKey(name: 'page_id') String pageId,@JsonKey(name: 'site_id') String? siteId, String name
});




}
/// @nodoc
class __$PageRowCopyWithImpl<$Res>
    implements _$PageRowCopyWith<$Res> {
  __$PageRowCopyWithImpl(this._self, this._then);

  final _PageRow _self;
  final $Res Function(_PageRow) _then;

/// Create a copy of PageRow
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? pageId = null,Object? siteId = freezed,Object? name = null,}) {
  return _then(_PageRow(
pageId: null == pageId ? _self.pageId : pageId // ignore: cast_nullable_to_non_nullable
as String,siteId: freezed == siteId ? _self.siteId : siteId // ignore: cast_nullable_to_non_nullable
as String?,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}


/// @nodoc
mixin _$TemplateRow {

 String get template;@JsonKey(fromJson: _stringify) String get version;@JsonKey(name: 'display_name') String get displayName; String get network; String get category; String get icon;
/// Create a copy of TemplateRow
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TemplateRowCopyWith<TemplateRow> get copyWith => _$TemplateRowCopyWithImpl<TemplateRow>(this as TemplateRow, _$identity);

  /// Serializes this TemplateRow to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TemplateRow&&(identical(other.template, template) || other.template == template)&&(identical(other.version, version) || other.version == version)&&(identical(other.displayName, displayName) || other.displayName == displayName)&&(identical(other.network, network) || other.network == network)&&(identical(other.category, category) || other.category == category)&&(identical(other.icon, icon) || other.icon == icon));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,template,version,displayName,network,category,icon);

@override
String toString() {
  return 'TemplateRow(template: $template, version: $version, displayName: $displayName, network: $network, category: $category, icon: $icon)';
}


}

/// @nodoc
abstract mixin class $TemplateRowCopyWith<$Res>  {
  factory $TemplateRowCopyWith(TemplateRow value, $Res Function(TemplateRow) _then) = _$TemplateRowCopyWithImpl;
@useResult
$Res call({
 String template,@JsonKey(fromJson: _stringify) String version,@JsonKey(name: 'display_name') String displayName, String network, String category, String icon
});




}
/// @nodoc
class _$TemplateRowCopyWithImpl<$Res>
    implements $TemplateRowCopyWith<$Res> {
  _$TemplateRowCopyWithImpl(this._self, this._then);

  final TemplateRow _self;
  final $Res Function(TemplateRow) _then;

/// Create a copy of TemplateRow
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? template = null,Object? version = null,Object? displayName = null,Object? network = null,Object? category = null,Object? icon = null,}) {
  return _then(_self.copyWith(
template: null == template ? _self.template : template // ignore: cast_nullable_to_non_nullable
as String,version: null == version ? _self.version : version // ignore: cast_nullable_to_non_nullable
as String,displayName: null == displayName ? _self.displayName : displayName // ignore: cast_nullable_to_non_nullable
as String,network: null == network ? _self.network : network // ignore: cast_nullable_to_non_nullable
as String,category: null == category ? _self.category : category // ignore: cast_nullable_to_non_nullable
as String,icon: null == icon ? _self.icon : icon // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// Adds pattern-matching-related methods to [TemplateRow].
extension TemplateRowPatterns on TemplateRow {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _TemplateRow value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _TemplateRow() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _TemplateRow value)  $default,){
final _that = this;
switch (_that) {
case _TemplateRow():
return $default(_that);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _TemplateRow value)?  $default,){
final _that = this;
switch (_that) {
case _TemplateRow() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String template, @JsonKey(fromJson: _stringify)  String version, @JsonKey(name: 'display_name')  String displayName,  String network,  String category,  String icon)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _TemplateRow() when $default != null:
return $default(_that.template,_that.version,_that.displayName,_that.network,_that.category,_that.icon);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String template, @JsonKey(fromJson: _stringify)  String version, @JsonKey(name: 'display_name')  String displayName,  String network,  String category,  String icon)  $default,) {final _that = this;
switch (_that) {
case _TemplateRow():
return $default(_that.template,_that.version,_that.displayName,_that.network,_that.category,_that.icon);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String template, @JsonKey(fromJson: _stringify)  String version, @JsonKey(name: 'display_name')  String displayName,  String network,  String category,  String icon)?  $default,) {final _that = this;
switch (_that) {
case _TemplateRow() when $default != null:
return $default(_that.template,_that.version,_that.displayName,_that.network,_that.category,_that.icon);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _TemplateRow implements TemplateRow {
  const _TemplateRow({required this.template, @JsonKey(fromJson: _stringify) this.version = '', @JsonKey(name: 'display_name') this.displayName = '', this.network = '', this.category = '', this.icon = ''});
  factory _TemplateRow.fromJson(Map<String, dynamic> json) => _$TemplateRowFromJson(json);

@override final  String template;
@override@JsonKey(fromJson: _stringify) final  String version;
@override@JsonKey(name: 'display_name') final  String displayName;
@override@JsonKey() final  String network;
@override@JsonKey() final  String category;
@override@JsonKey() final  String icon;

/// Create a copy of TemplateRow
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$TemplateRowCopyWith<_TemplateRow> get copyWith => __$TemplateRowCopyWithImpl<_TemplateRow>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$TemplateRowToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _TemplateRow&&(identical(other.template, template) || other.template == template)&&(identical(other.version, version) || other.version == version)&&(identical(other.displayName, displayName) || other.displayName == displayName)&&(identical(other.network, network) || other.network == network)&&(identical(other.category, category) || other.category == category)&&(identical(other.icon, icon) || other.icon == icon));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,template,version,displayName,network,category,icon);

@override
String toString() {
  return 'TemplateRow(template: $template, version: $version, displayName: $displayName, network: $network, category: $category, icon: $icon)';
}


}

/// @nodoc
abstract mixin class _$TemplateRowCopyWith<$Res> implements $TemplateRowCopyWith<$Res> {
  factory _$TemplateRowCopyWith(_TemplateRow value, $Res Function(_TemplateRow) _then) = __$TemplateRowCopyWithImpl;
@override @useResult
$Res call({
 String template,@JsonKey(fromJson: _stringify) String version,@JsonKey(name: 'display_name') String displayName, String network, String category, String icon
});




}
/// @nodoc
class __$TemplateRowCopyWithImpl<$Res>
    implements _$TemplateRowCopyWith<$Res> {
  __$TemplateRowCopyWithImpl(this._self, this._then);

  final _TemplateRow _self;
  final $Res Function(_TemplateRow) _then;

/// Create a copy of TemplateRow
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? template = null,Object? version = null,Object? displayName = null,Object? network = null,Object? category = null,Object? icon = null,}) {
  return _then(_TemplateRow(
template: null == template ? _self.template : template // ignore: cast_nullable_to_non_nullable
as String,version: null == version ? _self.version : version // ignore: cast_nullable_to_non_nullable
as String,displayName: null == displayName ? _self.displayName : displayName // ignore: cast_nullable_to_non_nullable
as String,network: null == network ? _self.network : network // ignore: cast_nullable_to_non_nullable
as String,category: null == category ? _self.category : category // ignore: cast_nullable_to_non_nullable
as String,icon: null == icon ? _self.icon : icon // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}


/// @nodoc
mixin _$TemplateYaml {

 String get template; String get yaml;
/// Create a copy of TemplateYaml
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TemplateYamlCopyWith<TemplateYaml> get copyWith => _$TemplateYamlCopyWithImpl<TemplateYaml>(this as TemplateYaml, _$identity);

  /// Serializes this TemplateYaml to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TemplateYaml&&(identical(other.template, template) || other.template == template)&&(identical(other.yaml, yaml) || other.yaml == yaml));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,template,yaml);

@override
String toString() {
  return 'TemplateYaml(template: $template, yaml: $yaml)';
}


}

/// @nodoc
abstract mixin class $TemplateYamlCopyWith<$Res>  {
  factory $TemplateYamlCopyWith(TemplateYaml value, $Res Function(TemplateYaml) _then) = _$TemplateYamlCopyWithImpl;
@useResult
$Res call({
 String template, String yaml
});




}
/// @nodoc
class _$TemplateYamlCopyWithImpl<$Res>
    implements $TemplateYamlCopyWith<$Res> {
  _$TemplateYamlCopyWithImpl(this._self, this._then);

  final TemplateYaml _self;
  final $Res Function(TemplateYaml) _then;

/// Create a copy of TemplateYaml
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? template = null,Object? yaml = null,}) {
  return _then(_self.copyWith(
template: null == template ? _self.template : template // ignore: cast_nullable_to_non_nullable
as String,yaml: null == yaml ? _self.yaml : yaml // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// Adds pattern-matching-related methods to [TemplateYaml].
extension TemplateYamlPatterns on TemplateYaml {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _TemplateYaml value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _TemplateYaml() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _TemplateYaml value)  $default,){
final _that = this;
switch (_that) {
case _TemplateYaml():
return $default(_that);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _TemplateYaml value)?  $default,){
final _that = this;
switch (_that) {
case _TemplateYaml() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String template,  String yaml)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _TemplateYaml() when $default != null:
return $default(_that.template,_that.yaml);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String template,  String yaml)  $default,) {final _that = this;
switch (_that) {
case _TemplateYaml():
return $default(_that.template,_that.yaml);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String template,  String yaml)?  $default,) {final _that = this;
switch (_that) {
case _TemplateYaml() when $default != null:
return $default(_that.template,_that.yaml);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _TemplateYaml implements TemplateYaml {
  const _TemplateYaml({required this.template, this.yaml = ''});
  factory _TemplateYaml.fromJson(Map<String, dynamic> json) => _$TemplateYamlFromJson(json);

@override final  String template;
@override@JsonKey() final  String yaml;

/// Create a copy of TemplateYaml
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$TemplateYamlCopyWith<_TemplateYaml> get copyWith => __$TemplateYamlCopyWithImpl<_TemplateYaml>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$TemplateYamlToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _TemplateYaml&&(identical(other.template, template) || other.template == template)&&(identical(other.yaml, yaml) || other.yaml == yaml));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,template,yaml);

@override
String toString() {
  return 'TemplateYaml(template: $template, yaml: $yaml)';
}


}

/// @nodoc
abstract mixin class _$TemplateYamlCopyWith<$Res> implements $TemplateYamlCopyWith<$Res> {
  factory _$TemplateYamlCopyWith(_TemplateYaml value, $Res Function(_TemplateYaml) _then) = __$TemplateYamlCopyWithImpl;
@override @useResult
$Res call({
 String template, String yaml
});




}
/// @nodoc
class __$TemplateYamlCopyWithImpl<$Res>
    implements _$TemplateYamlCopyWith<$Res> {
  __$TemplateYamlCopyWithImpl(this._self, this._then);

  final _TemplateYaml _self;
  final $Res Function(_TemplateYaml) _then;

/// Create a copy of TemplateYaml
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? template = null,Object? yaml = null,}) {
  return _then(_TemplateYaml(
template: null == template ? _self.template : template // ignore: cast_nullable_to_non_nullable
as String,yaml: null == yaml ? _self.yaml : yaml // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}


/// @nodoc
mixin _$PointRow {

@JsonKey(name: 'point_id') String get pointId;@JsonKey(name: 'device_id') String get deviceId;@JsonKey(name: 'point_key') String get pointKey; String get name; String? get unit; String get kind; String get widget; bool get writable;@JsonKey(name: 'trend_on') bool get trendOn;@JsonKey(name: 'alarm_on') bool get alarmOn;
/// Create a copy of PointRow
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PointRowCopyWith<PointRow> get copyWith => _$PointRowCopyWithImpl<PointRow>(this as PointRow, _$identity);

  /// Serializes this PointRow to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PointRow&&(identical(other.pointId, pointId) || other.pointId == pointId)&&(identical(other.deviceId, deviceId) || other.deviceId == deviceId)&&(identical(other.pointKey, pointKey) || other.pointKey == pointKey)&&(identical(other.name, name) || other.name == name)&&(identical(other.unit, unit) || other.unit == unit)&&(identical(other.kind, kind) || other.kind == kind)&&(identical(other.widget, widget) || other.widget == widget)&&(identical(other.writable, writable) || other.writable == writable)&&(identical(other.trendOn, trendOn) || other.trendOn == trendOn)&&(identical(other.alarmOn, alarmOn) || other.alarmOn == alarmOn));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,pointId,deviceId,pointKey,name,unit,kind,widget,writable,trendOn,alarmOn);

@override
String toString() {
  return 'PointRow(pointId: $pointId, deviceId: $deviceId, pointKey: $pointKey, name: $name, unit: $unit, kind: $kind, widget: $widget, writable: $writable, trendOn: $trendOn, alarmOn: $alarmOn)';
}


}

/// @nodoc
abstract mixin class $PointRowCopyWith<$Res>  {
  factory $PointRowCopyWith(PointRow value, $Res Function(PointRow) _then) = _$PointRowCopyWithImpl;
@useResult
$Res call({
@JsonKey(name: 'point_id') String pointId,@JsonKey(name: 'device_id') String deviceId,@JsonKey(name: 'point_key') String pointKey, String name, String? unit, String kind, String widget, bool writable,@JsonKey(name: 'trend_on') bool trendOn,@JsonKey(name: 'alarm_on') bool alarmOn
});




}
/// @nodoc
class _$PointRowCopyWithImpl<$Res>
    implements $PointRowCopyWith<$Res> {
  _$PointRowCopyWithImpl(this._self, this._then);

  final PointRow _self;
  final $Res Function(PointRow) _then;

/// Create a copy of PointRow
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? pointId = null,Object? deviceId = null,Object? pointKey = null,Object? name = null,Object? unit = freezed,Object? kind = null,Object? widget = null,Object? writable = null,Object? trendOn = null,Object? alarmOn = null,}) {
  return _then(_self.copyWith(
pointId: null == pointId ? _self.pointId : pointId // ignore: cast_nullable_to_non_nullable
as String,deviceId: null == deviceId ? _self.deviceId : deviceId // ignore: cast_nullable_to_non_nullable
as String,pointKey: null == pointKey ? _self.pointKey : pointKey // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,unit: freezed == unit ? _self.unit : unit // ignore: cast_nullable_to_non_nullable
as String?,kind: null == kind ? _self.kind : kind // ignore: cast_nullable_to_non_nullable
as String,widget: null == widget ? _self.widget : widget // ignore: cast_nullable_to_non_nullable
as String,writable: null == writable ? _self.writable : writable // ignore: cast_nullable_to_non_nullable
as bool,trendOn: null == trendOn ? _self.trendOn : trendOn // ignore: cast_nullable_to_non_nullable
as bool,alarmOn: null == alarmOn ? _self.alarmOn : alarmOn // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}

}


/// Adds pattern-matching-related methods to [PointRow].
extension PointRowPatterns on PointRow {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _PointRow value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _PointRow() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _PointRow value)  $default,){
final _that = this;
switch (_that) {
case _PointRow():
return $default(_that);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _PointRow value)?  $default,){
final _that = this;
switch (_that) {
case _PointRow() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function(@JsonKey(name: 'point_id')  String pointId, @JsonKey(name: 'device_id')  String deviceId, @JsonKey(name: 'point_key')  String pointKey,  String name,  String? unit,  String kind,  String widget,  bool writable, @JsonKey(name: 'trend_on')  bool trendOn, @JsonKey(name: 'alarm_on')  bool alarmOn)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _PointRow() when $default != null:
return $default(_that.pointId,_that.deviceId,_that.pointKey,_that.name,_that.unit,_that.kind,_that.widget,_that.writable,_that.trendOn,_that.alarmOn);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function(@JsonKey(name: 'point_id')  String pointId, @JsonKey(name: 'device_id')  String deviceId, @JsonKey(name: 'point_key')  String pointKey,  String name,  String? unit,  String kind,  String widget,  bool writable, @JsonKey(name: 'trend_on')  bool trendOn, @JsonKey(name: 'alarm_on')  bool alarmOn)  $default,) {final _that = this;
switch (_that) {
case _PointRow():
return $default(_that.pointId,_that.deviceId,_that.pointKey,_that.name,_that.unit,_that.kind,_that.widget,_that.writable,_that.trendOn,_that.alarmOn);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function(@JsonKey(name: 'point_id')  String pointId, @JsonKey(name: 'device_id')  String deviceId, @JsonKey(name: 'point_key')  String pointKey,  String name,  String? unit,  String kind,  String widget,  bool writable, @JsonKey(name: 'trend_on')  bool trendOn, @JsonKey(name: 'alarm_on')  bool alarmOn)?  $default,) {final _that = this;
switch (_that) {
case _PointRow() when $default != null:
return $default(_that.pointId,_that.deviceId,_that.pointKey,_that.name,_that.unit,_that.kind,_that.widget,_that.writable,_that.trendOn,_that.alarmOn);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _PointRow implements PointRow {
  const _PointRow({@JsonKey(name: 'point_id') required this.pointId, @JsonKey(name: 'device_id') this.deviceId = '', @JsonKey(name: 'point_key') this.pointKey = '', this.name = '', this.unit, this.kind = '', this.widget = 'stat', this.writable = false, @JsonKey(name: 'trend_on') this.trendOn = false, @JsonKey(name: 'alarm_on') this.alarmOn = false});
  factory _PointRow.fromJson(Map<String, dynamic> json) => _$PointRowFromJson(json);

@override@JsonKey(name: 'point_id') final  String pointId;
@override@JsonKey(name: 'device_id') final  String deviceId;
@override@JsonKey(name: 'point_key') final  String pointKey;
@override@JsonKey() final  String name;
@override final  String? unit;
@override@JsonKey() final  String kind;
@override@JsonKey() final  String widget;
@override@JsonKey() final  bool writable;
@override@JsonKey(name: 'trend_on') final  bool trendOn;
@override@JsonKey(name: 'alarm_on') final  bool alarmOn;

/// Create a copy of PointRow
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$PointRowCopyWith<_PointRow> get copyWith => __$PointRowCopyWithImpl<_PointRow>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$PointRowToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _PointRow&&(identical(other.pointId, pointId) || other.pointId == pointId)&&(identical(other.deviceId, deviceId) || other.deviceId == deviceId)&&(identical(other.pointKey, pointKey) || other.pointKey == pointKey)&&(identical(other.name, name) || other.name == name)&&(identical(other.unit, unit) || other.unit == unit)&&(identical(other.kind, kind) || other.kind == kind)&&(identical(other.widget, widget) || other.widget == widget)&&(identical(other.writable, writable) || other.writable == writable)&&(identical(other.trendOn, trendOn) || other.trendOn == trendOn)&&(identical(other.alarmOn, alarmOn) || other.alarmOn == alarmOn));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,pointId,deviceId,pointKey,name,unit,kind,widget,writable,trendOn,alarmOn);

@override
String toString() {
  return 'PointRow(pointId: $pointId, deviceId: $deviceId, pointKey: $pointKey, name: $name, unit: $unit, kind: $kind, widget: $widget, writable: $writable, trendOn: $trendOn, alarmOn: $alarmOn)';
}


}

/// @nodoc
abstract mixin class _$PointRowCopyWith<$Res> implements $PointRowCopyWith<$Res> {
  factory _$PointRowCopyWith(_PointRow value, $Res Function(_PointRow) _then) = __$PointRowCopyWithImpl;
@override @useResult
$Res call({
@JsonKey(name: 'point_id') String pointId,@JsonKey(name: 'device_id') String deviceId,@JsonKey(name: 'point_key') String pointKey, String name, String? unit, String kind, String widget, bool writable,@JsonKey(name: 'trend_on') bool trendOn,@JsonKey(name: 'alarm_on') bool alarmOn
});




}
/// @nodoc
class __$PointRowCopyWithImpl<$Res>
    implements _$PointRowCopyWith<$Res> {
  __$PointRowCopyWithImpl(this._self, this._then);

  final _PointRow _self;
  final $Res Function(_PointRow) _then;

/// Create a copy of PointRow
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? pointId = null,Object? deviceId = null,Object? pointKey = null,Object? name = null,Object? unit = freezed,Object? kind = null,Object? widget = null,Object? writable = null,Object? trendOn = null,Object? alarmOn = null,}) {
  return _then(_PointRow(
pointId: null == pointId ? _self.pointId : pointId // ignore: cast_nullable_to_non_nullable
as String,deviceId: null == deviceId ? _self.deviceId : deviceId // ignore: cast_nullable_to_non_nullable
as String,pointKey: null == pointKey ? _self.pointKey : pointKey // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,unit: freezed == unit ? _self.unit : unit // ignore: cast_nullable_to_non_nullable
as String?,kind: null == kind ? _self.kind : kind // ignore: cast_nullable_to_non_nullable
as String,widget: null == widget ? _self.widget : widget // ignore: cast_nullable_to_non_nullable
as String,writable: null == writable ? _self.writable : writable // ignore: cast_nullable_to_non_nullable
as bool,trendOn: null == trendOn ? _self.trendOn : trendOn // ignore: cast_nullable_to_non_nullable
as bool,alarmOn: null == alarmOn ? _self.alarmOn : alarmOn // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}


/// @nodoc
mixin _$WidgetRow {

@JsonKey(name: 'widget_id') String get widgetId;@JsonKey(name: 'page_id') String get pageId;@JsonKey(name: 'device_id') String get deviceId;@JsonKey(name: 'point_id') String? get pointId; String get widget; String? get role; String? get title;
/// Create a copy of WidgetRow
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WidgetRowCopyWith<WidgetRow> get copyWith => _$WidgetRowCopyWithImpl<WidgetRow>(this as WidgetRow, _$identity);

  /// Serializes this WidgetRow to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WidgetRow&&(identical(other.widgetId, widgetId) || other.widgetId == widgetId)&&(identical(other.pageId, pageId) || other.pageId == pageId)&&(identical(other.deviceId, deviceId) || other.deviceId == deviceId)&&(identical(other.pointId, pointId) || other.pointId == pointId)&&(identical(other.widget, widget) || other.widget == widget)&&(identical(other.role, role) || other.role == role)&&(identical(other.title, title) || other.title == title));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,widgetId,pageId,deviceId,pointId,widget,role,title);

@override
String toString() {
  return 'WidgetRow(widgetId: $widgetId, pageId: $pageId, deviceId: $deviceId, pointId: $pointId, widget: $widget, role: $role, title: $title)';
}


}

/// @nodoc
abstract mixin class $WidgetRowCopyWith<$Res>  {
  factory $WidgetRowCopyWith(WidgetRow value, $Res Function(WidgetRow) _then) = _$WidgetRowCopyWithImpl;
@useResult
$Res call({
@JsonKey(name: 'widget_id') String widgetId,@JsonKey(name: 'page_id') String pageId,@JsonKey(name: 'device_id') String deviceId,@JsonKey(name: 'point_id') String? pointId, String widget, String? role, String? title
});




}
/// @nodoc
class _$WidgetRowCopyWithImpl<$Res>
    implements $WidgetRowCopyWith<$Res> {
  _$WidgetRowCopyWithImpl(this._self, this._then);

  final WidgetRow _self;
  final $Res Function(WidgetRow) _then;

/// Create a copy of WidgetRow
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? widgetId = null,Object? pageId = null,Object? deviceId = null,Object? pointId = freezed,Object? widget = null,Object? role = freezed,Object? title = freezed,}) {
  return _then(_self.copyWith(
widgetId: null == widgetId ? _self.widgetId : widgetId // ignore: cast_nullable_to_non_nullable
as String,pageId: null == pageId ? _self.pageId : pageId // ignore: cast_nullable_to_non_nullable
as String,deviceId: null == deviceId ? _self.deviceId : deviceId // ignore: cast_nullable_to_non_nullable
as String,pointId: freezed == pointId ? _self.pointId : pointId // ignore: cast_nullable_to_non_nullable
as String?,widget: null == widget ? _self.widget : widget // ignore: cast_nullable_to_non_nullable
as String,role: freezed == role ? _self.role : role // ignore: cast_nullable_to_non_nullable
as String?,title: freezed == title ? _self.title : title // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}

}


/// Adds pattern-matching-related methods to [WidgetRow].
extension WidgetRowPatterns on WidgetRow {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _WidgetRow value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _WidgetRow() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _WidgetRow value)  $default,){
final _that = this;
switch (_that) {
case _WidgetRow():
return $default(_that);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _WidgetRow value)?  $default,){
final _that = this;
switch (_that) {
case _WidgetRow() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function(@JsonKey(name: 'widget_id')  String widgetId, @JsonKey(name: 'page_id')  String pageId, @JsonKey(name: 'device_id')  String deviceId, @JsonKey(name: 'point_id')  String? pointId,  String widget,  String? role,  String? title)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _WidgetRow() when $default != null:
return $default(_that.widgetId,_that.pageId,_that.deviceId,_that.pointId,_that.widget,_that.role,_that.title);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function(@JsonKey(name: 'widget_id')  String widgetId, @JsonKey(name: 'page_id')  String pageId, @JsonKey(name: 'device_id')  String deviceId, @JsonKey(name: 'point_id')  String? pointId,  String widget,  String? role,  String? title)  $default,) {final _that = this;
switch (_that) {
case _WidgetRow():
return $default(_that.widgetId,_that.pageId,_that.deviceId,_that.pointId,_that.widget,_that.role,_that.title);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function(@JsonKey(name: 'widget_id')  String widgetId, @JsonKey(name: 'page_id')  String pageId, @JsonKey(name: 'device_id')  String deviceId, @JsonKey(name: 'point_id')  String? pointId,  String widget,  String? role,  String? title)?  $default,) {final _that = this;
switch (_that) {
case _WidgetRow() when $default != null:
return $default(_that.widgetId,_that.pageId,_that.deviceId,_that.pointId,_that.widget,_that.role,_that.title);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _WidgetRow implements WidgetRow {
  const _WidgetRow({@JsonKey(name: 'widget_id') required this.widgetId, @JsonKey(name: 'page_id') this.pageId = '', @JsonKey(name: 'device_id') this.deviceId = '', @JsonKey(name: 'point_id') this.pointId, this.widget = 'stat', this.role, this.title});
  factory _WidgetRow.fromJson(Map<String, dynamic> json) => _$WidgetRowFromJson(json);

@override@JsonKey(name: 'widget_id') final  String widgetId;
@override@JsonKey(name: 'page_id') final  String pageId;
@override@JsonKey(name: 'device_id') final  String deviceId;
@override@JsonKey(name: 'point_id') final  String? pointId;
@override@JsonKey() final  String widget;
@override final  String? role;
@override final  String? title;

/// Create a copy of WidgetRow
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$WidgetRowCopyWith<_WidgetRow> get copyWith => __$WidgetRowCopyWithImpl<_WidgetRow>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$WidgetRowToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _WidgetRow&&(identical(other.widgetId, widgetId) || other.widgetId == widgetId)&&(identical(other.pageId, pageId) || other.pageId == pageId)&&(identical(other.deviceId, deviceId) || other.deviceId == deviceId)&&(identical(other.pointId, pointId) || other.pointId == pointId)&&(identical(other.widget, widget) || other.widget == widget)&&(identical(other.role, role) || other.role == role)&&(identical(other.title, title) || other.title == title));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,widgetId,pageId,deviceId,pointId,widget,role,title);

@override
String toString() {
  return 'WidgetRow(widgetId: $widgetId, pageId: $pageId, deviceId: $deviceId, pointId: $pointId, widget: $widget, role: $role, title: $title)';
}


}

/// @nodoc
abstract mixin class _$WidgetRowCopyWith<$Res> implements $WidgetRowCopyWith<$Res> {
  factory _$WidgetRowCopyWith(_WidgetRow value, $Res Function(_WidgetRow) _then) = __$WidgetRowCopyWithImpl;
@override @useResult
$Res call({
@JsonKey(name: 'widget_id') String widgetId,@JsonKey(name: 'page_id') String pageId,@JsonKey(name: 'device_id') String deviceId,@JsonKey(name: 'point_id') String? pointId, String widget, String? role, String? title
});




}
/// @nodoc
class __$WidgetRowCopyWithImpl<$Res>
    implements _$WidgetRowCopyWith<$Res> {
  __$WidgetRowCopyWithImpl(this._self, this._then);

  final _WidgetRow _self;
  final $Res Function(_WidgetRow) _then;

/// Create a copy of WidgetRow
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? widgetId = null,Object? pageId = null,Object? deviceId = null,Object? pointId = freezed,Object? widget = null,Object? role = freezed,Object? title = freezed,}) {
  return _then(_WidgetRow(
widgetId: null == widgetId ? _self.widgetId : widgetId // ignore: cast_nullable_to_non_nullable
as String,pageId: null == pageId ? _self.pageId : pageId // ignore: cast_nullable_to_non_nullable
as String,deviceId: null == deviceId ? _self.deviceId : deviceId // ignore: cast_nullable_to_non_nullable
as String,pointId: freezed == pointId ? _self.pointId : pointId // ignore: cast_nullable_to_non_nullable
as String?,widget: null == widget ? _self.widget : widget // ignore: cast_nullable_to_non_nullable
as String,role: freezed == role ? _self.role : role // ignore: cast_nullable_to_non_nullable
as String?,title: freezed == title ? _self.title : title // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}


}


/// @nodoc
mixin _$LabelRender {

@JsonKey(name: 'device_id') String get deviceId; String get serial;@JsonKey(name: 'qr_url') String get qrUrl; String get code128;@JsonKey(name: 'display_name') String get displayName;
/// Create a copy of LabelRender
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LabelRenderCopyWith<LabelRender> get copyWith => _$LabelRenderCopyWithImpl<LabelRender>(this as LabelRender, _$identity);

  /// Serializes this LabelRender to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LabelRender&&(identical(other.deviceId, deviceId) || other.deviceId == deviceId)&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.qrUrl, qrUrl) || other.qrUrl == qrUrl)&&(identical(other.code128, code128) || other.code128 == code128)&&(identical(other.displayName, displayName) || other.displayName == displayName));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,deviceId,serial,qrUrl,code128,displayName);

@override
String toString() {
  return 'LabelRender(deviceId: $deviceId, serial: $serial, qrUrl: $qrUrl, code128: $code128, displayName: $displayName)';
}


}

/// @nodoc
abstract mixin class $LabelRenderCopyWith<$Res>  {
  factory $LabelRenderCopyWith(LabelRender value, $Res Function(LabelRender) _then) = _$LabelRenderCopyWithImpl;
@useResult
$Res call({
@JsonKey(name: 'device_id') String deviceId, String serial,@JsonKey(name: 'qr_url') String qrUrl, String code128,@JsonKey(name: 'display_name') String displayName
});




}
/// @nodoc
class _$LabelRenderCopyWithImpl<$Res>
    implements $LabelRenderCopyWith<$Res> {
  _$LabelRenderCopyWithImpl(this._self, this._then);

  final LabelRender _self;
  final $Res Function(LabelRender) _then;

/// Create a copy of LabelRender
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? deviceId = null,Object? serial = null,Object? qrUrl = null,Object? code128 = null,Object? displayName = null,}) {
  return _then(_self.copyWith(
deviceId: null == deviceId ? _self.deviceId : deviceId // ignore: cast_nullable_to_non_nullable
as String,serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,qrUrl: null == qrUrl ? _self.qrUrl : qrUrl // ignore: cast_nullable_to_non_nullable
as String,code128: null == code128 ? _self.code128 : code128 // ignore: cast_nullable_to_non_nullable
as String,displayName: null == displayName ? _self.displayName : displayName // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// Adds pattern-matching-related methods to [LabelRender].
extension LabelRenderPatterns on LabelRender {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _LabelRender value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _LabelRender() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _LabelRender value)  $default,){
final _that = this;
switch (_that) {
case _LabelRender():
return $default(_that);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _LabelRender value)?  $default,){
final _that = this;
switch (_that) {
case _LabelRender() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function(@JsonKey(name: 'device_id')  String deviceId,  String serial, @JsonKey(name: 'qr_url')  String qrUrl,  String code128, @JsonKey(name: 'display_name')  String displayName)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _LabelRender() when $default != null:
return $default(_that.deviceId,_that.serial,_that.qrUrl,_that.code128,_that.displayName);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function(@JsonKey(name: 'device_id')  String deviceId,  String serial, @JsonKey(name: 'qr_url')  String qrUrl,  String code128, @JsonKey(name: 'display_name')  String displayName)  $default,) {final _that = this;
switch (_that) {
case _LabelRender():
return $default(_that.deviceId,_that.serial,_that.qrUrl,_that.code128,_that.displayName);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function(@JsonKey(name: 'device_id')  String deviceId,  String serial, @JsonKey(name: 'qr_url')  String qrUrl,  String code128, @JsonKey(name: 'display_name')  String displayName)?  $default,) {final _that = this;
switch (_that) {
case _LabelRender() when $default != null:
return $default(_that.deviceId,_that.serial,_that.qrUrl,_that.code128,_that.displayName);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _LabelRender implements LabelRender {
  const _LabelRender({@JsonKey(name: 'device_id') required this.deviceId, this.serial = '', @JsonKey(name: 'qr_url') this.qrUrl = '', this.code128 = '', @JsonKey(name: 'display_name') this.displayName = ''});
  factory _LabelRender.fromJson(Map<String, dynamic> json) => _$LabelRenderFromJson(json);

@override@JsonKey(name: 'device_id') final  String deviceId;
@override@JsonKey() final  String serial;
@override@JsonKey(name: 'qr_url') final  String qrUrl;
@override@JsonKey() final  String code128;
@override@JsonKey(name: 'display_name') final  String displayName;

/// Create a copy of LabelRender
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$LabelRenderCopyWith<_LabelRender> get copyWith => __$LabelRenderCopyWithImpl<_LabelRender>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$LabelRenderToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _LabelRender&&(identical(other.deviceId, deviceId) || other.deviceId == deviceId)&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.qrUrl, qrUrl) || other.qrUrl == qrUrl)&&(identical(other.code128, code128) || other.code128 == code128)&&(identical(other.displayName, displayName) || other.displayName == displayName));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,deviceId,serial,qrUrl,code128,displayName);

@override
String toString() {
  return 'LabelRender(deviceId: $deviceId, serial: $serial, qrUrl: $qrUrl, code128: $code128, displayName: $displayName)';
}


}

/// @nodoc
abstract mixin class _$LabelRenderCopyWith<$Res> implements $LabelRenderCopyWith<$Res> {
  factory _$LabelRenderCopyWith(_LabelRender value, $Res Function(_LabelRender) _then) = __$LabelRenderCopyWithImpl;
@override @useResult
$Res call({
@JsonKey(name: 'device_id') String deviceId, String serial,@JsonKey(name: 'qr_url') String qrUrl, String code128,@JsonKey(name: 'display_name') String displayName
});




}
/// @nodoc
class __$LabelRenderCopyWithImpl<$Res>
    implements _$LabelRenderCopyWith<$Res> {
  __$LabelRenderCopyWithImpl(this._self, this._then);

  final _LabelRender _self;
  final $Res Function(_LabelRender) _then;

/// Create a copy of LabelRender
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? deviceId = null,Object? serial = null,Object? qrUrl = null,Object? code128 = null,Object? displayName = null,}) {
  return _then(_LabelRender(
deviceId: null == deviceId ? _self.deviceId : deviceId // ignore: cast_nullable_to_non_nullable
as String,serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,qrUrl: null == qrUrl ? _self.qrUrl : qrUrl // ignore: cast_nullable_to_non_nullable
as String,code128: null == code128 ? _self.code128 : code128 // ignore: cast_nullable_to_non_nullable
as String,displayName: null == displayName ? _self.displayName : displayName // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
