//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'deployment_target_preview_response.g.dart';

/// DeploymentTargetPreviewResponse
///
/// Properties:
/// * [agentId]
/// * [agentOnline]
/// * [envGateStatus]
/// * [imageSpec]
/// * [nodeId]
/// * [nodeName]
/// * [scriptPath]
/// * [targetCode]
/// * [targetId]
@BuiltValue()
abstract class DeploymentTargetPreviewResponse implements Built<DeploymentTargetPreviewResponse, DeploymentTargetPreviewResponseBuilder> {
  @BuiltValueField(wireName: r'agent_id')
  String get agentId;

  @BuiltValueField(wireName: r'agent_online')
  bool get agentOnline;

  @BuiltValueField(wireName: r'env_gate_status')
  String get envGateStatus;

  @BuiltValueField(wireName: r'image_spec')
  JsonObject? get imageSpec;

  @BuiltValueField(wireName: r'node_id')
  String get nodeId;

  @BuiltValueField(wireName: r'node_name')
  String get nodeName;

  @BuiltValueField(wireName: r'script_path')
  String get scriptPath;

  @BuiltValueField(wireName: r'target_code')
  String get targetCode;

  @BuiltValueField(wireName: r'target_id')
  String get targetId;

  DeploymentTargetPreviewResponse._();

  factory DeploymentTargetPreviewResponse([void updates(DeploymentTargetPreviewResponseBuilder b)]) = _$DeploymentTargetPreviewResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(DeploymentTargetPreviewResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<DeploymentTargetPreviewResponse> get serializer => _$DeploymentTargetPreviewResponseSerializer();
}

class _$DeploymentTargetPreviewResponseSerializer implements PrimitiveSerializer<DeploymentTargetPreviewResponse> {
  @override
  final Iterable<Type> types = const [DeploymentTargetPreviewResponse, _$DeploymentTargetPreviewResponse];

  @override
  final String wireName = r'DeploymentTargetPreviewResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    DeploymentTargetPreviewResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'agent_id';
    yield serializers.serialize(
      object.agentId,
      specifiedType: const FullType(String),
    );
    yield r'agent_online';
    yield serializers.serialize(
      object.agentOnline,
      specifiedType: const FullType(bool),
    );
    yield r'env_gate_status';
    yield serializers.serialize(
      object.envGateStatus,
      specifiedType: const FullType(String),
    );
    if (object.imageSpec != null) {
      yield r'image_spec';
      yield serializers.serialize(
        object.imageSpec,
        specifiedType: const FullType.nullable(JsonObject),
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
    yield r'script_path';
    yield serializers.serialize(
      object.scriptPath,
      specifiedType: const FullType(String),
    );
    yield r'target_code';
    yield serializers.serialize(
      object.targetCode,
      specifiedType: const FullType(String),
    );
    yield r'target_id';
    yield serializers.serialize(
      object.targetId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    DeploymentTargetPreviewResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required DeploymentTargetPreviewResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'agent_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.agentId = valueDes;
          break;
        case r'agent_online':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.agentOnline = valueDes;
          break;
        case r'env_gate_status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.envGateStatus = valueDes;
          break;
        case r'image_spec':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.imageSpec = valueDes;
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
        case r'script_path':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.scriptPath = valueDes;
          break;
        case r'target_code':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.targetCode = valueDes;
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
  DeploymentTargetPreviewResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = DeploymentTargetPreviewResponseBuilder();
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
