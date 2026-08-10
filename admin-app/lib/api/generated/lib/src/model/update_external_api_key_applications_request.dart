//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'update_external_api_key_applications_request.g.dart';

/// UpdateExternalApiKeyApplicationsRequest
///
/// Properties:
/// * [applicationIds]
@BuiltValue()
abstract class UpdateExternalApiKeyApplicationsRequest implements Built<UpdateExternalApiKeyApplicationsRequest, UpdateExternalApiKeyApplicationsRequestBuilder> {
  @BuiltValueField(wireName: r'application_ids')
  BuiltList<String> get applicationIds;

  UpdateExternalApiKeyApplicationsRequest._();

  factory UpdateExternalApiKeyApplicationsRequest([void updates(UpdateExternalApiKeyApplicationsRequestBuilder b)]) = _$UpdateExternalApiKeyApplicationsRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UpdateExternalApiKeyApplicationsRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UpdateExternalApiKeyApplicationsRequest> get serializer => _$UpdateExternalApiKeyApplicationsRequestSerializer();
}

class _$UpdateExternalApiKeyApplicationsRequestSerializer implements PrimitiveSerializer<UpdateExternalApiKeyApplicationsRequest> {
  @override
  final Iterable<Type> types = const [UpdateExternalApiKeyApplicationsRequest, _$UpdateExternalApiKeyApplicationsRequest];

  @override
  final String wireName = r'UpdateExternalApiKeyApplicationsRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UpdateExternalApiKeyApplicationsRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'application_ids';
    yield serializers.serialize(
      object.applicationIds,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    UpdateExternalApiKeyApplicationsRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required UpdateExternalApiKeyApplicationsRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'application_ids':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.applicationIds.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  UpdateExternalApiKeyApplicationsRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UpdateExternalApiKeyApplicationsRequestBuilder();
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
