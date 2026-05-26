//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'token_response.g.dart';

/// Successful response body for `POST /auth/token`.
///
/// Properties:
/// * [expiresAt] - Absolute UTC expiry (RFC3339). Advisory in v1 — clients react to 401 rather than pre-emptively refreshing.
/// * [token] - The plaintext bearer (`sak_<id>.<secret>`). Shown once; the server stores only the argon2id hash of the secret.
/// * [tokenType] - Always `\"Bearer\"`. Reserved for the future refresh-token flow.
@BuiltValue()
abstract class TokenResponse implements Built<TokenResponse, TokenResponseBuilder> {
  /// Absolute UTC expiry (RFC3339). Advisory in v1 — clients react to 401 rather than pre-emptively refreshing.
  @BuiltValueField(wireName: r'expires_at')
  DateTime get expiresAt;

  /// The plaintext bearer (`sak_<id>.<secret>`). Shown once; the server stores only the argon2id hash of the secret.
  @BuiltValueField(wireName: r'token')
  String get token;

  /// Always `\"Bearer\"`. Reserved for the future refresh-token flow.
  @BuiltValueField(wireName: r'token_type')
  String get tokenType;

  TokenResponse._();

  factory TokenResponse([void updates(TokenResponseBuilder b)]) = _$TokenResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(TokenResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<TokenResponse> get serializer => _$TokenResponseSerializer();
}

class _$TokenResponseSerializer implements PrimitiveSerializer<TokenResponse> {
  @override
  final Iterable<Type> types = const [TokenResponse, _$TokenResponse];

  @override
  final String wireName = r'TokenResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    TokenResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'expires_at';
    yield serializers.serialize(
      object.expiresAt,
      specifiedType: const FullType(DateTime),
    );
    yield r'token';
    yield serializers.serialize(
      object.token,
      specifiedType: const FullType(String),
    );
    yield r'token_type';
    yield serializers.serialize(
      object.tokenType,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    TokenResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required TokenResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'expires_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.expiresAt = valueDes;
          break;
        case r'token':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.token = valueDes;
          break;
        case r'token_type':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.tokenType = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  TokenResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = TokenResponseBuilder();
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

