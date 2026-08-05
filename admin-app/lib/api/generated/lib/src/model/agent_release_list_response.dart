//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:deploy_go_api_client/src/model/agent_release_response.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'agent_release_list_response.g.dart';

/// AgentReleaseListResponse
///
/// Properties:
/// * [currentVersion]
/// * [items]
@BuiltValue()
abstract class AgentReleaseListResponse implements Built<AgentReleaseListResponse, AgentReleaseListResponseBuilder> {
  @BuiltValueField(wireName: r'current_version')
  String? get currentVersion;

  @BuiltValueField(wireName: r'items')
  BuiltList<AgentReleaseResponse> get items;

  AgentReleaseListResponse._();

  factory AgentReleaseListResponse([void updates(AgentReleaseListResponseBuilder b)]) = _$AgentReleaseListResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AgentReleaseListResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AgentReleaseListResponse> get serializer => _$AgentReleaseListResponseSerializer();
}

class _$AgentReleaseListResponseSerializer implements PrimitiveSerializer<AgentReleaseListResponse> {
  @override
  final Iterable<Type> types = const [AgentReleaseListResponse, _$AgentReleaseListResponse];

  @override
  final String wireName = r'AgentReleaseListResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AgentReleaseListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.currentVersion != null) {
      yield r'current_version';
      yield serializers.serialize(
        object.currentVersion,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'items';
    yield serializers.serialize(
      object.items,
      specifiedType: const FullType(BuiltList, [FullType(AgentReleaseResponse)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    AgentReleaseListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required AgentReleaseListResponseBuilder result,
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
        case r'items':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(AgentReleaseResponse)]),
          ) as BuiltList<AgentReleaseResponse>;
          result.items.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  AgentReleaseListResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AgentReleaseListResponseBuilder();
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
