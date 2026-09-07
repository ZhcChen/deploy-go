//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/runtime_probe_item_response.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'runtime_probe_batch_response.g.dart';

/// RuntimeProbeBatchResponse
///
/// Properties:
/// * [items]
@BuiltValue()
abstract class RuntimeProbeBatchResponse implements Built<RuntimeProbeBatchResponse, RuntimeProbeBatchResponseBuilder> {
  @BuiltValueField(wireName: r'items')
  BuiltList<RuntimeProbeItemResponse> get items;

  RuntimeProbeBatchResponse._();

  factory RuntimeProbeBatchResponse([void updates(RuntimeProbeBatchResponseBuilder b)]) = _$RuntimeProbeBatchResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RuntimeProbeBatchResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RuntimeProbeBatchResponse> get serializer => _$RuntimeProbeBatchResponseSerializer();
}

class _$RuntimeProbeBatchResponseSerializer implements PrimitiveSerializer<RuntimeProbeBatchResponse> {
  @override
  final Iterable<Type> types = const [RuntimeProbeBatchResponse, _$RuntimeProbeBatchResponse];

  @override
  final String wireName = r'RuntimeProbeBatchResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RuntimeProbeBatchResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'items';
    yield serializers.serialize(
      object.items,
      specifiedType: const FullType(BuiltList, [FullType(RuntimeProbeItemResponse)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RuntimeProbeBatchResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required RuntimeProbeBatchResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'items':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(RuntimeProbeItemResponse)]),
          ) as BuiltList<RuntimeProbeItemResponse>;
          result.items.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RuntimeProbeBatchResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RuntimeProbeBatchResponseBuilder();
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
