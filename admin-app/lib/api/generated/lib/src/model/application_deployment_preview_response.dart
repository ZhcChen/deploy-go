//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:deploy_go_api_client/src/model/deployment_target_preview_response.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_deployment_preview_response.g.dart';

/// ApplicationDeploymentPreviewResponse
///
/// Properties:
/// * [applicationId]
/// * [applicationName]
/// * [deploymentBranch]
/// * [executionMode]
/// * [modules]
/// * [parameters]
/// * [releaseStrategy]
/// * [releaseVersion]
/// * [resolvedCommitSha]
/// * [snapshotHash]
/// * [targets]
@BuiltValue()
abstract class ApplicationDeploymentPreviewResponse implements Built<ApplicationDeploymentPreviewResponse, ApplicationDeploymentPreviewResponseBuilder> {
  @BuiltValueField(wireName: r'application_id')
  String get applicationId;

  @BuiltValueField(wireName: r'application_name')
  String get applicationName;

  @BuiltValueField(wireName: r'deployment_branch')
  String? get deploymentBranch;

  @BuiltValueField(wireName: r'execution_mode')
  String get executionMode;

  @BuiltValueField(wireName: r'modules')
  BuiltList<String>? get modules;

  @BuiltValueField(wireName: r'parameters')
  JsonObject? get parameters;

  @BuiltValueField(wireName: r'release_strategy')
  String get releaseStrategy;

  @BuiltValueField(wireName: r'release_version')
  String? get releaseVersion;

  @BuiltValueField(wireName: r'resolved_commit_sha')
  String? get resolvedCommitSha;

  @BuiltValueField(wireName: r'snapshot_hash')
  String get snapshotHash;

  @BuiltValueField(wireName: r'targets')
  BuiltList<DeploymentTargetPreviewResponse> get targets;

  ApplicationDeploymentPreviewResponse._();

  factory ApplicationDeploymentPreviewResponse([void updates(ApplicationDeploymentPreviewResponseBuilder b)]) = _$ApplicationDeploymentPreviewResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationDeploymentPreviewResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationDeploymentPreviewResponse> get serializer => _$ApplicationDeploymentPreviewResponseSerializer();
}

class _$ApplicationDeploymentPreviewResponseSerializer implements PrimitiveSerializer<ApplicationDeploymentPreviewResponse> {
  @override
  final Iterable<Type> types = const [ApplicationDeploymentPreviewResponse, _$ApplicationDeploymentPreviewResponse];

  @override
  final String wireName = r'ApplicationDeploymentPreviewResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationDeploymentPreviewResponse object, {
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
    yield r'parameters';
    yield object.parameters == null ? null : serializers.serialize(
      object.parameters,
      specifiedType: const FullType.nullable(JsonObject),
    );
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
    if (object.resolvedCommitSha != null) {
      yield r'resolved_commit_sha';
      yield serializers.serialize(
        object.resolvedCommitSha,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'snapshot_hash';
    yield serializers.serialize(
      object.snapshotHash,
      specifiedType: const FullType(String),
    );
    yield r'targets';
    yield serializers.serialize(
      object.targets,
      specifiedType: const FullType(BuiltList, [FullType(DeploymentTargetPreviewResponse)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ApplicationDeploymentPreviewResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationDeploymentPreviewResponseBuilder result,
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
        case r'parameters':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.parameters = valueDes;
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
        case r'resolved_commit_sha':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.resolvedCommitSha = valueDes;
          break;
        case r'snapshot_hash':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.snapshotHash = valueDes;
          break;
        case r'targets':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(DeploymentTargetPreviewResponse)]),
          ) as BuiltList<DeploymentTargetPreviewResponse>;
          result.targets.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ApplicationDeploymentPreviewResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationDeploymentPreviewResponseBuilder();
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
