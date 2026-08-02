//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/deployment_response.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'deployment_list_response.g.dart';

/// DeploymentListResponse
///
/// Properties:
/// * [items]
/// * [nextCursor]
@BuiltValue()
abstract class DeploymentListResponse implements Built<DeploymentListResponse, DeploymentListResponseBuilder> {
  @BuiltValueField(wireName: r'items')
  BuiltList<DeploymentResponse> get items;

  @BuiltValueField(wireName: r'next_cursor')
  String? get nextCursor;

  DeploymentListResponse._();

  factory DeploymentListResponse([void updates(DeploymentListResponseBuilder b)]) = _$DeploymentListResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(DeploymentListResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<DeploymentListResponse> get serializer => _$DeploymentListResponseSerializer();
}

class _$DeploymentListResponseSerializer implements PrimitiveSerializer<DeploymentListResponse> {
  @override
  final Iterable<Type> types = const [DeploymentListResponse, _$DeploymentListResponse];

  @override
  final String wireName = r'DeploymentListResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    DeploymentListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'items';
    yield serializers.serialize(
      object.items,
      specifiedType: const FullType(BuiltList, [FullType(DeploymentResponse)]),
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
    DeploymentListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required DeploymentListResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'items':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(DeploymentResponse)]),
          ) as BuiltList<DeploymentResponse>;
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
  DeploymentListResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = DeploymentListResponseBuilder();
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
