//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_source_response.g.dart';

/// ApplicationSourceResponse
///
/// Properties:
/// * [applicationId]
/// * [branchVerifiedAt]
/// * [buildAgentId]
/// * [buildAgentName]
/// * [createdAt]
/// * [deploymentBranch]
/// * [gitCredentialId]
/// * [gitCredentialName]
/// * [id]
/// * [repositoryUrl]
/// * [sourcePolicy]
/// * [status]
/// * [updatedAt]
/// * [version]
@BuiltValue()
abstract class ApplicationSourceResponse implements Built<ApplicationSourceResponse, ApplicationSourceResponseBuilder> {
  @BuiltValueField(wireName: r'application_id')
  String get applicationId;

  @BuiltValueField(wireName: r'branch_verified_at')
  String? get branchVerifiedAt;

  @BuiltValueField(wireName: r'build_agent_id')
  String get buildAgentId;

  @BuiltValueField(wireName: r'build_agent_name')
  String? get buildAgentName;

  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  @BuiltValueField(wireName: r'deployment_branch')
  String? get deploymentBranch;

  @BuiltValueField(wireName: r'git_credential_id')
  String? get gitCredentialId;

  @BuiltValueField(wireName: r'git_credential_name')
  String? get gitCredentialName;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'repository_url')
  String get repositoryUrl;

  @BuiltValueField(wireName: r'source_policy')
  String get sourcePolicy;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'updated_at')
  String get updatedAt;

  @BuiltValueField(wireName: r'version')
  int get version;

  ApplicationSourceResponse._();

  factory ApplicationSourceResponse([void updates(ApplicationSourceResponseBuilder b)]) = _$ApplicationSourceResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationSourceResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationSourceResponse> get serializer => _$ApplicationSourceResponseSerializer();
}

class _$ApplicationSourceResponseSerializer implements PrimitiveSerializer<ApplicationSourceResponse> {
  @override
  final Iterable<Type> types = const [ApplicationSourceResponse, _$ApplicationSourceResponse];

  @override
  final String wireName = r'ApplicationSourceResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationSourceResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'application_id';
    yield serializers.serialize(
      object.applicationId,
      specifiedType: const FullType(String),
    );
    if (object.branchVerifiedAt != null) {
      yield r'branch_verified_at';
      yield serializers.serialize(
        object.branchVerifiedAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'build_agent_id';
    yield serializers.serialize(
      object.buildAgentId,
      specifiedType: const FullType(String),
    );
    if (object.buildAgentName != null) {
      yield r'build_agent_name';
      yield serializers.serialize(
        object.buildAgentName,
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
    if (object.gitCredentialId != null) {
      yield r'git_credential_id';
      yield serializers.serialize(
        object.gitCredentialId,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.gitCredentialName != null) {
      yield r'git_credential_name';
      yield serializers.serialize(
        object.gitCredentialName,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'repository_url';
    yield serializers.serialize(
      object.repositoryUrl,
      specifiedType: const FullType(String),
    );
    yield r'source_policy';
    yield serializers.serialize(
      object.sourcePolicy,
      specifiedType: const FullType(String),
    );
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
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
  }

  @override
  Object serialize(
    Serializers serializers,
    ApplicationSourceResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationSourceResponseBuilder result,
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
        case r'branch_verified_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.branchVerifiedAt = valueDes;
          break;
        case r'build_agent_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.buildAgentId = valueDes;
          break;
        case r'build_agent_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.buildAgentName = valueDes;
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
        case r'git_credential_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.gitCredentialId = valueDes;
          break;
        case r'git_credential_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.gitCredentialName = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'repository_url':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.repositoryUrl = valueDes;
          break;
        case r'source_policy':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.sourcePolicy = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
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
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ApplicationSourceResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationSourceResponseBuilder();
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
