//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'external_deployment_target.g.dart';

/// ExternalDeploymentTarget
///
/// Properties:
/// * [environment]
/// * [executionMode]
/// * [id]
/// * [nodeId]
/// * [nodeName]
/// * [privilegedRelease]
/// * [status]
@BuiltValue()
abstract class ExternalDeploymentTarget implements Built<ExternalDeploymentTarget, ExternalDeploymentTargetBuilder> {
  @BuiltValueField(wireName: r'environment')
  String get environment;

  @BuiltValueField(wireName: r'execution_mode')
  String get executionMode;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'node_id')
  String get nodeId;

  @BuiltValueField(wireName: r'node_name')
  String get nodeName;

  @BuiltValueField(wireName: r'privileged_release')
  bool get privilegedRelease;

  @BuiltValueField(wireName: r'status')
  String get status;

  ExternalDeploymentTarget._();

  factory ExternalDeploymentTarget([void updates(ExternalDeploymentTargetBuilder b)]) = _$ExternalDeploymentTarget;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ExternalDeploymentTargetBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ExternalDeploymentTarget> get serializer => _$ExternalDeploymentTargetSerializer();
}

class _$ExternalDeploymentTargetSerializer implements PrimitiveSerializer<ExternalDeploymentTarget> {
  @override
  final Iterable<Type> types = const [ExternalDeploymentTarget, _$ExternalDeploymentTarget];

  @override
  final String wireName = r'ExternalDeploymentTarget';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ExternalDeploymentTarget object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
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
    yield r'privileged_release';
    yield serializers.serialize(
      object.privilegedRelease,
      specifiedType: const FullType(bool),
    );
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ExternalDeploymentTarget object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ExternalDeploymentTargetBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
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
        case r'privileged_release':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.privilegedRelease = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ExternalDeploymentTarget deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ExternalDeploymentTargetBuilder();
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
