//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'role.g.dart';

class Role extends EnumClass {

  /// Coarse permission level. Three roles are enough for the common case; consumers needing more wire their own `Authenticator` and translate to/from this set at the boundary.
  @BuiltValueEnumConst(wireName: r'reader')
  static const Role reader = _$reader;
  /// Coarse permission level. Three roles are enough for the common case; consumers needing more wire their own `Authenticator` and translate to/from this set at the boundary.
  @BuiltValueEnumConst(wireName: r'writer')
  static const Role writer = _$writer;
  /// Coarse permission level. Three roles are enough for the common case; consumers needing more wire their own `Authenticator` and translate to/from this set at the boundary.
  @BuiltValueEnumConst(wireName: r'admin')
  static const Role admin = _$admin;

  static Serializer<Role> get serializer => _$roleSerializer;

  const Role._(String name): super(name);

  static BuiltSet<Role> get values => _$values;
  static Role valueOf(String name) => _$valueOf(name);
}

/// Optionally, enum_class can generate a mixin to go with your enum for use
/// with Angular. It exposes your enum constants as getters. So, if you mix it
/// in to your Dart component class, the values become available to the
/// corresponding Angular template.
///
/// Trigger mixin generation by writing a line like this one next to your enum.
abstract class RoleMixin = Object with _$RoleMixin;

