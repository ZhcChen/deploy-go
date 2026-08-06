//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'deployment_target_run_response.g.dart';

/// DeploymentTargetRunResponse
///
/// Properties:
/// * [agentId]
/// * [createdAt]
/// * [envGateStatus]
/// * [errorCode]
/// * [finishedAt]
/// * [id]
/// * [nodeId]
/// * [phase]
/// * [resultSummary]
/// * [sourceRunId]
/// * [startedAt]
/// * [status]
/// * [targetId]
/// * [updatedAt]
@BuiltValue()
abstract class DeploymentTargetRunResponse implements Built<DeploymentTargetRunResponse, DeploymentTargetRunResponseBuilder> {
  @BuiltValueField(wireName: r'agent_id')
  String? get agentId;

  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  @BuiltValueField(wireName: r'env_gate_status')
  String get envGateStatus;

  @BuiltValueField(wireName: r'error_code')
  String? get errorCode;

  @BuiltValueField(wireName: r'finished_at')
  String? get finishedAt;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'node_id')
  String get nodeId;

  @BuiltValueField(wireName: r'phase')
  String get phase;

  @BuiltValueField(wireName: r'result_summary')
  String? get resultSummary;

  @BuiltValueField(wireName: r'source_run_id')
  String? get sourceRunId;

  @BuiltValueField(wireName: r'started_at')
  String? get startedAt;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'target_id')
  String get targetId;

  @BuiltValueField(wireName: r'updated_at')
  String get updatedAt;

  DeploymentTargetRunResponse._();

  factory DeploymentTargetRunResponse([void updates(DeploymentTargetRunResponseBuilder b)]) = _$DeploymentTargetRunResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(DeploymentTargetRunResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<DeploymentTargetRunResponse> get serializer => _$DeploymentTargetRunResponseSerializer();
}

class _$DeploymentTargetRunResponseSerializer implements PrimitiveSerializer<DeploymentTargetRunResponse> {
  @override
  final Iterable<Type> types = const [DeploymentTargetRunResponse, _$DeploymentTargetRunResponse];

  @override
  final String wireName = r'DeploymentTargetRunResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    DeploymentTargetRunResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.agentId != null) {
      yield r'agent_id';
      yield serializers.serialize(
        object.agentId,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(String),
    );
    yield r'env_gate_status';
    yield serializers.serialize(
      object.envGateStatus,
      specifiedType: const FullType(String),
    );
    if (object.errorCode != null) {
      yield r'error_code';
      yield serializers.serialize(
        object.errorCode,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.finishedAt != null) {
      yield r'finished_at';
      yield serializers.serialize(
        object.finishedAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'node_id';
    yield serializers.serialize(
      object.nodeId,
      specifiedType: const FullType(String),
    );
    yield r'phase';
    yield serializers.serialize(
      object.phase,
      specifiedType: const FullType(String),
    );
    if (object.resultSummary != null) {
      yield r'result_summary';
      yield serializers.serialize(
        object.resultSummary,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.sourceRunId != null) {
      yield r'source_run_id';
      yield serializers.serialize(
        object.sourceRunId,
        specifiedType: const FullType.nullable(String),
      );
    }
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
    yield r'target_id';
    yield serializers.serialize(
      object.targetId,
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
    DeploymentTargetRunResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required DeploymentTargetRunResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'agent_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.agentId = valueDes;
          break;
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.createdAt = valueDes;
          break;
        case r'env_gate_status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.envGateStatus = valueDes;
          break;
        case r'error_code':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.errorCode = valueDes;
          break;
        case r'finished_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.finishedAt = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'node_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.nodeId = valueDes;
          break;
        case r'phase':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.phase = valueDes;
          break;
        case r'result_summary':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.resultSummary = valueDes;
          break;
        case r'source_run_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.sourceRunId = valueDes;
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
        case r'target_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.targetId = valueDes;
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
  DeploymentTargetRunResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = DeploymentTargetRunResponseBuilder();
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
