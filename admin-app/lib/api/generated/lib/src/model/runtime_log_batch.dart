//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/runtime_log_entry_schema.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'runtime_log_batch.g.dart';

/// RuntimeLogBatch
///
/// Properties:
/// * [after]
/// * [component]
/// * [entries]
/// * [epoch]
/// * [evictedBytes]
/// * [minimum]
@BuiltValue()
abstract class RuntimeLogBatch implements Built<RuntimeLogBatch, RuntimeLogBatchBuilder> {
  @BuiltValueField(wireName: r'after')
  int get after;

  @BuiltValueField(wireName: r'component')
  String get component;

  @BuiltValueField(wireName: r'entries')
  BuiltList<RuntimeLogEntrySchema> get entries;

  @BuiltValueField(wireName: r'epoch')
  String get epoch;

  @BuiltValueField(wireName: r'evicted_bytes')
  int get evictedBytes;

  @BuiltValueField(wireName: r'minimum')
  int get minimum;

  RuntimeLogBatch._();

  factory RuntimeLogBatch([void updates(RuntimeLogBatchBuilder b)]) = _$RuntimeLogBatch;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RuntimeLogBatchBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RuntimeLogBatch> get serializer => _$RuntimeLogBatchSerializer();
}

class _$RuntimeLogBatchSerializer implements PrimitiveSerializer<RuntimeLogBatch> {
  @override
  final Iterable<Type> types = const [RuntimeLogBatch, _$RuntimeLogBatch];

  @override
  final String wireName = r'RuntimeLogBatch';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RuntimeLogBatch object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'after';
    yield serializers.serialize(
      object.after,
      specifiedType: const FullType(int),
    );
    yield r'component';
    yield serializers.serialize(
      object.component,
      specifiedType: const FullType(String),
    );
    yield r'entries';
    yield serializers.serialize(
      object.entries,
      specifiedType: const FullType(BuiltList, [FullType(RuntimeLogEntrySchema)]),
    );
    yield r'epoch';
    yield serializers.serialize(
      object.epoch,
      specifiedType: const FullType(String),
    );
    yield r'evicted_bytes';
    yield serializers.serialize(
      object.evictedBytes,
      specifiedType: const FullType(int),
    );
    yield r'minimum';
    yield serializers.serialize(
      object.minimum,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RuntimeLogBatch object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required RuntimeLogBatchBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'after':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.after = valueDes;
          break;
        case r'component':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.component = valueDes;
          break;
        case r'entries':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(RuntimeLogEntrySchema)]),
          ) as BuiltList<RuntimeLogEntrySchema>;
          result.entries.replace(valueDes);
          break;
        case r'epoch':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.epoch = valueDes;
          break;
        case r'evicted_bytes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.evictedBytes = valueDes;
          break;
        case r'minimum':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.minimum = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RuntimeLogBatch deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RuntimeLogBatchBuilder();
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
