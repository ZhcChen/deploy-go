//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'token_pair_response.g.dart';

/// TokenPairResponse
///
/// Properties:
/// * [accessExpiresAt]
/// * [accessToken]
/// * [agentId]
/// * [refreshExpiresAt]
/// * [refreshToken]
@BuiltValue()
abstract class TokenPairResponse implements Built<TokenPairResponse, TokenPairResponseBuilder> {
  @BuiltValueField(wireName: r'access_expires_at')
  String get accessExpiresAt;

  @BuiltValueField(wireName: r'access_token')
  String get accessToken;

  @BuiltValueField(wireName: r'agent_id')
  String get agentId;

  @BuiltValueField(wireName: r'refresh_expires_at')
  String get refreshExpiresAt;

  @BuiltValueField(wireName: r'refresh_token')
  String get refreshToken;

  TokenPairResponse._();

  factory TokenPairResponse([void updates(TokenPairResponseBuilder b)]) = _$TokenPairResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(TokenPairResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<TokenPairResponse> get serializer => _$TokenPairResponseSerializer();
}

class _$TokenPairResponseSerializer implements PrimitiveSerializer<TokenPairResponse> {
  @override
  final Iterable<Type> types = const [TokenPairResponse, _$TokenPairResponse];

  @override
  final String wireName = r'TokenPairResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    TokenPairResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'access_expires_at';
    yield serializers.serialize(
      object.accessExpiresAt,
      specifiedType: const FullType(String),
    );
    yield r'access_token';
    yield serializers.serialize(
      object.accessToken,
      specifiedType: const FullType(String),
    );
    yield r'agent_id';
    yield serializers.serialize(
      object.agentId,
      specifiedType: const FullType(String),
    );
    yield r'refresh_expires_at';
    yield serializers.serialize(
      object.refreshExpiresAt,
      specifiedType: const FullType(String),
    );
    yield r'refresh_token';
    yield serializers.serialize(
      object.refreshToken,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    TokenPairResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required TokenPairResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'access_expires_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.accessExpiresAt = valueDes;
          break;
        case r'access_token':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.accessToken = valueDes;
          break;
        case r'agent_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.agentId = valueDes;
          break;
        case r'refresh_expires_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.refreshExpiresAt = valueDes;
          break;
        case r'refresh_token':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.refreshToken = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  TokenPairResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = TokenPairResponseBuilder();
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
