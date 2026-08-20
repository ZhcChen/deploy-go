//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:deploy_go_api_client/src/model/config_grant_action.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'config_reauthenticate_request.g.dart';

/// ConfigReauthenticateRequest
///
/// Properties:
/// * [action]
/// * [password]
@BuiltValue()
abstract class ConfigReauthenticateRequest implements Built<ConfigReauthenticateRequest, ConfigReauthenticateRequestBuilder> {
  @BuiltValueField(wireName: r'action')
  ConfigGrantAction? get action;
  // enum actionEnum {  read_write,  };

  @BuiltValueField(wireName: r'password')
  String get password;

  ConfigReauthenticateRequest._();

  factory ConfigReauthenticateRequest([void updates(ConfigReauthenticateRequestBuilder b)]) = _$ConfigReauthenticateRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ConfigReauthenticateRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ConfigReauthenticateRequest> get serializer => _$ConfigReauthenticateRequestSerializer();
}

class _$ConfigReauthenticateRequestSerializer implements PrimitiveSerializer<ConfigReauthenticateRequest> {
  @override
  final Iterable<Type> types = const [ConfigReauthenticateRequest, _$ConfigReauthenticateRequest];

  @override
  final String wireName = r'ConfigReauthenticateRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ConfigReauthenticateRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.action != null) {
      yield r'action';
      yield serializers.serialize(
        object.action,
        specifiedType: const FullType(ConfigGrantAction),
      );
    }
    yield r'password';
    yield serializers.serialize(
      object.password,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ConfigReauthenticateRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ConfigReauthenticateRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'action':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(ConfigGrantAction),
          ) as ConfigGrantAction?;
          if (valueDes == null) continue;
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
  ConfigReauthenticateRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ConfigReauthenticateRequestBuilder();
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
