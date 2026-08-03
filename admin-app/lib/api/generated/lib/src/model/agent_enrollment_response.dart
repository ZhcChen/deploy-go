//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:deploy_go_api_client/src/model/agent_response.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'agent_enrollment_response.g.dart';

/// AgentEnrollmentResponse
///
/// Properties:
/// * [agent]
/// * [enrollmentExpiresAt]
/// * [enrollmentToken]
/// * [installCommand]
@BuiltValue()
abstract class AgentEnrollmentResponse implements Built<AgentEnrollmentResponse, AgentEnrollmentResponseBuilder> {
  @BuiltValueField(wireName: r'agent')
  AgentResponse get agent;

  @BuiltValueField(wireName: r'enrollment_expires_at')
  String get enrollmentExpiresAt;

  @BuiltValueField(wireName: r'enrollment_token')
  String get enrollmentToken;

  @BuiltValueField(wireName: r'install_command')
  String get installCommand;

  AgentEnrollmentResponse._();

  factory AgentEnrollmentResponse([void updates(AgentEnrollmentResponseBuilder b)]) = _$AgentEnrollmentResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AgentEnrollmentResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AgentEnrollmentResponse> get serializer => _$AgentEnrollmentResponseSerializer();
}

class _$AgentEnrollmentResponseSerializer implements PrimitiveSerializer<AgentEnrollmentResponse> {
  @override
  final Iterable<Type> types = const [AgentEnrollmentResponse, _$AgentEnrollmentResponse];

  @override
  final String wireName = r'AgentEnrollmentResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AgentEnrollmentResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'agent';
    yield serializers.serialize(
      object.agent,
      specifiedType: const FullType(AgentResponse),
    );
    yield r'enrollment_expires_at';
    yield serializers.serialize(
      object.enrollmentExpiresAt,
      specifiedType: const FullType(String),
    );
    yield r'enrollment_token';
    yield serializers.serialize(
      object.enrollmentToken,
      specifiedType: const FullType(String),
    );
    yield r'install_command';
    yield serializers.serialize(
      object.installCommand,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    AgentEnrollmentResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required AgentEnrollmentResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'agent':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(AgentResponse),
          ) as AgentResponse;
          result.agent.replace(valueDes);
          break;
        case r'enrollment_expires_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.enrollmentExpiresAt = valueDes;
          break;
        case r'enrollment_token':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.enrollmentToken = valueDes;
          break;
        case r'install_command':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.installCommand = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  AgentEnrollmentResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AgentEnrollmentResponseBuilder();
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
