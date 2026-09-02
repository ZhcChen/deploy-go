//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'workspace_source_response.g.dart';

/// WorkspaceSourceResponse
///
/// Properties:
/// * [applicationId]
/// * [buildAgentId]
/// * [buildAgentName]
/// * [createdAt]
/// * [createdBy]
/// * [id]
/// * [status]
/// * [updatedAt]
/// * [version]
/// * [workspacePath]
/// * [workspaceVersion]
@BuiltValue()
abstract class WorkspaceSourceResponse implements Built<WorkspaceSourceResponse, WorkspaceSourceResponseBuilder> {
  @BuiltValueField(wireName: r'application_id')
  String get applicationId;

  @BuiltValueField(wireName: r'build_agent_id')
  String get buildAgentId;

  @BuiltValueField(wireName: r'build_agent_name')
  String? get buildAgentName;

  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  @BuiltValueField(wireName: r'created_by')
  String? get createdBy;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'updated_at')
  String get updatedAt;

  @BuiltValueField(wireName: r'version')
  int get version;

  @BuiltValueField(wireName: r'workspace_path')
  String get workspacePath;

  @BuiltValueField(wireName: r'workspace_version')
  int get workspaceVersion;

  WorkspaceSourceResponse._();

  factory WorkspaceSourceResponse([void updates(WorkspaceSourceResponseBuilder b)]) = _$WorkspaceSourceResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(WorkspaceSourceResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<WorkspaceSourceResponse> get serializer => _$WorkspaceSourceResponseSerializer();
}

class _$WorkspaceSourceResponseSerializer implements PrimitiveSerializer<WorkspaceSourceResponse> {
  @override
  final Iterable<Type> types = const [WorkspaceSourceResponse, _$WorkspaceSourceResponse];

  @override
  final String wireName = r'WorkspaceSourceResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    WorkspaceSourceResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'application_id';
    yield serializers.serialize(
      object.applicationId,
      specifiedType: const FullType(String),
    );
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
    if (object.createdBy != null) {
      yield r'created_by';
      yield serializers.serialize(
        object.createdBy,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'id';
    yield serializers.serialize(
      object.id,
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
    yield r'workspace_path';
    yield serializers.serialize(
      object.workspacePath,
      specifiedType: const FullType(String),
    );
    yield r'workspace_version';
    yield serializers.serialize(
      object.workspaceVersion,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    WorkspaceSourceResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required WorkspaceSourceResponseBuilder result,
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
        case r'created_by':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.createdBy = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
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
        case r'workspace_path':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.workspacePath = valueDes;
          break;
        case r'workspace_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
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
  WorkspaceSourceResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = WorkspaceSourceResponseBuilder();
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
