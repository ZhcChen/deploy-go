//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'deployment_event_response.g.dart';

/// DeploymentEventResponse
///
/// Properties:
/// * [createdAt]
/// * [eventName]
/// * [failureStage]
/// * [id]
/// * [message]
/// * [module]
/// * [moduleName]
/// * [stage]
/// * [status]
/// * [step]
/// * [stepId]
@BuiltValue()
abstract class DeploymentEventResponse implements Built<DeploymentEventResponse, DeploymentEventResponseBuilder> {
  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  @BuiltValueField(wireName: r'event_name')
  String get eventName;

  @BuiltValueField(wireName: r'failure_stage')
  String? get failureStage;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'message')
  String? get message;

  @BuiltValueField(wireName: r'module')
  String? get module;

  @BuiltValueField(wireName: r'module_name')
  String? get moduleName;

  @BuiltValueField(wireName: r'stage')
  String? get stage;

  @BuiltValueField(wireName: r'status')
  String? get status;

  @BuiltValueField(wireName: r'step')
  String? get step;

  @BuiltValueField(wireName: r'step_id')
  String? get stepId;

  DeploymentEventResponse._();

  factory DeploymentEventResponse([void updates(DeploymentEventResponseBuilder b)]) = _$DeploymentEventResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(DeploymentEventResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<DeploymentEventResponse> get serializer => _$DeploymentEventResponseSerializer();
}

class _$DeploymentEventResponseSerializer implements PrimitiveSerializer<DeploymentEventResponse> {
  @override
  final Iterable<Type> types = const [DeploymentEventResponse, _$DeploymentEventResponse];

  @override
  final String wireName = r'DeploymentEventResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    DeploymentEventResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(String),
    );
    yield r'event_name';
    yield serializers.serialize(
      object.eventName,
      specifiedType: const FullType(String),
    );
    if (object.failureStage != null) {
      yield r'failure_stage';
      yield serializers.serialize(
        object.failureStage,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    if (object.message != null) {
      yield r'message';
      yield serializers.serialize(
        object.message,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.module != null) {
      yield r'module';
      yield serializers.serialize(
        object.module,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.moduleName != null) {
      yield r'module_name';
      yield serializers.serialize(
        object.moduleName,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.stage != null) {
      yield r'stage';
      yield serializers.serialize(
        object.stage,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.status != null) {
      yield r'status';
      yield serializers.serialize(
        object.status,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.step != null) {
      yield r'step';
      yield serializers.serialize(
        object.step,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.stepId != null) {
      yield r'step_id';
      yield serializers.serialize(
        object.stepId,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    DeploymentEventResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required DeploymentEventResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.createdAt = valueDes;
          break;
        case r'event_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.eventName = valueDes;
          break;
        case r'failure_stage':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.failureStage = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'message':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.message = valueDes;
          break;
        case r'module':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.module = valueDes;
          break;
        case r'module_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.moduleName = valueDes;
          break;
        case r'stage':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.stage = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.status = valueDes;
          break;
        case r'step':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.step = valueDes;
          break;
        case r'step_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.stepId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  DeploymentEventResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = DeploymentEventResponseBuilder();
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
