//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/deployment_target_response.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'deployment_target_list_response.g.dart';

/// DeploymentTargetListResponse
///
/// Properties:
/// * [items]
/// * [nextCursor]
@BuiltValue()
abstract class DeploymentTargetListResponse implements Built<DeploymentTargetListResponse, DeploymentTargetListResponseBuilder> {
  @BuiltValueField(wireName: r'items')
  BuiltList<DeploymentTargetResponse> get items;

  @BuiltValueField(wireName: r'next_cursor')
  String? get nextCursor;

  DeploymentTargetListResponse._();

  factory DeploymentTargetListResponse([void updates(DeploymentTargetListResponseBuilder b)]) = _$DeploymentTargetListResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(DeploymentTargetListResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<DeploymentTargetListResponse> get serializer => _$DeploymentTargetListResponseSerializer();
}

class _$DeploymentTargetListResponseSerializer implements PrimitiveSerializer<DeploymentTargetListResponse> {
  @override
  final Iterable<Type> types = const [DeploymentTargetListResponse, _$DeploymentTargetListResponse];

  @override
  final String wireName = r'DeploymentTargetListResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    DeploymentTargetListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'items';
    yield serializers.serialize(
      object.items,
      specifiedType: const FullType(BuiltList, [FullType(DeploymentTargetResponse)]),
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
    DeploymentTargetListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required DeploymentTargetListResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'items':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(DeploymentTargetResponse)]),
          ) as BuiltList<DeploymentTargetResponse>;
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
  DeploymentTargetListResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = DeploymentTargetListResponseBuilder();
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
