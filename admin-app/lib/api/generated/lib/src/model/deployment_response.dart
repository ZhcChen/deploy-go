//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:deploy_go_api_client/src/model/deployment_stage_task_summary.dart';
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/deployment_target_run_response.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'deployment_response.g.dart';

/// DeploymentResponse
///
/// Properties:
/// * [applicationId]
/// * [applicationName]
/// * [cancelRequestedAt]
/// * [createdAt]
/// * [deploymentBranch]
/// * [executionMode]
/// * [exitCode]
/// * [finishedAt]
/// * [id]
/// * [imageSpec]
/// * [modules]
/// * [phase]
/// * [protocolComplete]
/// * [queuedAt]
/// * [referenceDurationSeconds]
/// * [releaseStrategy]
/// * [releaseVersion]
/// * [requestedBy]
/// * [resolvedCommitSha]
/// * [resultSummary]
/// * [retryOfId]
/// * [snapshotHash]
/// * [stageTasks]
/// * [startedAt]
/// * [status]
/// * [targetId]
/// * [targetRuns]
/// * [updatedAt]
/// * [version]
/// * [workspacePath]
/// * [workspaceVersion]
@BuiltValue()
abstract class DeploymentResponse implements Built<DeploymentResponse, DeploymentResponseBuilder> {
  @BuiltValueField(wireName: r'application_id')
  String get applicationId;

  @BuiltValueField(wireName: r'application_name')
  String get applicationName;

  @BuiltValueField(wireName: r'cancel_requested_at')
  String? get cancelRequestedAt;

  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  @BuiltValueField(wireName: r'deployment_branch')
  String? get deploymentBranch;

  @BuiltValueField(wireName: r'execution_mode')
  String get executionMode;

  @BuiltValueField(wireName: r'exit_code')
  int? get exitCode;

  @BuiltValueField(wireName: r'finished_at')
  String? get finishedAt;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'image_spec')
  JsonObject? get imageSpec;

  @BuiltValueField(wireName: r'modules')
  BuiltList<String>? get modules;

  @BuiltValueField(wireName: r'phase')
  String get phase;

  @BuiltValueField(wireName: r'protocol_complete')
  bool get protocolComplete;

  @BuiltValueField(wireName: r'queued_at')
  String get queuedAt;

  @BuiltValueField(wireName: r'reference_duration_seconds')
  int? get referenceDurationSeconds;

  @BuiltValueField(wireName: r'release_strategy')
  String get releaseStrategy;

  @BuiltValueField(wireName: r'release_version')
  String? get releaseVersion;

  @BuiltValueField(wireName: r'requested_by')
  String get requestedBy;

  @BuiltValueField(wireName: r'resolved_commit_sha')
  String? get resolvedCommitSha;

  @BuiltValueField(wireName: r'result_summary')
  String? get resultSummary;

  @BuiltValueField(wireName: r'retry_of_id')
  String? get retryOfId;

  @BuiltValueField(wireName: r'snapshot_hash')
  String get snapshotHash;

  @BuiltValueField(wireName: r'stage_tasks')
  BuiltList<DeploymentStageTaskSummary> get stageTasks;

  @BuiltValueField(wireName: r'started_at')
  String? get startedAt;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'target_id')
  String get targetId;

  @BuiltValueField(wireName: r'target_runs')
  BuiltList<DeploymentTargetRunResponse> get targetRuns;

  @BuiltValueField(wireName: r'updated_at')
  String get updatedAt;

  @BuiltValueField(wireName: r'version')
  int get version;

  @BuiltValueField(wireName: r'workspace_path')
  String? get workspacePath;

  @BuiltValueField(wireName: r'workspace_version')
  int? get workspaceVersion;

  DeploymentResponse._();

  factory DeploymentResponse([void updates(DeploymentResponseBuilder b)]) = _$DeploymentResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(DeploymentResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<DeploymentResponse> get serializer => _$DeploymentResponseSerializer();
}

class _$DeploymentResponseSerializer implements PrimitiveSerializer<DeploymentResponse> {
  @override
  final Iterable<Type> types = const [DeploymentResponse, _$DeploymentResponse];

