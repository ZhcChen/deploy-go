//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'runtime_log_entry_schema.g.dart';

/// RuntimeLogEntrySchema
///
/// Properties:
/// * [fields]
/// * [level]
/// * [message]
/// * [requestId]
/// * [sequence]
/// * [target]
/// * [timestamp]
@BuiltValue()
abstract class RuntimeLogEntrySchema implements Built<RuntimeLogEntrySchema, RuntimeLogEntrySchemaBuilder> {
  @BuiltValueField(wireName: r'fields')
  BuiltMap<String, JsonObject?> get fields;

  @BuiltValueField(wireName: r'level')
  String get level;

  @BuiltValueField(wireName: r'message')
  String get message;

  @BuiltValueField(wireName: r'request_id')
  String? get requestId;

  @BuiltValueField(wireName: r'sequence')
  int get sequence;

  @BuiltValueField(wireName: r'target')
  String get target;

  @BuiltValueField(wireName: r'timestamp')
  String get timestamp;

  RuntimeLogEntrySchema._();

  factory RuntimeLogEntrySchema([void updates(RuntimeLogEntrySchemaBuilder b)]) = _$RuntimeLogEntrySchema;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RuntimeLogEntrySchemaBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RuntimeLogEntrySchema> get serializer => _$RuntimeLogEntrySchemaSerializer();
}

class _$RuntimeLogEntrySchemaSerializer implements PrimitiveSerializer<RuntimeLogEntrySchema> {
  @override
  final Iterable<Type> types = const [RuntimeLogEntrySchema, _$RuntimeLogEntrySchema];

  @override
  final String wireName = r'RuntimeLogEntrySchema';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RuntimeLogEntrySchema object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'fields';
    yield serializers.serialize(
      object.fields,
      specifiedType: const FullType(BuiltMap, [FullType(String), FullType.nullable(JsonObject)]),
    );
    yield r'level';
    yield serializers.serialize(
      object.level,
      specifiedType: const FullType(String),
    );
    yield r'message';
    yield serializers.serialize(
      object.message,
      specifiedType: const FullType(String),
    );
    if (object.requestId != null) {
      yield r'request_id';
      yield serializers.serialize(
        object.requestId,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'sequence';
    yield serializers.serialize(
      object.sequence,
      specifiedType: const FullType(int),
    );
    yield r'target';
    yield serializers.serialize(
      object.target,
      specifiedType: const FullType(String),
    );
    yield r'timestamp';
    yield serializers.serialize(
      object.timestamp,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RuntimeLogEntrySchema object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required RuntimeLogEntrySchemaBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'fields':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltMap, [FullType(String), FullType.nullable(JsonObject)]),
          ) as BuiltMap<String, JsonObject?>;
          result.fields.replace(valueDes);
          break;
        case r'level':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.level = valueDes;
          break;
        case r'message':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.message = valueDes;
          break;
        case r'request_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.requestId = valueDes;
          break;
        case r'sequence':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.sequence = valueDes;
          break;
        case r'target':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.target = valueDes;
          break;
        case r'timestamp':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.timestamp = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RuntimeLogEntrySchema deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RuntimeLogEntrySchemaBuilder();
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
