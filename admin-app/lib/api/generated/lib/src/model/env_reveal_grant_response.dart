//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:deploy_go_api_client/src/model/env_grant_action.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'env_reveal_grant_response.g.dart';

/// EnvRevealGrantResponse
///
/// Properties:
/// * [action]
/// * [expiresAt]
/// * [grantToken]
@BuiltValue()
abstract class EnvRevealGrantResponse implements Built<EnvRevealGrantResponse, EnvRevealGrantResponseBuilder> {
  @BuiltValueField(wireName: r'action')
  EnvGrantAction get action;
  // enum actionEnum {  read_write,  delete,  };

  @BuiltValueField(wireName: r'expires_at')
  String get expiresAt;

  @BuiltValueField(wireName: r'grant_token')
  String get grantToken;

  EnvRevealGrantResponse._();

  factory EnvRevealGrantResponse([void updates(EnvRevealGrantResponseBuilder b)]) = _$EnvRevealGrantResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(EnvRevealGrantResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<EnvRevealGrantResponse> get serializer => _$EnvRevealGrantResponseSerializer();
}

class _$EnvRevealGrantResponseSerializer implements PrimitiveSerializer<EnvRevealGrantResponse> {
  @override
  final Iterable<Type> types = const [EnvRevealGrantResponse, _$EnvRevealGrantResponse];

  @override
  final String wireName = r'EnvRevealGrantResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    EnvRevealGrantResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'action';
    yield serializers.serialize(
      object.action,
      specifiedType: const FullType(EnvGrantAction),
    );
    yield r'expires_at';
    yield serializers.serialize(
      object.expiresAt,
      specifiedType: const FullType(String),
    );
    yield r'grant_token';
    yield serializers.serialize(
      object.grantToken,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    EnvRevealGrantResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required EnvRevealGrantResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'action':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(EnvGrantAction),
          ) as EnvGrantAction;
          result.action = valueDes;
          break;
        case r'expires_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.expiresAt = valueDes;
          break;
        case r'grant_token':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.grantToken = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  EnvRevealGrantResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = EnvRevealGrantResponseBuilder();
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
