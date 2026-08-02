//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'secret_file_reference.g.dart';

/// SecretFileReference
///
/// Properties:
/// * [environmentKey]
/// * [filePath]
@BuiltValue()
abstract class SecretFileReference implements Built<SecretFileReference, SecretFileReferenceBuilder> {
  @BuiltValueField(wireName: r'environment_key')
  String get environmentKey;

  @BuiltValueField(wireName: r'file_path')
  String get filePath;

  SecretFileReference._();

  factory SecretFileReference([void updates(SecretFileReferenceBuilder b)]) = _$SecretFileReference;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SecretFileReferenceBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SecretFileReference> get serializer => _$SecretFileReferenceSerializer();
}

class _$SecretFileReferenceSerializer implements PrimitiveSerializer<SecretFileReference> {
  @override
  final Iterable<Type> types = const [SecretFileReference, _$SecretFileReference];

  @override
  final String wireName = r'SecretFileReference';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SecretFileReference object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'environment_key';
    yield serializers.serialize(
      object.environmentKey,
      specifiedType: const FullType(String),
    );
    yield r'file_path';
    yield serializers.serialize(
      object.filePath,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SecretFileReference object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required SecretFileReferenceBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'environment_key':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.environmentKey = valueDes;
          break;
        case r'file_path':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.filePath = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SecretFileReference deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SecretFileReferenceBuilder();
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
