//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'missing_tenant_id_response.g.dart';

/// Body returned when `tenant_id` is required (no tenants store wired) and the client omitted it.
///
/// Properties:
/// * [error] - Always `\"missing_tenant_id\"`.
@BuiltValue()
abstract class MissingTenantIdResponse implements Built<MissingTenantIdResponse, MissingTenantIdResponseBuilder> {
  /// Always `\"missing_tenant_id\"`.
  @BuiltValueField(wireName: r'error')
  String get error;

  MissingTenantIdResponse._();

  factory MissingTenantIdResponse([void updates(MissingTenantIdResponseBuilder b)]) = _$MissingTenantIdResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(MissingTenantIdResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<MissingTenantIdResponse> get serializer => _$MissingTenantIdResponseSerializer();
}

class _$MissingTenantIdResponseSerializer implements PrimitiveSerializer<MissingTenantIdResponse> {
  @override
  final Iterable<Type> types = const [MissingTenantIdResponse, _$MissingTenantIdResponse];

  @override
  final String wireName = r'MissingTenantIdResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    MissingTenantIdResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'error';
    yield serializers.serialize(
      object.error,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    MissingTenantIdResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required MissingTenantIdResponseBuilder result,
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
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  MissingTenantIdResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = MissingTenantIdResponseBuilder();
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

