//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:rubix_api/src/model/role.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'me_response.g.dart';

/// Response body for `GET /auth/me`.
///
/// Properties:
/// * [email] - User's email.
/// * [role] - Role: reader / writer / admin.
/// * [subject] - Stable user identifier (the `Principal.subject`).
@BuiltValue()
abstract class MeResponse implements Built<MeResponse, MeResponseBuilder> {
  /// User's email.
  @BuiltValueField(wireName: r'email')
  String get email;

  /// Role: reader / writer / admin.
  @BuiltValueField(wireName: r'role')
  Role get role;
  // enum roleEnum {  reader,  writer,  admin,  };

  /// Stable user identifier (the `Principal.subject`).
  @BuiltValueField(wireName: r'subject')
  String get subject;

  MeResponse._();

  factory MeResponse([void updates(MeResponseBuilder b)]) = _$MeResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(MeResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<MeResponse> get serializer => _$MeResponseSerializer();
}

class _$MeResponseSerializer implements PrimitiveSerializer<MeResponse> {
  @override
  final Iterable<Type> types = const [MeResponse, _$MeResponse];

  @override
  final String wireName = r'MeResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    MeResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'email';
    yield serializers.serialize(
      object.email,
      specifiedType: const FullType(String),
    );
    yield r'role';
    yield serializers.serialize(
      object.role,
      specifiedType: const FullType(Role),
    );
    yield r'subject';
    yield serializers.serialize(
      object.subject,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    MeResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required MeResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'email':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.email = valueDes;
          break;
        case r'role':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(Role),
          ) as Role;
          result.role = valueDes;
          break;
        case r'subject':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.subject = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  MeResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = MeResponseBuilder();
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

