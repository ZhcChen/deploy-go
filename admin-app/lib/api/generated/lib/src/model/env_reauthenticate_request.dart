//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:deploy_go_api_client/src/model/env_grant_action.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'env_reauthenticate_request.g.dart';

/// EnvReauthenticateRequest
///
/// Properties:
/// * [action]
/// * [password]
@BuiltValue()
abstract class EnvReauthenticateRequest implements Built<EnvReauthenticateRequest, EnvReauthenticateRequestBuilder> {
  @BuiltValueField(wireName: r'action')
  EnvGrantAction get action;
  // enum actionEnum {  read_write,  delete,  };

  @BuiltValueField(wireName: r'password')
  String get password;

  EnvReauthenticateRequest._();

  factory EnvReauthenticateRequest([void updates(EnvReauthenticateRequestBuilder b)]) = _$EnvReauthenticateRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(EnvReauthenticateRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<EnvReauthenticateRequest> get serializer => _$EnvReauthenticateRequestSerializer();
}

class _$EnvReauthenticateRequestSerializer implements PrimitiveSerializer<EnvReauthenticateRequest> {
  @override
  final Iterable<Type> types = const [EnvReauthenticateRequest, _$EnvReauthenticateRequest];

  @override
  final String wireName = r'EnvReauthenticateRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    EnvReauthenticateRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'action';
    yield serializers.serialize(
      object.action,
      specifiedType: const FullType(EnvGrantAction),
    );
    yield r'password';
    yield serializers.serialize(
      object.password,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    EnvReauthenticateRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required EnvReauthenticateRequestBuilder result,
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
        case r'password':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.password = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  EnvReauthenticateRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = EnvReauthenticateRequestBuilder();
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
