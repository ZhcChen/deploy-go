//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:deploy_go_api_client/src/model/agent_response.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'agent_list_response.g.dart';

/// AgentListResponse
///
/// Properties:
/// * [items]
/// * [nextCursor]
@BuiltValue()
abstract class AgentListResponse implements Built<AgentListResponse, AgentListResponseBuilder> {
  @BuiltValueField(wireName: r'items')
  BuiltList<AgentResponse> get items;

  @BuiltValueField(wireName: r'next_cursor')
  String? get nextCursor;

  AgentListResponse._();

  factory AgentListResponse([void updates(AgentListResponseBuilder b)]) = _$AgentListResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AgentListResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AgentListResponse> get serializer => _$AgentListResponseSerializer();
}

class _$AgentListResponseSerializer implements PrimitiveSerializer<AgentListResponse> {
  @override
  final Iterable<Type> types = const [AgentListResponse, _$AgentListResponse];

  @override
  final String wireName = r'AgentListResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AgentListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'items';
    yield serializers.serialize(
      object.items,
      specifiedType: const FullType(BuiltList, [FullType(AgentResponse)]),
    );
    if (object.nextCursor != null) {
      yield r'next_cursor';
      yield serializers.serialize(
        object.nextCursor,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    AgentListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required AgentListResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'items':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(AgentResponse)]),
          ) as BuiltList<AgentResponse>;
          result.items.replace(valueDes);
          break;
        case r'next_cursor':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.nextCursor = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  AgentListResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AgentListResponseBuilder();
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
