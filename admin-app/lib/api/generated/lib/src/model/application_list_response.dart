//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:deploy_go_api_client/src/model/application_response.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_list_response.g.dart';

/// ApplicationListResponse
///
/// Properties:
/// * [items]
/// * [nextCursor]
@BuiltValue()
abstract class ApplicationListResponse implements Built<ApplicationListResponse, ApplicationListResponseBuilder> {
  @BuiltValueField(wireName: r'items')
  BuiltList<ApplicationResponse> get items;

  @BuiltValueField(wireName: r'next_cursor')
  String? get nextCursor;

  ApplicationListResponse._();

  factory ApplicationListResponse([void updates(ApplicationListResponseBuilder b)]) = _$ApplicationListResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationListResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationListResponse> get serializer => _$ApplicationListResponseSerializer();
}

class _$ApplicationListResponseSerializer implements PrimitiveSerializer<ApplicationListResponse> {
  @override
  final Iterable<Type> types = const [ApplicationListResponse, _$ApplicationListResponse];

  @override
  final String wireName = r'ApplicationListResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'items';
    yield serializers.serialize(
      object.items,
      specifiedType: const FullType(BuiltList, [FullType(ApplicationResponse)]),
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
    ApplicationListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationListResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'items':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(ApplicationResponse)]),
          ) as BuiltList<ApplicationResponse>;
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
  ApplicationListResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationListResponseBuilder();
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
