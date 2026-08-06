//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/register_application_env_content.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'register_application_envs_request.g.dart';

/// RegisterApplicationEnvsRequest
///
/// Properties:
/// * [files]
/// * [manifestJson]
@BuiltValue()
abstract class RegisterApplicationEnvsRequest implements Built<RegisterApplicationEnvsRequest, RegisterApplicationEnvsRequestBuilder> {
  @BuiltValueField(wireName: r'files')
  BuiltList<RegisterApplicationEnvContent> get files;

  @BuiltValueField(wireName: r'manifest_json')
  String get manifestJson;

  RegisterApplicationEnvsRequest._();

  factory RegisterApplicationEnvsRequest([void updates(RegisterApplicationEnvsRequestBuilder b)]) = _$RegisterApplicationEnvsRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RegisterApplicationEnvsRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RegisterApplicationEnvsRequest> get serializer => _$RegisterApplicationEnvsRequestSerializer();
}

class _$RegisterApplicationEnvsRequestSerializer implements PrimitiveSerializer<RegisterApplicationEnvsRequest> {
  @override
  final Iterable<Type> types = const [RegisterApplicationEnvsRequest, _$RegisterApplicationEnvsRequest];

  @override
  final String wireName = r'RegisterApplicationEnvsRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RegisterApplicationEnvsRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'files';
    yield serializers.serialize(
      object.files,
      specifiedType: const FullType(BuiltList, [FullType(RegisterApplicationEnvContent)]),
    );
    yield r'manifest_json';
    yield serializers.serialize(
      object.manifestJson,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RegisterApplicationEnvsRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required RegisterApplicationEnvsRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'files':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(RegisterApplicationEnvContent)]),
          ) as BuiltList<RegisterApplicationEnvContent>;
          result.files.replace(valueDes);
          break;
        case r'manifest_json':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.manifestJson = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RegisterApplicationEnvsRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RegisterApplicationEnvsRequestBuilder();
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
