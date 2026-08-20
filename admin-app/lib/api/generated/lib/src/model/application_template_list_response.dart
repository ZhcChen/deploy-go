//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/application_template_response.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_template_list_response.g.dart';

/// ApplicationTemplateListResponse
///
/// Properties:
/// * [items]
@BuiltValue()
abstract class ApplicationTemplateListResponse implements Built<ApplicationTemplateListResponse, ApplicationTemplateListResponseBuilder> {
  @BuiltValueField(wireName: r'items')
  BuiltList<ApplicationTemplateResponse> get items;

  ApplicationTemplateListResponse._();

  factory ApplicationTemplateListResponse([void updates(ApplicationTemplateListResponseBuilder b)]) = _$ApplicationTemplateListResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationTemplateListResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationTemplateListResponse> get serializer => _$ApplicationTemplateListResponseSerializer();
}

class _$ApplicationTemplateListResponseSerializer implements PrimitiveSerializer<ApplicationTemplateListResponse> {
  @override
  final Iterable<Type> types = const [ApplicationTemplateListResponse, _$ApplicationTemplateListResponse];

  @override
  final String wireName = r'ApplicationTemplateListResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationTemplateListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'items';
    yield serializers.serialize(
      object.items,
      specifiedType: const FullType(BuiltList, [FullType(ApplicationTemplateResponse)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ApplicationTemplateListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationTemplateListResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'items':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(ApplicationTemplateResponse)]),
          ) as BuiltList<ApplicationTemplateResponse>;
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
  ApplicationTemplateListResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationTemplateListResponseBuilder();
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
