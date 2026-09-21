//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'agent_upgrade_response.g.dart';

/// AgentUpgradeResponse
///
/// Properties:
/// * [agentId]
/// * [attemptCount]
/// * [currentVersion]
/// * [errorCode]
/// * [errorSummary]
/// * [finishedAt]
/// * [id]
/// * [nodeId]
/// * [phase]
/// * [queuedAt]
/// * [startedAt]
/// * [status]
/// * [targetArchitecture]
/// * [targetVersion]
/// * [updatedAt]
@BuiltValue()
abstract class AgentUpgradeResponse implements Built<AgentUpgradeResponse, AgentUpgradeResponseBuilder> {
  @BuiltValueField(wireName: r'agent_id')
  String get agentId;

  @BuiltValueField(wireName: r'attempt_count')
  int get attemptCount;

  @BuiltValueField(wireName: r'current_version')
  String? get currentVersion;

  @BuiltValueField(wireName: r'error_code')
  String? get errorCode;

  @BuiltValueField(wireName: r'error_summary')
  String? get errorSummary;

  @BuiltValueField(wireName: r'finished_at')
  String? get finishedAt;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'node_id')
  String get nodeId;

  @BuiltValueField(wireName: r'phase')
  String? get phase;

  @BuiltValueField(wireName: r'queued_at')
  String get queuedAt;

  @BuiltValueField(wireName: r'started_at')
  String? get startedAt;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'target_architecture')
  String get targetArchitecture;

  @BuiltValueField(wireName: r'target_version')
  String get targetVersion;

  @BuiltValueField(wireName: r'updated_at')
  String get updatedAt;

  AgentUpgradeResponse._();

  factory AgentUpgradeResponse([void updates(AgentUpgradeResponseBuilder b)]) = _$AgentUpgradeResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AgentUpgradeResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AgentUpgradeResponse> get serializer => _$AgentUpgradeResponseSerializer();
}

class _$AgentUpgradeResponseSerializer implements PrimitiveSerializer<AgentUpgradeResponse> {
  @override
  final Iterable<Type> types = const [AgentUpgradeResponse, _$AgentUpgradeResponse];

  @override
  final String wireName = r'AgentUpgradeResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AgentUpgradeResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'agent_id';
    yield serializers.serialize(
      object.agentId,
      specifiedType: const FullType(String),
    );
    yield r'attempt_count';
    yield serializers.serialize(
      object.attemptCount,
      specifiedType: const FullType(int),
    );
    if (object.currentVersion != null) {
      yield r'current_version';
      yield serializers.serialize(
        object.currentVersion,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.errorCode != null) {
      yield r'error_code';
      yield serializers.serialize(
        object.errorCode,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.errorSummary != null) {
      yield r'error_summary';
      yield serializers.serialize(
        object.errorSummary,
        specifiedType: const FullType.nullable(String),
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
    yield r'node_id';
    yield serializers.serialize(
      object.nodeId,
      specifiedType: const FullType(String),
    );
    if (object.phase != null) {
      yield r'phase';
      yield serializers.serialize(
        object.phase,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'queued_at';
    yield serializers.serialize(
      object.queuedAt,
      specifiedType: const FullType(String),
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
    yield r'target_architecture';
    yield serializers.serialize(
      object.targetArchitecture,
      specifiedType: const FullType(String),
    );
    yield r'target_version';
    yield serializers.serialize(
      object.targetVersion,
      specifiedType: const FullType(String),
    );
    yield r'updated_at';
    yield serializers.serialize(
      object.updatedAt,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    AgentUpgradeResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required AgentUpgradeResponseBuilder result,
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
        case r'attempt_count':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.attemptCount = valueDes;
          break;
        case r'current_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.currentVersion = valueDes;
          break;
        case r'error_code':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.errorCode = valueDes;
          break;
        case r'error_summary':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.errorSummary = valueDes;
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
        case r'node_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.nodeId = valueDes;
          break;
        case r'phase':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.phase = valueDes;
          break;
        case r'queued_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.queuedAt = valueDes;
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
        case r'target_architecture':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.targetArchitecture = valueDes;
          break;
        case r'target_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.targetVersion = valueDes;
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.updatedAt = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  AgentUpgradeResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AgentUpgradeResponseBuilder();
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
