//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'runtime_log_batch_response.g.dart';

/// RuntimeLogBatchResponse
///
/// Properties:
/// * [acknowledgedSequence]
/// * [epoch]
@BuiltValue()
abstract class RuntimeLogBatchResponse implements Built<RuntimeLogBatchResponse, RuntimeLogBatchResponseBuilder> {
  @BuiltValueField(wireName: r'acknowledged_sequence')
  int get acknowledgedSequence;

  @BuiltValueField(wireName: r'epoch')
  String get epoch;

  RuntimeLogBatchResponse._();

  factory RuntimeLogBatchResponse([void updates(RuntimeLogBatchResponseBuilder b)]) = _$RuntimeLogBatchResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RuntimeLogBatchResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RuntimeLogBatchResponse> get serializer => _$RuntimeLogBatchResponseSerializer();
}

class _$RuntimeLogBatchResponseSerializer implements PrimitiveSerializer<RuntimeLogBatchResponse> {
  @override
  final Iterable<Type> types = const [RuntimeLogBatchResponse, _$RuntimeLogBatchResponse];

  @override
  final String wireName = r'RuntimeLogBatchResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RuntimeLogBatchResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'acknowledged_sequence';
    yield serializers.serialize(
      object.acknowledgedSequence,
      specifiedType: const FullType(int),
    );
    yield r'epoch';
    yield serializers.serialize(
      object.epoch,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RuntimeLogBatchResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required RuntimeLogBatchResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'acknowledged_sequence':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.acknowledgedSequence = valueDes;
          break;
        case r'epoch':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.epoch = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RuntimeLogBatchResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RuntimeLogBatchResponseBuilder();
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
