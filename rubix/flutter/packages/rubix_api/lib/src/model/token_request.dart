//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'token_request.g.dart';

/// Request body for `POST /auth/token`.
///
/// Properties:
/// * [email] - User's email — same identifier as `POST /auth/login`.
/// * [password] - Plaintext password.
/// * [tenantId] - Optional tenant binding. When omitted, the route resolves the tenant from the user's memberships (requires [`AuthState::with_tenants`]). See design doc §payload.
@BuiltValue()
abstract class TokenRequest implements Built<TokenRequest, TokenRequestBuilder> {
  /// User's email — same identifier as `POST /auth/login`.
  @BuiltValueField(wireName: r'email')
  String get email;

  /// Plaintext password.
  @BuiltValueField(wireName: r'password')
  String get password;

  /// Optional tenant binding. When omitted, the route resolves the tenant from the user's memberships (requires [`AuthState::with_tenants`]). See design doc §payload.
  @BuiltValueField(wireName: r'tenant_id')
  String? get tenantId;

  TokenRequest._();

  factory TokenRequest([void updates(TokenRequestBuilder b)]) = _$TokenRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(TokenRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<TokenRequest> get serializer => _$TokenRequestSerializer();
}

class _$TokenRequestSerializer implements PrimitiveSerializer<TokenRequest> {
  @override
  final Iterable<Type> types = const [TokenRequest, _$TokenRequest];

  @override
  final String wireName = r'TokenRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    TokenRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'email';
    yield serializers.serialize(
      object.email,
      specifiedType: const FullType(String),
    );
    yield r'password';
    yield serializers.serialize(
      object.password,
      specifiedType: const FullType(String),
    );
    if (object.tenantId != null) {
      yield r'tenant_id';
      yield serializers.serialize(
        object.tenantId,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    TokenRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required TokenRequestBuilder result,
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
        case r'password':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.password = valueDes;
          break;
        case r'tenant_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
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
  TokenRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = TokenRequestBuilder();
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

