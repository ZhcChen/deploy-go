//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'register_application_env_content.g.dart';

/// RegisterApplicationEnvContent
///
/// Properties:
/// * [contentBase64]
/// * [fileName]
@BuiltValue()
abstract class RegisterApplicationEnvContent implements Built<RegisterApplicationEnvContent, RegisterApplicationEnvContentBuilder> {
  @BuiltValueField(wireName: r'content_base64')
  String get contentBase64;

  @BuiltValueField(wireName: r'file_name')
  String get fileName;

  RegisterApplicationEnvContent._();

  factory RegisterApplicationEnvContent([void updates(RegisterApplicationEnvContentBuilder b)]) = _$RegisterApplicationEnvContent;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RegisterApplicationEnvContentBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RegisterApplicationEnvContent> get serializer => _$RegisterApplicationEnvContentSerializer();
}

class _$RegisterApplicationEnvContentSerializer implements PrimitiveSerializer<RegisterApplicationEnvContent> {
  @override
  final Iterable<Type> types = const [RegisterApplicationEnvContent, _$RegisterApplicationEnvContent];

  @override
  final String wireName = r'RegisterApplicationEnvContent';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RegisterApplicationEnvContent object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'content_base64';
    yield serializers.serialize(
      object.contentBase64,
      specifiedType: const FullType(String),
    );
    yield r'file_name';
    yield serializers.serialize(
      object.fileName,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RegisterApplicationEnvContent object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required RegisterApplicationEnvContentBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'content_base64':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.contentBase64 = valueDes;
          break;
        case r'file_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.fileName = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RegisterApplicationEnvContent deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RegisterApplicationEnvContentBuilder();
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
