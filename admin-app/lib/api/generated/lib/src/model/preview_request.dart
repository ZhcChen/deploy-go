//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'preview_request.g.dart';

/// PreviewRequest
///
/// Properties:
/// * [parameters]
@BuiltValue()
abstract class PreviewRequest implements Built<PreviewRequest, PreviewRequestBuilder> {
  @BuiltValueField(wireName: r'parameters')
  JsonObject? get parameters;

  PreviewRequest._();

  factory PreviewRequest([void updates(PreviewRequestBuilder b)]) = _$PreviewRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(PreviewRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<PreviewRequest> get serializer => _$PreviewRequestSerializer();
}

class _$PreviewRequestSerializer implements PrimitiveSerializer<PreviewRequest> {
  @override
  final Iterable<Type> types = const [PreviewRequest, _$PreviewRequest];

  @override
  final String wireName = r'PreviewRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    PreviewRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'parameters';
    yield object.parameters == null ? null : serializers.serialize(
      object.parameters,
      specifiedType: const FullType.nullable(JsonObject),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    PreviewRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required PreviewRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'parameters':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.parameters = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  PreviewRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = PreviewRequestBuilder();
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
