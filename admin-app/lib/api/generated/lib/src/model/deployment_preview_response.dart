//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'deployment_preview_response.g.dart';

/// DeploymentPreviewResponse
///
/// Properties:
/// * [applicationId]
/// * [applicationName]
/// * [deploymentBranch]
/// * [environment]
/// * [executionMode]
/// * [modules]
/// * [nodeId]
/// * [nodeName]
/// * [parameters]
/// * [releaseVersion]
/// * [resolvedCommitSha]
/// * [scriptPath]
/// * [snapshotHash]
/// * [sourcePolicy]
/// * [targetId]
@BuiltValue()
abstract class DeploymentPreviewResponse implements Built<DeploymentPreviewResponse, DeploymentPreviewResponseBuilder> {
  @BuiltValueField(wireName: r'application_id')
  String get applicationId;

  @BuiltValueField(wireName: r'application_name')
  String get applicationName;

  @BuiltValueField(wireName: r'deployment_branch')
  String? get deploymentBranch;

  @BuiltValueField(wireName: r'environment')
  String get environment;

  @BuiltValueField(wireName: r'execution_mode')
  String get executionMode;

  @BuiltValueField(wireName: r'modules')
  BuiltList<String>? get modules;

  @BuiltValueField(wireName: r'node_id')
  String get nodeId;

  @BuiltValueField(wireName: r'node_name')
  String get nodeName;

  @BuiltValueField(wireName: r'parameters')
  JsonObject? get parameters;

  @BuiltValueField(wireName: r'release_version')
  String? get releaseVersion;

  @BuiltValueField(wireName: r'resolved_commit_sha')
  String? get resolvedCommitSha;

  @BuiltValueField(wireName: r'script_path')
  String get scriptPath;

  @BuiltValueField(wireName: r'snapshot_hash')
  String get snapshotHash;

  @BuiltValueField(wireName: r'source_policy')
  String? get sourcePolicy;

  @BuiltValueField(wireName: r'target_id')
  String get targetId;

  DeploymentPreviewResponse._();

  factory DeploymentPreviewResponse([void updates(DeploymentPreviewResponseBuilder b)]) = _$DeploymentPreviewResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(DeploymentPreviewResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<DeploymentPreviewResponse> get serializer => _$DeploymentPreviewResponseSerializer();
}

class _$DeploymentPreviewResponseSerializer implements PrimitiveSerializer<DeploymentPreviewResponse> {
  @override
  final Iterable<Type> types = const [DeploymentPreviewResponse, _$DeploymentPreviewResponse];

  @override
  final String wireName = r'DeploymentPreviewResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    DeploymentPreviewResponse object, {
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
    if (object.deploymentBranch != null) {
      yield r'deployment_branch';
      yield serializers.serialize(
        object.deploymentBranch,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'environment';
    yield serializers.serialize(
      object.environment,
      specifiedType: const FullType(String),
    );
    yield r'execution_mode';
    yield serializers.serialize(
      object.executionMode,
      specifiedType: const FullType(String),
    );
    if (object.modules != null) {
      yield r'modules';
      yield serializers.serialize(
        object.modules,
        specifiedType: const FullType.nullable(BuiltList, [FullType(String)]),
      );
    }
    yield r'node_id';
    yield serializers.serialize(
      object.nodeId,
      specifiedType: const FullType(String),
    );
    yield r'node_name';
    yield serializers.serialize(
      object.nodeName,
      specifiedType: const FullType(String),
    );
    yield r'parameters';
    yield object.parameters == null ? null : serializers.serialize(
      object.parameters,
      specifiedType: const FullType.nullable(JsonObject),
    );
    if (object.releaseVersion != null) {
      yield r'release_version';
      yield serializers.serialize(
        object.releaseVersion,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.resolvedCommitSha != null) {
      yield r'resolved_commit_sha';
      yield serializers.serialize(
        object.resolvedCommitSha,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'script_path';
    yield serializers.serialize(
      object.scriptPath,
      specifiedType: const FullType(String),
    );
    yield r'snapshot_hash';
    yield serializers.serialize(
      object.snapshotHash,
      specifiedType: const FullType(String),
    );
    if (object.sourcePolicy != null) {
      yield r'source_policy';
      yield serializers.serialize(
        object.sourcePolicy,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'target_id';
    yield serializers.serialize(
      object.targetId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    DeploymentPreviewResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required DeploymentPreviewResponseBuilder result,
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
        case r'deployment_branch':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.deploymentBranch = valueDes;
          break;
        case r'environment':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.environment = valueDes;
          break;
        case r'execution_mode':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.executionMode = valueDes;
          break;
        case r'modules':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(BuiltList, [FullType(String)]),
          ) as BuiltList<String>?;
          if (valueDes == null) continue;
          result.modules.replace(valueDes);
          break;
        case r'node_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.nodeId = valueDes;
          break;
        case r'node_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.nodeName = valueDes;
          break;
        case r'parameters':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.parameters = valueDes;
          break;
        case r'release_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.releaseVersion = valueDes;
          break;
        case r'resolved_commit_sha':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.resolvedCommitSha = valueDes;
          break;
        case r'script_path':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.scriptPath = valueDes;
          break;
        case r'snapshot_hash':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.snapshotHash = valueDes;
          break;
        case r'source_policy':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.sourcePolicy = valueDes;
          break;
        case r'target_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.targetId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  DeploymentPreviewResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = DeploymentPreviewResponseBuilder();
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
