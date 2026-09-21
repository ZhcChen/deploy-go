//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'agent_upgrade_summary.g.dart';

/// AgentUpgradeSummary
///
/// Properties:
/// * [currentVersion]
/// * [errorCode]
/// * [errorSummary]
/// * [jobId]
/// * [phase]
/// * [state]
/// * [targetVersion]
/// * [updatedAt]
@BuiltValue()
abstract class AgentUpgradeSummary implements Built<AgentUpgradeSummary, AgentUpgradeSummaryBuilder> {
  @BuiltValueField(wireName: r'current_version')
  String? get currentVersion;

  @BuiltValueField(wireName: r'error_code')
  String? get errorCode;

  @BuiltValueField(wireName: r'error_summary')
  String? get errorSummary;

  @BuiltValueField(wireName: r'job_id')
  String? get jobId;

  @BuiltValueField(wireName: r'phase')
  String? get phase;

  @BuiltValueField(wireName: r'state')
  String get state;

  @BuiltValueField(wireName: r'target_version')
  String? get targetVersion;

  @BuiltValueField(wireName: r'updated_at')
  String? get updatedAt;

  AgentUpgradeSummary._();

  factory AgentUpgradeSummary([void updates(AgentUpgradeSummaryBuilder b)]) = _$AgentUpgradeSummary;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AgentUpgradeSummaryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AgentUpgradeSummary> get serializer => _$AgentUpgradeSummarySerializer();
}

class _$AgentUpgradeSummarySerializer implements PrimitiveSerializer<AgentUpgradeSummary> {
  @override
  final Iterable<Type> types = const [AgentUpgradeSummary, _$AgentUpgradeSummary];

  @override
  final String wireName = r'AgentUpgradeSummary';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AgentUpgradeSummary object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
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
    if (object.jobId != null) {
      yield r'job_id';
      yield serializers.serialize(
        object.jobId,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.phase != null) {
      yield r'phase';
      yield serializers.serialize(
        object.phase,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'state';
    yield serializers.serialize(
      object.state,
      specifiedType: const FullType(String),
    );
    if (object.targetVersion != null) {
      yield r'target_version';
      yield serializers.serialize(
        object.targetVersion,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.updatedAt != null) {
      yield r'updated_at';
      yield serializers.serialize(
        object.updatedAt,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    AgentUpgradeSummary object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required AgentUpgradeSummaryBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
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
        case r'job_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.jobId = valueDes;
          break;
        case r'phase':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.phase = valueDes;
          break;
        case r'state':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.state = valueDes;
          break;
        case r'target_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.targetVersion = valueDes;
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
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
  AgentUpgradeSummary deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AgentUpgradeSummaryBuilder();
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
