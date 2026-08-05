//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'agent_release_response.g.dart';

/// AgentReleaseResponse
///
/// Properties:
/// * [active]
/// * [protocolMaximum]
/// * [protocolMinimum]
/// * [version]
@BuiltValue()
abstract class AgentReleaseResponse implements Built<AgentReleaseResponse, AgentReleaseResponseBuilder> {
  @BuiltValueField(wireName: r'active')
  bool get active;

  @BuiltValueField(wireName: r'protocol_maximum')
  int get protocolMaximum;

  @BuiltValueField(wireName: r'protocol_minimum')
  int get protocolMinimum;

  @BuiltValueField(wireName: r'version')
  String get version;

  AgentReleaseResponse._();

  factory AgentReleaseResponse([void updates(AgentReleaseResponseBuilder b)]) = _$AgentReleaseResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AgentReleaseResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AgentReleaseResponse> get serializer => _$AgentReleaseResponseSerializer();
}

class _$AgentReleaseResponseSerializer implements PrimitiveSerializer<AgentReleaseResponse> {
  @override
  final Iterable<Type> types = const [AgentReleaseResponse, _$AgentReleaseResponse];

  @override
  final String wireName = r'AgentReleaseResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AgentReleaseResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'active';
    yield serializers.serialize(
      object.active,
      specifiedType: const FullType(bool),
    );
    yield r'protocol_maximum';
    yield serializers.serialize(
      object.protocolMaximum,
      specifiedType: const FullType(int),
    );
    yield r'protocol_minimum';
    yield serializers.serialize(
      object.protocolMinimum,
      specifiedType: const FullType(int),
    );
    yield r'version';
    yield serializers.serialize(
      object.version,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    AgentReleaseResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required AgentReleaseResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'active':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.active = valueDes;
          break;
        case r'protocol_maximum':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.protocolMaximum = valueDes;
          break;
        case r'protocol_minimum':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.protocolMinimum = valueDes;
          break;
        case r'version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
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
  AgentReleaseResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AgentReleaseResponseBuilder();
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
