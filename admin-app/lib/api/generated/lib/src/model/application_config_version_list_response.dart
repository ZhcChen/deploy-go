//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:deploy_go_api_client/src/model/application_config_version_response.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_config_version_list_response.g.dart';

/// ApplicationConfigVersionListResponse
///
/// Properties:
/// * [items]
@BuiltValue()
abstract class ApplicationConfigVersionListResponse implements Built<ApplicationConfigVersionListResponse, ApplicationConfigVersionListResponseBuilder> {
  @BuiltValueField(wireName: r'items')
  BuiltList<ApplicationConfigVersionResponse> get items;

  ApplicationConfigVersionListResponse._();

  factory ApplicationConfigVersionListResponse([void updates(ApplicationConfigVersionListResponseBuilder b)]) = _$ApplicationConfigVersionListResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationConfigVersionListResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationConfigVersionListResponse> get serializer => _$ApplicationConfigVersionListResponseSerializer();
}

class _$ApplicationConfigVersionListResponseSerializer implements PrimitiveSerializer<ApplicationConfigVersionListResponse> {
  @override
  final Iterable<Type> types = const [ApplicationConfigVersionListResponse, _$ApplicationConfigVersionListResponse];

  @override
  final String wireName = r'ApplicationConfigVersionListResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationConfigVersionListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'items';
    yield serializers.serialize(
      object.items,
      specifiedType: const FullType(BuiltList, [FullType(ApplicationConfigVersionResponse)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ApplicationConfigVersionListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationConfigVersionListResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'items':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(ApplicationConfigVersionResponse)]),
          ) as BuiltList<ApplicationConfigVersionResponse>;
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
  ApplicationConfigVersionListResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationConfigVersionListResponseBuilder();
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
