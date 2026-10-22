//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'update_external_api_key_nodes_request.g.dart';

/// UpdateExternalApiKeyNodesRequest
///
/// Properties:
/// * [nodeIds]
@BuiltValue()
abstract class UpdateExternalApiKeyNodesRequest implements Built<UpdateExternalApiKeyNodesRequest, UpdateExternalApiKeyNodesRequestBuilder> {
  @BuiltValueField(wireName: r'node_ids')
  BuiltList<String> get nodeIds;

  UpdateExternalApiKeyNodesRequest._();

  factory UpdateExternalApiKeyNodesRequest([void updates(UpdateExternalApiKeyNodesRequestBuilder b)]) = _$UpdateExternalApiKeyNodesRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UpdateExternalApiKeyNodesRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UpdateExternalApiKeyNodesRequest> get serializer => _$UpdateExternalApiKeyNodesRequestSerializer();
}

class _$UpdateExternalApiKeyNodesRequestSerializer implements PrimitiveSerializer<UpdateExternalApiKeyNodesRequest> {
  @override
  final Iterable<Type> types = const [UpdateExternalApiKeyNodesRequest, _$UpdateExternalApiKeyNodesRequest];

  @override
  final String wireName = r'UpdateExternalApiKeyNodesRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UpdateExternalApiKeyNodesRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'node_ids';
    yield serializers.serialize(
      object.nodeIds,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    UpdateExternalApiKeyNodesRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required UpdateExternalApiKeyNodesRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'node_ids':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.nodeIds.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  UpdateExternalApiKeyNodesRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UpdateExternalApiKeyNodesRequestBuilder();
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
