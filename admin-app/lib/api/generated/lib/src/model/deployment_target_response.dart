//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/image_deploy_spec.dart';
import 'package:deploy_go_api_client/src/model/secret_file_reference.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'deployment_target_response.g.dart';

/// DeploymentTargetResponse
///
/// Properties:
/// * [applicationId]
/// * [createdAt]
/// * [environment]
/// * [executionMode]
/// * [id]
/// * [imageSpec]
/// * [nodeId]
/// * [parameterSchema]
/// * [privilegedRelease]
/// * [scriptPath]
/// * [secretFileReferences]
/// * [snapshotHash]
/// * [status]
/// * [targetCode]
/// * [timeoutSeconds]
/// * [updatedAt]
/// * [verificationConfig]
/// * [version]
@BuiltValue()
abstract class DeploymentTargetResponse implements Built<DeploymentTargetResponse, DeploymentTargetResponseBuilder> {
  @BuiltValueField(wireName: r'application_id')
  String get applicationId;

  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  @BuiltValueField(wireName: r'environment')
  String get environment;

  @BuiltValueField(wireName: r'execution_mode')
  String get executionMode;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'image_spec')
  ImageDeploySpec? get imageSpec;

  @BuiltValueField(wireName: r'node_id')
  String get nodeId;

  @BuiltValueField(wireName: r'parameter_schema')
  JsonObject? get parameterSchema;

  @BuiltValueField(wireName: r'privileged_release')
  bool get privilegedRelease;

  @BuiltValueField(wireName: r'script_path')
  String get scriptPath;

  @BuiltValueField(wireName: r'secret_file_references')
  BuiltList<SecretFileReference> get secretFileReferences;

  @BuiltValueField(wireName: r'snapshot_hash')
  String get snapshotHash;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'target_code')
  String get targetCode;

  @BuiltValueField(wireName: r'timeout_seconds')
  int get timeoutSeconds;

  @BuiltValueField(wireName: r'updated_at')
  String get updatedAt;

  @BuiltValueField(wireName: r'verification_config')
  JsonObject? get verificationConfig;

  @BuiltValueField(wireName: r'version')
  int get version;

  DeploymentTargetResponse._();

  factory DeploymentTargetResponse([void updates(DeploymentTargetResponseBuilder b)]) = _$DeploymentTargetResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(DeploymentTargetResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<DeploymentTargetResponse> get serializer => _$DeploymentTargetResponseSerializer();
}

class _$DeploymentTargetResponseSerializer implements PrimitiveSerializer<DeploymentTargetResponse> {
  @override
  final Iterable<Type> types = const [DeploymentTargetResponse, _$DeploymentTargetResponse];

  @override
  final String wireName = r'DeploymentTargetResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    DeploymentTargetResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'application_id';
    yield serializers.serialize(
      object.applicationId,
      specifiedType: const FullType(String),
    );
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(String),
    );
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
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    if (object.imageSpec != null) {
      yield r'image_spec';
      yield serializers.serialize(
        object.imageSpec,
        specifiedType: const FullType.nullable(ImageDeploySpec),
      );
    }
    yield r'node_id';
    yield serializers.serialize(
      object.nodeId,
      specifiedType: const FullType(String),
    );
    yield r'parameter_schema';
    yield object.parameterSchema == null ? null : serializers.serialize(
      object.parameterSchema,
      specifiedType: const FullType.nullable(JsonObject),
    );
    yield r'privileged_release';
    yield serializers.serialize(
      object.privilegedRelease,
      specifiedType: const FullType(bool),
    );
    yield r'script_path';
    yield serializers.serialize(
      object.scriptPath,
      specifiedType: const FullType(String),
    );
    yield r'secret_file_references';
    yield serializers.serialize(
      object.secretFileReferences,
      specifiedType: const FullType(BuiltList, [FullType(SecretFileReference)]),
    );
    yield r'snapshot_hash';
    yield serializers.serialize(
      object.snapshotHash,
      specifiedType: const FullType(String),
    );
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
    yield r'target_code';
    yield serializers.serialize(
      object.targetCode,
      specifiedType: const FullType(String),
    );
    yield r'timeout_seconds';
    yield serializers.serialize(
      object.timeoutSeconds,
      specifiedType: const FullType(int),
    );
    yield r'updated_at';
    yield serializers.serialize(
      object.updatedAt,
      specifiedType: const FullType(String),
    );
    yield r'verification_config';
    yield object.verificationConfig == null ? null : serializers.serialize(
      object.verificationConfig,
      specifiedType: const FullType.nullable(JsonObject),
    );
    yield r'version';
    yield serializers.serialize(
      object.version,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    DeploymentTargetResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required DeploymentTargetResponseBuilder result,
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
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.createdAt = valueDes;
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
            specifiedType: const FullType.nullable(ImageDeploySpec),
          ) as ImageDeploySpec?;
          if (valueDes == null) continue;
          result.imageSpec.replace(valueDes);
          break;
        case r'node_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.nodeId = valueDes;
          break;
        case r'parameter_schema':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.parameterSchema = valueDes;
          break;
        case r'privileged_release':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.privilegedRelease = valueDes;
          break;
        case r'script_path':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.scriptPath = valueDes;
          break;
        case r'secret_file_references':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(SecretFileReference)]),
          ) as BuiltList<SecretFileReference>;
          result.secretFileReferences.replace(valueDes);
          break;
        case r'snapshot_hash':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.snapshotHash = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        case r'target_code':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.targetCode = valueDes;
          break;
        case r'timeout_seconds':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.timeoutSeconds = valueDes;
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.updatedAt = valueDes;
          break;
        case r'verification_config':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.verificationConfig = valueDes;
          break;
        case r'version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.version = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  DeploymentTargetResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = DeploymentTargetResponseBuilder();
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
