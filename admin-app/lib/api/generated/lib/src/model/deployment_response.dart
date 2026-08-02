//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'deployment_response.g.dart';

/// DeploymentResponse
///
/// Properties:
/// * [cancelRequestedAt]
/// * [createdAt]
/// * [exitCode]
/// * [finishedAt]
/// * [id]
/// * [phase]
/// * [protocolComplete]
/// * [queuedAt]
/// * [requestedBy]
/// * [resultSummary]
/// * [retryOfId]
/// * [snapshotHash]
/// * [startedAt]
/// * [status]
/// * [targetId]
/// * [updatedAt]
/// * [version]
@BuiltValue()
abstract class DeploymentResponse implements Built<DeploymentResponse, DeploymentResponseBuilder> {
  @BuiltValueField(wireName: r'cancel_requested_at')
  String? get cancelRequestedAt;

  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  @BuiltValueField(wireName: r'exit_code')
  int? get exitCode;

  @BuiltValueField(wireName: r'finished_at')
  String? get finishedAt;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'phase')
  String get phase;

  @BuiltValueField(wireName: r'protocol_complete')
  bool get protocolComplete;

  @BuiltValueField(wireName: r'queued_at')
  String get queuedAt;

  @BuiltValueField(wireName: r'requested_by')
  String get requestedBy;

  @BuiltValueField(wireName: r'result_summary')
  String? get resultSummary;

  @BuiltValueField(wireName: r'retry_of_id')
  String? get retryOfId;

  @BuiltValueField(wireName: r'snapshot_hash')
  String get snapshotHash;

  @BuiltValueField(wireName: r'started_at')
  String? get startedAt;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'target_id')
  String get targetId;

  @BuiltValueField(wireName: r'updated_at')
  String get updatedAt;

  @BuiltValueField(wireName: r'version')
  int get version;

  DeploymentResponse._();

  factory DeploymentResponse([void updates(DeploymentResponseBuilder b)]) = _$DeploymentResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(DeploymentResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<DeploymentResponse> get serializer => _$DeploymentResponseSerializer();
}

class _$DeploymentResponseSerializer implements PrimitiveSerializer<DeploymentResponse> {
  @override
  final Iterable<Type> types = const [DeploymentResponse, _$DeploymentResponse];

  @override
  final String wireName = r'DeploymentResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    DeploymentResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.cancelRequestedAt != null) {
      yield r'cancel_requested_at';
      yield serializers.serialize(
        object.cancelRequestedAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(String),
    );
    if (object.exitCode != null) {
      yield r'exit_code';
      yield serializers.serialize(
        object.exitCode,
        specifiedType: const FullType.nullable(int),
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
    yield r'phase';
    yield serializers.serialize(
      object.phase,
      specifiedType: const FullType(String),
    );
    yield r'protocol_complete';
    yield serializers.serialize(
      object.protocolComplete,
      specifiedType: const FullType(bool),
    );
    yield r'queued_at';
    yield serializers.serialize(
      object.queuedAt,
      specifiedType: const FullType(String),
    );
    yield r'requested_by';
    yield serializers.serialize(
      object.requestedBy,
      specifiedType: const FullType(String),
    );
    if (object.resultSummary != null) {
      yield r'result_summary';
      yield serializers.serialize(
        object.resultSummary,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.retryOfId != null) {
      yield r'retry_of_id';
      yield serializers.serialize(
        object.retryOfId,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'snapshot_hash';
    yield serializers.serialize(
      object.snapshotHash,
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
    yield r'target_id';
    yield serializers.serialize(
      object.targetId,
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
    DeploymentResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required DeploymentResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'cancel_requested_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.cancelRequestedAt = valueDes;
          break;
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.createdAt = valueDes;
          break;
        case r'exit_code':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.exitCode = valueDes;
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
        case r'phase':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.phase = valueDes;
          break;
        case r'protocol_complete':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.protocolComplete = valueDes;
          break;
        case r'queued_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.queuedAt = valueDes;
          break;
        case r'requested_by':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.requestedBy = valueDes;
          break;
        case r'result_summary':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.resultSummary = valueDes;
          break;
        case r'retry_of_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.retryOfId = valueDes;
          break;
        case r'snapshot_hash':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.snapshotHash = valueDes;
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
        case r'target_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.targetId = valueDes;
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
  DeploymentResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = DeploymentResponseBuilder();
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
