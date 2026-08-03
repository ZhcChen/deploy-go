//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'refresh_token_pair_response.g.dart';

/// RefreshTokenPairResponse
///
/// Properties:
/// * [accessExpiresAt]
/// * [accessToken]
/// * [agentId]
/// * [refreshExpiresAt]
/// * [refreshToken]
/// * [rotationId]
@BuiltValue()
abstract class RefreshTokenPairResponse implements Built<RefreshTokenPairResponse, RefreshTokenPairResponseBuilder> {
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

  @BuiltValueField(wireName: r'rotation_id')
  String get rotationId;

  RefreshTokenPairResponse._();

  factory RefreshTokenPairResponse([void updates(RefreshTokenPairResponseBuilder b)]) = _$RefreshTokenPairResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RefreshTokenPairResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RefreshTokenPairResponse> get serializer => _$RefreshTokenPairResponseSerializer();
}

class _$RefreshTokenPairResponseSerializer implements PrimitiveSerializer<RefreshTokenPairResponse> {
  @override
  final Iterable<Type> types = const [RefreshTokenPairResponse, _$RefreshTokenPairResponse];

  @override
  final String wireName = r'RefreshTokenPairResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RefreshTokenPairResponse object, {
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
    yield r'rotation_id';
    yield serializers.serialize(
      object.rotationId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RefreshTokenPairResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required RefreshTokenPairResponseBuilder result,
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
        case r'rotation_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.rotationId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RefreshTokenPairResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RefreshTokenPairResponseBuilder();
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
