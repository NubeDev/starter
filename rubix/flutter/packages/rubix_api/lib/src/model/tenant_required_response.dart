//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:rubix_api/src/model/tenant_membership_entry.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'tenant_required_response.g.dart';

/// Body returned when the route cannot disambiguate the user's tenant — multiple memberships exist and the client did not pass `tenant_id`. The client re-POSTs with `tenant_id` set to one of the entries below.
///
/// Properties:
/// * [error] - Always `\"tenant_required\"`. Discriminator string.
/// * [memberships] - One entry per membership row for the authenticated user.
@BuiltValue()
abstract class TenantRequiredResponse implements Built<TenantRequiredResponse, TenantRequiredResponseBuilder> {
  /// Always `\"tenant_required\"`. Discriminator string.
  @BuiltValueField(wireName: r'error')
  String get error;

  /// One entry per membership row for the authenticated user.
  @BuiltValueField(wireName: r'memberships')
  BuiltList<TenantMembershipEntry> get memberships;

  TenantRequiredResponse._();

  factory TenantRequiredResponse([void updates(TenantRequiredResponseBuilder b)]) = _$TenantRequiredResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(TenantRequiredResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<TenantRequiredResponse> get serializer => _$TenantRequiredResponseSerializer();
}

class _$TenantRequiredResponseSerializer implements PrimitiveSerializer<TenantRequiredResponse> {
  @override
  final Iterable<Type> types = const [TenantRequiredResponse, _$TenantRequiredResponse];

  @override
  final String wireName = r'TenantRequiredResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    TenantRequiredResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'error';
    yield serializers.serialize(
      object.error,
      specifiedType: const FullType(String),
    );
    yield r'memberships';
    yield serializers.serialize(
      object.memberships,
      specifiedType: const FullType(BuiltList, [FullType(TenantMembershipEntry)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    TenantRequiredResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required TenantRequiredResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'error':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.error = valueDes;
          break;
        case r'memberships':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(TenantMembershipEntry)]),
          ) as BuiltList<TenantMembershipEntry>;
          result.memberships.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  TenantRequiredResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = TenantRequiredResponseBuilder();
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

