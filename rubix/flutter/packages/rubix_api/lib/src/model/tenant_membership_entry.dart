//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'tenant_membership_entry.g.dart';

/// One membership the user could pick on retry.
///
/// Properties:
/// * [role] - User's role within that tenant (`reader | writer | admin`).
/// * [tenantId] - Tenant id to echo back in `TokenRequest.tenant_id`.
@BuiltValue()
abstract class TenantMembershipEntry implements Built<TenantMembershipEntry, TenantMembershipEntryBuilder> {
  /// User's role within that tenant (`reader | writer | admin`).
  @BuiltValueField(wireName: r'role')
  String get role;

  /// Tenant id to echo back in `TokenRequest.tenant_id`.
  @BuiltValueField(wireName: r'tenant_id')
  String get tenantId;

  TenantMembershipEntry._();

  factory TenantMembershipEntry([void updates(TenantMembershipEntryBuilder b)]) = _$TenantMembershipEntry;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(TenantMembershipEntryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<TenantMembershipEntry> get serializer => _$TenantMembershipEntrySerializer();
}

class _$TenantMembershipEntrySerializer implements PrimitiveSerializer<TenantMembershipEntry> {
  @override
  final Iterable<Type> types = const [TenantMembershipEntry, _$TenantMembershipEntry];

  @override
  final String wireName = r'TenantMembershipEntry';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    TenantMembershipEntry object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'role';
    yield serializers.serialize(
      object.role,
      specifiedType: const FullType(String),
    );
    yield r'tenant_id';
    yield serializers.serialize(
      object.tenantId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    TenantMembershipEntry object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required TenantMembershipEntryBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'role':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.role = valueDes;
          break;
        case r'tenant_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.tenantId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  TenantMembershipEntry deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = TenantMembershipEntryBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}

