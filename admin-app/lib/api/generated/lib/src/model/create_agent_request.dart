//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'create_agent_request.g.dart';

/// CreateAgentRequest
///
/// Properties:
/// * [name]
/// * [nodeId]
@BuiltValue()
abstract class CreateAgentRequest implements Built<CreateAgentRequest, CreateAgentRequestBuilder> {
  @BuiltValueField(wireName: r'name')
  String get name;

  @BuiltValueField(wireName: r'node_id')
  String? get nodeId;

  CreateAgentRequest._();

  factory CreateAgentRequest([void updates(CreateAgentRequestBuilder b)]) = _$CreateAgentRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CreateAgentRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CreateAgentRequest> get serializer => _$CreateAgentRequestSerializer();
}

class _$CreateAgentRequestSerializer implements PrimitiveSerializer<CreateAgentRequest> {
  @override
  final Iterable<Type> types = const [CreateAgentRequest, _$CreateAgentRequest];

  @override
  final String wireName = r'CreateAgentRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CreateAgentRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    if (object.nodeId != null) {
      yield r'node_id';
      yield serializers.serialize(
        object.nodeId,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    CreateAgentRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required CreateAgentRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'node_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.nodeId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CreateAgentRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CreateAgentRequestBuilder();
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
