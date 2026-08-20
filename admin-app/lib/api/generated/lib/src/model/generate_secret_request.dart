//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'generate_secret_request.g.dart';

/// GenerateSecretRequest
///
/// Properties:
/// * [bytes]
/// * [expectedVersion]
/// * [key]
@BuiltValue()
abstract class GenerateSecretRequest implements Built<GenerateSecretRequest, GenerateSecretRequestBuilder> {
  @BuiltValueField(wireName: r'bytes')
  int? get bytes;

  @BuiltValueField(wireName: r'expected_version')
  int get expectedVersion;

  @BuiltValueField(wireName: r'key')
  String get key;

  GenerateSecretRequest._();

  factory GenerateSecretRequest([void updates(GenerateSecretRequestBuilder b)]) = _$GenerateSecretRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(GenerateSecretRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<GenerateSecretRequest> get serializer => _$GenerateSecretRequestSerializer();
}

class _$GenerateSecretRequestSerializer implements PrimitiveSerializer<GenerateSecretRequest> {
  @override
  final Iterable<Type> types = const [GenerateSecretRequest, _$GenerateSecretRequest];

  @override
  final String wireName = r'GenerateSecretRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    GenerateSecretRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.bytes != null) {
      yield r'bytes';
      yield serializers.serialize(
        object.bytes,
        specifiedType: const FullType.nullable(int),
      );
    }
    yield r'expected_version';
    yield serializers.serialize(
      object.expectedVersion,
      specifiedType: const FullType(int),
    );
    yield r'key';
    yield serializers.serialize(
      object.key,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    GenerateSecretRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required GenerateSecretRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'bytes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.bytes = valueDes;
          break;
        case r'expected_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.expectedVersion = valueDes;
          break;
        case r'key':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.key = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  GenerateSecretRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = GenerateSecretRequestBuilder();
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
