//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'save_workspace_source_request.g.dart';

/// SaveWorkspaceSourceRequest
///
/// Properties:
/// * [buildAgentId]
/// * [version]
/// * [workspacePath]
@BuiltValue()
abstract class SaveWorkspaceSourceRequest implements Built<SaveWorkspaceSourceRequest, SaveWorkspaceSourceRequestBuilder> {
  @BuiltValueField(wireName: r'build_agent_id')
  String get buildAgentId;

  @BuiltValueField(wireName: r'version')
  int? get version;

  @BuiltValueField(wireName: r'workspace_path')
  String get workspacePath;

  SaveWorkspaceSourceRequest._();

  factory SaveWorkspaceSourceRequest([void updates(SaveWorkspaceSourceRequestBuilder b)]) = _$SaveWorkspaceSourceRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SaveWorkspaceSourceRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SaveWorkspaceSourceRequest> get serializer => _$SaveWorkspaceSourceRequestSerializer();
}

class _$SaveWorkspaceSourceRequestSerializer implements PrimitiveSerializer<SaveWorkspaceSourceRequest> {
  @override
  final Iterable<Type> types = const [SaveWorkspaceSourceRequest, _$SaveWorkspaceSourceRequest];

  @override
  final String wireName = r'SaveWorkspaceSourceRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SaveWorkspaceSourceRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'build_agent_id';
    yield serializers.serialize(
      object.buildAgentId,
      specifiedType: const FullType(String),
    );
    if (object.version != null) {
      yield r'version';
      yield serializers.serialize(
        object.version,
        specifiedType: const FullType.nullable(int),
      );
    }
    yield r'workspace_path';
    yield serializers.serialize(
      object.workspacePath,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SaveWorkspaceSourceRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required SaveWorkspaceSourceRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'build_agent_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.buildAgentId = valueDes;
          break;
        case r'version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.version = valueDes;
          break;
        case r'workspace_path':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.workspacePath = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SaveWorkspaceSourceRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SaveWorkspaceSourceRequestBuilder();
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