  @override
  final String wireName = r'DeploymentResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    DeploymentResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'application_id';
    yield serializers.serialize(
      object.applicationId,
      specifiedType: const FullType(String),
    );
    yield r'application_name';
    yield serializers.serialize(
      object.applicationName,
      specifiedType: const FullType(String),
    );
    if (object.cancelRequestedAt != null) {
      yield r'cancel_requested_at';
      yield serializers.serialize(
        object.cancelRequestedAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(String),
    );
    if (object.deploymentBranch != null) {
      yield r'deployment_branch';
      yield serializers.serialize(
        object.deploymentBranch,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'execution_mode';
    yield serializers.serialize(
      object.executionMode,
      specifiedType: const FullType(String),
    );
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
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    if (object.imageSpec != null) {
      yield r'image_spec';
      yield serializers.serialize(
        object.imageSpec,
        specifiedType: const FullType.nullable(JsonObject),
      );
    }
    if (object.modules != null) {
      yield r'modules';
      yield serializers.serialize(
        object.modules,
        specifiedType: const FullType.nullable(BuiltList, [FullType(String)]),
      );
    }
    yield r'phase';
    yield serializers.serialize(
      object.phase,
      specifiedType: const FullType(String),
    );
    yield r'protocol_complete';
    yield serializers.serialize(
      object.protocolComplete,
      specifiedType: const FullType(bool),
    );
    yield r'queued_at';
    yield serializers.serialize(
      object.queuedAt,
      specifiedType: const FullType(String),
    );
    if (object.referenceDurationSeconds != null) {
      yield r'reference_duration_seconds';
      yield serializers.serialize(
        object.referenceDurationSeconds,
        specifiedType: const FullType.nullable(int),
      );
    }
    yield r'release_strategy';
    yield serializers.serialize(
      object.releaseStrategy,
      specifiedType: const FullType(String),
    );
    if (object.releaseVersion != null) {
      yield r'release_version';
      yield serializers.serialize(
        object.releaseVersion,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'requested_by';
    yield serializers.serialize(
      object.requestedBy,
      specifiedType: const FullType(String),
    );
    if (object.resolvedCommitSha != null) {
      yield r'resolved_commit_sha';
      yield serializers.serialize(
        object.resolvedCommitSha,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.resultSummary != null) {
      yield r'result_summary';
      yield serializers.serialize(
        object.resultSummary,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.retryOfId != null) {
      yield r'retry_of_id';
      yield serializers.serialize(
        object.retryOfId,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'snapshot_hash';
    yield serializers.serialize(
      object.snapshotHash,
      specifiedType: const FullType(String),
    );
    yield r'stage_tasks';
    yield serializers.serialize(
      object.stageTasks,
      specifiedType: const FullType(BuiltList, [FullType(DeploymentStageTaskSummary)]),
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
    yield r'target_id';
    yield serializers.serialize(
      object.targetId,
      specifiedType: const FullType(String),
    );
    yield r'target_runs';
    yield serializers.serialize(
      object.targetRuns,
      specifiedType: const FullType(BuiltList, [FullType(DeploymentTargetRunResponse)]),
    );
    yield r'updated_at';
    yield serializers.serialize(
      object.updatedAt,
      specifiedType: const FullType(String),
    );
    yield r'version';
    yield serializers.serialize(
      object.version,
      specifiedType: const FullType(int),
    );
    if (object.workspacePath != null) {
      yield r'workspace_path';
      yield serializers.serialize(
        object.workspacePath,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.workspaceVersion != null) {
      yield r'workspace_version';
      yield serializers.serialize(
        object.workspaceVersion,
        specifiedType: const FullType.nullable(int),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    DeploymentResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required DeploymentResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'application_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.applicationId = valueDes;
          break;
        case r'application_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.applicationName = valueDes;
          break;
        case r'cancel_requested_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.cancelRequestedAt = valueDes;
          break;
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.createdAt = valueDes;
          break;
        case r'deployment_branch':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.deploymentBranch = valueDes;
          break;
        case r'execution_mode':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.executionMode = valueDes;
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
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'image_spec':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.imageSpec = valueDes;
          break;
        case r'modules':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(BuiltList, [FullType(String)]),
          ) as BuiltList<String>?;
          if (valueDes == null) continue;
          result.modules.replace(valueDes);
          break;
        case r'phase':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.phase = valueDes;
          break;
        case r'protocol_complete':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.protocolComplete = valueDes;
          break;
        case r'queued_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.queuedAt = valueDes;
          break;
        case r'reference_duration_seconds':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.referenceDurationSeconds = valueDes;
          break;
        case r'release_strategy':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.releaseStrategy = valueDes;
          break;
        case r'release_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.releaseVersion = valueDes;
          break;
        case r'requested_by':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.requestedBy = valueDes;
          break;
        case r'resolved_commit_sha':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.resolvedCommitSha = valueDes;
          break;
        case r'result_summary':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.resultSummary = valueDes;
          break;
        case r'retry_of_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.retryOfId = valueDes;
          break;
        case r'snapshot_hash':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.snapshotHash = valueDes;
          break;
        case r'stage_tasks':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(DeploymentStageTaskSummary)]),
          ) as BuiltList<DeploymentStageTaskSummary>;
          result.stageTasks.replace(valueDes);
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
        case r'target_runs':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(DeploymentTargetRunResponse)]),
          ) as BuiltList<DeploymentTargetRunResponse>;
          result.targetRuns.replace(valueDes);
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.updatedAt = valueDes;
          break;
        case r'version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.version = valueDes;
          break;
        case r'workspace_path':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.workspacePath = valueDes;
          break;
        case r'workspace_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.workspaceVersion = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  DeploymentResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = DeploymentResponseBuilder();
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
