//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'deployment_stage_task_summary.g.dart';

/// DeploymentStageTaskSummary
///
/// Properties:
/// * [createdAt]
/// * [errorCode]
/// * [exitCode]
/// * [finishedAt]
/// * [stage]
/// * [startedAt]
/// * [status]
/// * [taskId]
/// * [updatedAt]
@BuiltValue()
abstract class DeploymentStageTaskSummary implements Built<DeploymentStageTaskSummary, DeploymentStageTaskSummaryBuilder> {
  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  @BuiltValueField(wireName: r'error_code')
  String? get errorCode;

  @BuiltValueField(wireName: r'exit_code')
  int? get exitCode;

  @BuiltValueField(wireName: r'finished_at')
  String? get finishedAt;

  @BuiltValueField(wireName: r'stage')
  String get stage;

  @BuiltValueField(wireName: r'started_at')
  String? get startedAt;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'task_id')
  String get taskId;

  @BuiltValueField(wireName: r'updated_at')
  String get updatedAt;

  DeploymentStageTaskSummary._();

  factory DeploymentStageTaskSummary([void updates(DeploymentStageTaskSummaryBuilder b)]) = _$DeploymentStageTaskSummary;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(DeploymentStageTaskSummaryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<DeploymentStageTaskSummary> get serializer => _$DeploymentStageTaskSummarySerializer();
}

class _$DeploymentStageTaskSummarySerializer implements PrimitiveSerializer<DeploymentStageTaskSummary> {
  @override
  final Iterable<Type> types = const [DeploymentStageTaskSummary, _$DeploymentStageTaskSummary];

  @override
  final String wireName = r'DeploymentStageTaskSummary';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    DeploymentStageTaskSummary object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(String),
    );
    if (object.errorCode != null) {
      yield r'error_code';
      yield serializers.serialize(
        object.errorCode,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.exitCode != null) {
      yield r'exit_code';
      yield serializers.serialize(
        object.exitCode,
        specifiedType: const FullType.nullable(int),
      );
    }
    if (object.finishedAt != null) {
      yield r'finished_at';
      yield serializers.serialize(
        object.finishedAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'stage';
    yield serializers.serialize(
      object.stage,
      specifiedType: const FullType(String),
    );
    if (object.startedAt != null) {
      yield r'started_at';
      yield serializers.serialize(
        object.startedAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
    yield r'task_id';
    yield serializers.serialize(
      object.taskId,
      specifiedType: const FullType(String),
    );
    yield r'updated_at';
    yield serializers.serialize(
      object.updatedAt,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    DeploymentStageTaskSummary object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required DeploymentStageTaskSummaryBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.createdAt = valueDes;
          break;
        case r'error_code':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.errorCode = valueDes;
          break;
        case r'exit_code':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.exitCode = valueDes;
          break;
        case r'finished_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.finishedAt = valueDes;
          break;
        case r'stage':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.stage = valueDes;
          break;
        case r'started_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.startedAt = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        case r'task_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.taskId = valueDes;
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.updatedAt = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  DeploymentStageTaskSummary deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = DeploymentStageTaskSummaryBuilder();
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
