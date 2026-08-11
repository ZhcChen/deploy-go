//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:deploy_go_api_client/src/model/register_admin_application_env_content.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'register_admin_application_envs_request.g.dart';

/// RegisterAdminApplicationEnvsRequest
///
/// Properties:
/// * [files]
@BuiltValue()
abstract class RegisterAdminApplicationEnvsRequest implements Built<RegisterAdminApplicationEnvsRequest, RegisterAdminApplicationEnvsRequestBuilder> {
  @BuiltValueField(wireName: r'files')
  BuiltList<RegisterAdminApplicationEnvContent> get files;

  RegisterAdminApplicationEnvsRequest._();

  factory RegisterAdminApplicationEnvsRequest([void updates(RegisterAdminApplicationEnvsRequestBuilder b)]) = _$RegisterAdminApplicationEnvsRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RegisterAdminApplicationEnvsRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RegisterAdminApplicationEnvsRequest> get serializer => _$RegisterAdminApplicationEnvsRequestSerializer();
}

class _$RegisterAdminApplicationEnvsRequestSerializer implements PrimitiveSerializer<RegisterAdminApplicationEnvsRequest> {
  @override
  final Iterable<Type> types = const [RegisterAdminApplicationEnvsRequest, _$RegisterAdminApplicationEnvsRequest];

  @override
  final String wireName = r'RegisterAdminApplicationEnvsRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RegisterAdminApplicationEnvsRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'files';
    yield serializers.serialize(
      object.files,
      specifiedType: const FullType(BuiltList, [FullType(RegisterAdminApplicationEnvContent)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RegisterAdminApplicationEnvsRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required RegisterAdminApplicationEnvsRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'files':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(RegisterAdminApplicationEnvContent)]),
          ) as BuiltList<RegisterAdminApplicationEnvContent>;
          result.files.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RegisterAdminApplicationEnvsRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RegisterAdminApplicationEnvsRequestBuilder();
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
