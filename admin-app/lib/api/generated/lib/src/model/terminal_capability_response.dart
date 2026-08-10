//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'terminal_capability_response.g.dart';

/// TerminalCapabilityResponse
///
/// Properties:
/// * [agentId]
/// * [agentOnline]
/// * [available]
/// * [identityValid]
/// * [nodeId]
/// * [privilegedExecution]
/// * [protocolVersion]
/// * [ptyTerminal]
/// * [unavailableCode]
@BuiltValue()
abstract class TerminalCapabilityResponse implements Built<TerminalCapabilityResponse, TerminalCapabilityResponseBuilder> {
  @BuiltValueField(wireName: r'agent_id')
  String? get agentId;

  @BuiltValueField(wireName: r'agent_online')
  bool get agentOnline;

  @BuiltValueField(wireName: r'available')
  bool get available;

  @BuiltValueField(wireName: r'identity_valid')
  bool get identityValid;

  @BuiltValueField(wireName: r'node_id')
  String get nodeId;

  @BuiltValueField(wireName: r'privileged_execution')
  bool get privilegedExecution;

  @BuiltValueField(wireName: r'protocol_version')
  int? get protocolVersion;

  @BuiltValueField(wireName: r'pty_terminal')
  bool get ptyTerminal;

  @BuiltValueField(wireName: r'unavailable_code')
  String? get unavailableCode;

  TerminalCapabilityResponse._();

  factory TerminalCapabilityResponse([void updates(TerminalCapabilityResponseBuilder b)]) = _$TerminalCapabilityResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(TerminalCapabilityResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<TerminalCapabilityResponse> get serializer => _$TerminalCapabilityResponseSerializer();
}

class _$TerminalCapabilityResponseSerializer implements PrimitiveSerializer<TerminalCapabilityResponse> {
  @override
  final Iterable<Type> types = const [TerminalCapabilityResponse, _$TerminalCapabilityResponse];

  @override
  final String wireName = r'TerminalCapabilityResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    TerminalCapabilityResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.agentId != null) {
      yield r'agent_id';
      yield serializers.serialize(
        object.agentId,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'agent_online';
    yield serializers.serialize(
      object.agentOnline,
      specifiedType: const FullType(bool),
    );
    yield r'available';
    yield serializers.serialize(
      object.available,
      specifiedType: const FullType(bool),
    );
    yield r'identity_valid';
    yield serializers.serialize(
      object.identityValid,
      specifiedType: const FullType(bool),
    );
    yield r'node_id';
    yield serializers.serialize(
      object.nodeId,
      specifiedType: const FullType(String),
    );
    yield r'privileged_execution';
    yield serializers.serialize(
      object.privilegedExecution,
      specifiedType: const FullType(bool),
    );
    if (object.protocolVersion != null) {
      yield r'protocol_version';
      yield serializers.serialize(
        object.protocolVersion,
        specifiedType: const FullType.nullable(int),
      );
    }
    yield r'pty_terminal';
    yield serializers.serialize(
      object.ptyTerminal,
      specifiedType: const FullType(bool),
    );
    if (object.unavailableCode != null) {
      yield r'unavailable_code';
      yield serializers.serialize(
        object.unavailableCode,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    TerminalCapabilityResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required TerminalCapabilityResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'agent_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.agentId = valueDes;
          break;
        case r'agent_online':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.agentOnline = valueDes;
          break;
        case r'available':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.available = valueDes;
          break;
        case r'identity_valid':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.identityValid = valueDes;
          break;
        case r'node_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.nodeId = valueDes;
          break;
        case r'privileged_execution':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.privilegedExecution = valueDes;
          break;
        case r'protocol_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.protocolVersion = valueDes;
          break;
        case r'pty_terminal':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.ptyTerminal = valueDes;
          break;
        case r'unavailable_code':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.unavailableCode = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  TerminalCapabilityResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = TerminalCapabilityResponseBuilder();
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
