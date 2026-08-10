//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'terminal_session_response.g.dart';

/// TerminalSessionResponse
///
/// Properties:
/// * [actorId]
/// * [agentId]
/// * [closeRequestedAt]
/// * [exitCode]
/// * [exitReason]
/// * [finishedAt]
/// * [id]
/// * [inputBytes]
/// * [nodeId]
/// * [openedAt]
/// * [outputBytes]
/// * [startedAt]
/// * [status]
@BuiltValue()
abstract class TerminalSessionResponse implements Built<TerminalSessionResponse, TerminalSessionResponseBuilder> {
  @BuiltValueField(wireName: r'actor_id')
  String get actorId;

  @BuiltValueField(wireName: r'agent_id')
  String get agentId;

  @BuiltValueField(wireName: r'close_requested_at')
  String? get closeRequestedAt;

  @BuiltValueField(wireName: r'exit_code')
  int? get exitCode;

  @BuiltValueField(wireName: r'exit_reason')
  String? get exitReason;

  @BuiltValueField(wireName: r'finished_at')
  String? get finishedAt;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'input_bytes')
  int get inputBytes;

  @BuiltValueField(wireName: r'node_id')
  String get nodeId;

  @BuiltValueField(wireName: r'opened_at')
  String? get openedAt;

  @BuiltValueField(wireName: r'output_bytes')
  int get outputBytes;

  @BuiltValueField(wireName: r'started_at')
  String get startedAt;

  @BuiltValueField(wireName: r'status')
  String get status;

  TerminalSessionResponse._();

  factory TerminalSessionResponse([void updates(TerminalSessionResponseBuilder b)]) = _$TerminalSessionResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(TerminalSessionResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<TerminalSessionResponse> get serializer => _$TerminalSessionResponseSerializer();
}

class _$TerminalSessionResponseSerializer implements PrimitiveSerializer<TerminalSessionResponse> {
  @override
  final Iterable<Type> types = const [TerminalSessionResponse, _$TerminalSessionResponse];

  @override
  final String wireName = r'TerminalSessionResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    TerminalSessionResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'actor_id';
    yield serializers.serialize(
      object.actorId,
      specifiedType: const FullType(String),
    );
    yield r'agent_id';
    yield serializers.serialize(
      object.agentId,
      specifiedType: const FullType(String),
    );
    if (object.closeRequestedAt != null) {
      yield r'close_requested_at';
      yield serializers.serialize(
        object.closeRequestedAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.exitCode != null) {
      yield r'exit_code';
      yield serializers.serialize(
        object.exitCode,
        specifiedType: const FullType.nullable(int),
      );
    }
    if (object.exitReason != null) {
      yield r'exit_reason';
      yield serializers.serialize(
        object.exitReason,
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
    yield r'input_bytes';
    yield serializers.serialize(
      object.inputBytes,
      specifiedType: const FullType(int),
    );
    yield r'node_id';
    yield serializers.serialize(
      object.nodeId,
      specifiedType: const FullType(String),
    );
    if (object.openedAt != null) {
      yield r'opened_at';
      yield serializers.serialize(
        object.openedAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'output_bytes';
    yield serializers.serialize(
      object.outputBytes,
      specifiedType: const FullType(int),
    );
    yield r'started_at';
    yield serializers.serialize(
      object.startedAt,
      specifiedType: const FullType(String),
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
    TerminalSessionResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required TerminalSessionResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'actor_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.actorId = valueDes;
          break;
        case r'agent_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.agentId = valueDes;
          break;
        case r'close_requested_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.closeRequestedAt = valueDes;
          break;
        case r'exit_code':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.exitCode = valueDes;
          break;
        case r'exit_reason':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.exitReason = valueDes;
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
        case r'input_bytes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.inputBytes = valueDes;
          break;
        case r'node_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.nodeId = valueDes;
          break;
        case r'opened_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.openedAt = valueDes;
          break;
        case r'output_bytes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.outputBytes = valueDes;
          break;
        case r'started_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.startedAt = valueDes;
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
  TerminalSessionResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = TerminalSessionResponseBuilder();
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
