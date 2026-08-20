//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:deploy_go_api_client/src/model/application_config_file_response.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'generate_secret_response.g.dart';

/// GenerateSecretResponse
///
/// Properties:
/// * [file]
/// * [key]
/// * [secret]
@BuiltValue()
abstract class GenerateSecretResponse implements Built<GenerateSecretResponse, GenerateSecretResponseBuilder> {
  @BuiltValueField(wireName: r'file')
  ApplicationConfigFileResponse get file;

  @BuiltValueField(wireName: r'key')
  String get key;

  @BuiltValueField(wireName: r'secret')
  String get secret;

  GenerateSecretResponse._();

  factory GenerateSecretResponse([void updates(GenerateSecretResponseBuilder b)]) = _$GenerateSecretResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(GenerateSecretResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<GenerateSecretResponse> get serializer => _$GenerateSecretResponseSerializer();
}

class _$GenerateSecretResponseSerializer implements PrimitiveSerializer<GenerateSecretResponse> {
  @override
  final Iterable<Type> types = const [GenerateSecretResponse, _$GenerateSecretResponse];

  @override
  final String wireName = r'GenerateSecretResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    GenerateSecretResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'file';
    yield serializers.serialize(
      object.file,
      specifiedType: const FullType(ApplicationConfigFileResponse),
    );
    yield r'key';
    yield serializers.serialize(
      object.key,
      specifiedType: const FullType(String),
    );
    yield r'secret';
    yield serializers.serialize(
      object.secret,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    GenerateSecretResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required GenerateSecretResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'file':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(ApplicationConfigFileResponse),
          ) as ApplicationConfigFileResponse;
          result.file.replace(valueDes);
          break;
        case r'key':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.key = valueDes;
          break;
        case r'secret':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.secret = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  GenerateSecretResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = GenerateSecretResponseBuilder();
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
