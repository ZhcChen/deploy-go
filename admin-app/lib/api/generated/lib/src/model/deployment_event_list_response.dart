//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/deployment_event_response.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'deployment_event_list_response.g.dart';

/// DeploymentEventListResponse
///
/// Properties:
/// * [items]
/// * [nextCursor]
@BuiltValue()
abstract class DeploymentEventListResponse implements Built<DeploymentEventListResponse, DeploymentEventListResponseBuilder> {
  @BuiltValueField(wireName: r'items')
  BuiltList<DeploymentEventResponse> get items;

  @BuiltValueField(wireName: r'next_cursor')
  String? get nextCursor;

  DeploymentEventListResponse._();

  factory DeploymentEventListResponse([void updates(DeploymentEventListResponseBuilder b)]) = _$DeploymentEventListResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(DeploymentEventListResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<DeploymentEventListResponse> get serializer => _$DeploymentEventListResponseSerializer();
}

class _$DeploymentEventListResponseSerializer implements PrimitiveSerializer<DeploymentEventListResponse> {
  @override
  final Iterable<Type> types = const [DeploymentEventListResponse, _$DeploymentEventListResponse];

  @override
  final String wireName = r'DeploymentEventListResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    DeploymentEventListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'items';
    yield serializers.serialize(
      object.items,
      specifiedType: const FullType(BuiltList, [FullType(DeploymentEventResponse)]),
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
    DeploymentEventListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required DeploymentEventListResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'items':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(DeploymentEventResponse)]),
          ) as BuiltList<DeploymentEventResponse>;
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
  DeploymentEventListResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = DeploymentEventListResponseBuilder();
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
