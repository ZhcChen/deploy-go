//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'runtime_probe_batch_request.g.dart';

/// RuntimeProbeBatchRequest
///
/// Properties:
/// * [applicationIds]
@BuiltValue()
abstract class RuntimeProbeBatchRequest implements Built<RuntimeProbeBatchRequest, RuntimeProbeBatchRequestBuilder> {
  @BuiltValueField(wireName: r'application_ids')
  BuiltList<String> get applicationIds;

  RuntimeProbeBatchRequest._();

  factory RuntimeProbeBatchRequest([void updates(RuntimeProbeBatchRequestBuilder b)]) = _$RuntimeProbeBatchRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RuntimeProbeBatchRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RuntimeProbeBatchRequest> get serializer => _$RuntimeProbeBatchRequestSerializer();
}

class _$RuntimeProbeBatchRequestSerializer implements PrimitiveSerializer<RuntimeProbeBatchRequest> {
  @override
  final Iterable<Type> types = const [RuntimeProbeBatchRequest, _$RuntimeProbeBatchRequest];

  @override
  final String wireName = r'RuntimeProbeBatchRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RuntimeProbeBatchRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'application_ids';
    yield serializers.serialize(
      object.applicationIds,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RuntimeProbeBatchRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required RuntimeProbeBatchRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'application_ids':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.applicationIds.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RuntimeProbeBatchRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RuntimeProbeBatchRequestBuilder();
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
