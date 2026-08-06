//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'deployment_log_response.g.dart';

/// DeploymentLogResponse
///
/// Properties:
/// * [content]
/// * [createdAt]
/// * [sequence]
/// * [stage]
/// * [stream]
/// * [taskId]
/// * [taskSequence]
/// * [truncated]
@BuiltValue()
abstract class DeploymentLogResponse implements Built<DeploymentLogResponse, DeploymentLogResponseBuilder> {
  @BuiltValueField(wireName: r'content')
  String get content;

  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  @BuiltValueField(wireName: r'sequence')
  int get sequence;

  @BuiltValueField(wireName: r'stage')
  String? get stage;

  @BuiltValueField(wireName: r'stream')
  String get stream;

  @BuiltValueField(wireName: r'task_id')
  String? get taskId;

  @BuiltValueField(wireName: r'task_sequence')
  int? get taskSequence;

  @BuiltValueField(wireName: r'truncated')
  bool get truncated;

  DeploymentLogResponse._();

  factory DeploymentLogResponse([void updates(DeploymentLogResponseBuilder b)]) = _$DeploymentLogResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(DeploymentLogResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<DeploymentLogResponse> get serializer => _$DeploymentLogResponseSerializer();
}

class _$DeploymentLogResponseSerializer implements PrimitiveSerializer<DeploymentLogResponse> {
  @override
  final Iterable<Type> types = const [DeploymentLogResponse, _$DeploymentLogResponse];

  @override
  final String wireName = r'DeploymentLogResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    DeploymentLogResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'content';
    yield serializers.serialize(
      object.content,
      specifiedType: const FullType(String),
    );
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(String),
    );
    yield r'sequence';
    yield serializers.serialize(
      object.sequence,
      specifiedType: const FullType(int),
    );
    if (object.stage != null) {
      yield r'stage';
      yield serializers.serialize(
        object.stage,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'stream';
    yield serializers.serialize(
      object.stream,
      specifiedType: const FullType(String),
    );
    if (object.taskId != null) {
      yield r'task_id';
      yield serializers.serialize(
        object.taskId,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.taskSequence != null) {
      yield r'task_sequence';
      yield serializers.serialize(
        object.taskSequence,
        specifiedType: const FullType.nullable(int),
      );
    }
    yield r'truncated';
    yield serializers.serialize(
      object.truncated,
      specifiedType: const FullType(bool),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    DeploymentLogResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required DeploymentLogResponseBuilder result,
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
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.createdAt = valueDes;
          break;
        case r'sequence':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.sequence = valueDes;
          break;
        case r'stage':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.stage = valueDes;
          break;
        case r'stream':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.stream = valueDes;
          break;
        case r'task_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.taskId = valueDes;
          break;
        case r'task_sequence':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.taskSequence = valueDes;
          break;
        case r'truncated':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.truncated = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  DeploymentLogResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = DeploymentLogResponseBuilder();
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
