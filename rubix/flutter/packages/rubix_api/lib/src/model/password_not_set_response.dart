//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'password_not_set_response.g.dart';

/// Body returned by `POST /auth/login` when the matched user has no local password (`password_hash IS NULL`). The SPA reads `providers` to render \"Sign in with GitHub / Google\" buttons without a guess-and-check round trip.
///
/// Properties:
/// * [error] - Always `\"password_not_set\"`. Discriminator field; lets clients pattern-match without inspecting the HTTP status alone.
/// * [providers] - Provider ids the user has linked. Empty list when no third-party path is configured (the default [`crate::NoLinkedProviders`] impl).
@BuiltValue()
abstract class PasswordNotSetResponse implements Built<PasswordNotSetResponse, PasswordNotSetResponseBuilder> {
  /// Always `\"password_not_set\"`. Discriminator field; lets clients pattern-match without inspecting the HTTP status alone.
  @BuiltValueField(wireName: r'error')
  String get error;

  /// Provider ids the user has linked. Empty list when no third-party path is configured (the default [`crate::NoLinkedProviders`] impl).
  @BuiltValueField(wireName: r'providers')
  BuiltList<String> get providers;

  PasswordNotSetResponse._();

  factory PasswordNotSetResponse([void updates(PasswordNotSetResponseBuilder b)]) = _$PasswordNotSetResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(PasswordNotSetResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<PasswordNotSetResponse> get serializer => _$PasswordNotSetResponseSerializer();
}

class _$PasswordNotSetResponseSerializer implements PrimitiveSerializer<PasswordNotSetResponse> {
  @override
  final Iterable<Type> types = const [PasswordNotSetResponse, _$PasswordNotSetResponse];

  @override
  final String wireName = r'PasswordNotSetResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    PasswordNotSetResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'error';
    yield serializers.serialize(
      object.error,
      specifiedType: const FullType(String),
    );
    yield r'providers';
    yield serializers.serialize(
      object.providers,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    PasswordNotSetResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required PasswordNotSetResponseBuilder result,
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
        case r'providers':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.providers.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  PasswordNotSetResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = PasswordNotSetResponseBuilder();
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

