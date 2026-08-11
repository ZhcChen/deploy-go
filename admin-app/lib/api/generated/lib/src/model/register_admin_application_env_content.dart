//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'register_admin_application_env_content.g.dart';

/// RegisterAdminApplicationEnvContent
///
/// Properties:
/// * [content]
/// * [fileName]
/// * [format]
/// * [module]
@BuiltValue()
abstract class RegisterAdminApplicationEnvContent implements Built<RegisterAdminApplicationEnvContent, RegisterAdminApplicationEnvContentBuilder> {
  @BuiltValueField(wireName: r'content')
  String get content;

  @BuiltValueField(wireName: r'file_name')
  String get fileName;

  @BuiltValueField(wireName: r'format')
  String get format;

  @BuiltValueField(wireName: r'module')
  String get module;

  RegisterAdminApplicationEnvContent._();

  factory RegisterAdminApplicationEnvContent([void updates(RegisterAdminApplicationEnvContentBuilder b)]) = _$RegisterAdminApplicationEnvContent;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RegisterAdminApplicationEnvContentBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RegisterAdminApplicationEnvContent> get serializer => _$RegisterAdminApplicationEnvContentSerializer();
}

class _$RegisterAdminApplicationEnvContentSerializer implements PrimitiveSerializer<RegisterAdminApplicationEnvContent> {
  @override
  final Iterable<Type> types = const [RegisterAdminApplicationEnvContent, _$RegisterAdminApplicationEnvContent];

  @override
  final String wireName = r'RegisterAdminApplicationEnvContent';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RegisterAdminApplicationEnvContent object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'content';
    yield serializers.serialize(
      object.content,
      specifiedType: const FullType(String),
    );
    yield r'file_name';
    yield serializers.serialize(
      object.fileName,
      specifiedType: const FullType(String),
    );
    yield r'format';
    yield serializers.serialize(
      object.format,
      specifiedType: const FullType(String),
    );
    yield r'module';
    yield serializers.serialize(
      object.module,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RegisterAdminApplicationEnvContent object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required RegisterAdminApplicationEnvContentBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'content':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.content = valueDes;
          break;
        case r'file_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.fileName = valueDes;
          break;
        case r'format':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.format = valueDes;
          break;
        case r'module':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.module = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RegisterAdminApplicationEnvContent deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RegisterAdminApplicationEnvContentBuilder();
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
