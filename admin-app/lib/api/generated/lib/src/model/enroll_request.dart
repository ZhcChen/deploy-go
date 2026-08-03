//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'enroll_request.g.dart';

/// EnrollRequest
///
/// Properties:
/// * [agentId]
/// * [agentVersion]
/// * [architecture]
/// * [enrollmentToken]
/// * [hostname]
/// * [os]
/// * [protocolVersion]
@BuiltValue()
abstract class EnrollRequest implements Built<EnrollRequest, EnrollRequestBuilder> {
  @BuiltValueField(wireName: r'agent_id')
  String get agentId;

  @BuiltValueField(wireName: r'agent_version')
  String get agentVersion;

  @BuiltValueField(wireName: r'architecture')
  String get architecture;

  @BuiltValueField(wireName: r'enrollment_token')
  String get enrollmentToken;

  @BuiltValueField(wireName: r'hostname')
  String get hostname;

  @BuiltValueField(wireName: r'os')
  String get os;

  @BuiltValueField(wireName: r'protocol_version')
  int get protocolVersion;

  EnrollRequest._();

  factory EnrollRequest([void updates(EnrollRequestBuilder b)]) = _$EnrollRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(EnrollRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<EnrollRequest> get serializer => _$EnrollRequestSerializer();
}

class _$EnrollRequestSerializer implements PrimitiveSerializer<EnrollRequest> {
  @override
  final Iterable<Type> types = const [EnrollRequest, _$EnrollRequest];

  @override
  final String wireName = r'EnrollRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    EnrollRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'agent_id';
    yield serializers.serialize(
      object.agentId,
      specifiedType: const FullType(String),
    );
    yield r'agent_version';
    yield serializers.serialize(
      object.agentVersion,
      specifiedType: const FullType(String),
    );
    yield r'architecture';
    yield serializers.serialize(
      object.architecture,
      specifiedType: const FullType(String),
    );
    yield r'enrollment_token';
    yield serializers.serialize(
      object.enrollmentToken,
      specifiedType: const FullType(String),
    );
    yield r'hostname';
    yield serializers.serialize(
      object.hostname,
      specifiedType: const FullType(String),
    );
    yield r'os';
    yield serializers.serialize(
      object.os,
      specifiedType: const FullType(String),
    );
    yield r'protocol_version';
    yield serializers.serialize(
      object.protocolVersion,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    EnrollRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required EnrollRequestBuilder result,
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
        case r'agent_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.agentVersion = valueDes;
          break;
        case r'architecture':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.architecture = valueDes;
          break;
        case r'enrollment_token':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.enrollmentToken = valueDes;
          break;
        case r'hostname':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.hostname = valueDes;
          break;
        case r'os':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.os = valueDes;
          break;
        case r'protocol_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.protocolVersion = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  EnrollRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = EnrollRequestBuilder();
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
