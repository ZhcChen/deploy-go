//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'controlled_patch_request.g.dart';

/// ControlledPatchRequest
///
/// Properties:
/// * [expectedVersion]
/// * [key]
/// * [value]
@BuiltValue()
abstract class ControlledPatchRequest implements Built<ControlledPatchRequest, ControlledPatchRequestBuilder> {
  @BuiltValueField(wireName: r'expected_version')
  int get expectedVersion;

  @BuiltValueField(wireName: r'key')
  String get key;

  @BuiltValueField(wireName: r'value')
  String get value;

  ControlledPatchRequest._();

  factory ControlledPatchRequest([void updates(ControlledPatchRequestBuilder b)]) = _$ControlledPatchRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ControlledPatchRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ControlledPatchRequest> get serializer => _$ControlledPatchRequestSerializer();
}

class _$ControlledPatchRequestSerializer implements PrimitiveSerializer<ControlledPatchRequest> {
  @override
  final Iterable<Type> types = const [ControlledPatchRequest, _$ControlledPatchRequest];

  @override
  final String wireName = r'ControlledPatchRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ControlledPatchRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
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
    yield r'value';
    yield serializers.serialize(
      object.value,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ControlledPatchRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ControlledPatchRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
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
        case r'value':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.value = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ControlledPatchRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ControlledPatchRequestBuilder();
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
