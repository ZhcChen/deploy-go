//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_tag_list_response.g.dart';

/// ApplicationTagListResponse
///
/// Properties:
/// * [tags]
@BuiltValue()
abstract class ApplicationTagListResponse implements Built<ApplicationTagListResponse, ApplicationTagListResponseBuilder> {
  @BuiltValueField(wireName: r'tags')
  BuiltList<String> get tags;

  ApplicationTagListResponse._();

  factory ApplicationTagListResponse([void updates(ApplicationTagListResponseBuilder b)]) = _$ApplicationTagListResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationTagListResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationTagListResponse> get serializer => _$ApplicationTagListResponseSerializer();
}

class _$ApplicationTagListResponseSerializer implements PrimitiveSerializer<ApplicationTagListResponse> {
  @override
  final Iterable<Type> types = const [ApplicationTagListResponse, _$ApplicationTagListResponse];

  @override
  final String wireName = r'ApplicationTagListResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationTagListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'tags';
    yield serializers.serialize(
      object.tags,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ApplicationTagListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationTagListResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'tags':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.tags.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ApplicationTagListResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationTagListResponseBuilder();
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
