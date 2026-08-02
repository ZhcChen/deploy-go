//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/application_grant_response.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_grant_list_response.g.dart';

/// ApplicationGrantListResponse
///
/// Properties:
/// * [items]
/// * [nextCursor]
@BuiltValue()
abstract class ApplicationGrantListResponse implements Built<ApplicationGrantListResponse, ApplicationGrantListResponseBuilder> {
  @BuiltValueField(wireName: r'items')
  BuiltList<ApplicationGrantResponse> get items;

  @BuiltValueField(wireName: r'next_cursor')
  String? get nextCursor;

  ApplicationGrantListResponse._();

  factory ApplicationGrantListResponse([void updates(ApplicationGrantListResponseBuilder b)]) = _$ApplicationGrantListResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationGrantListResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationGrantListResponse> get serializer => _$ApplicationGrantListResponseSerializer();
}

class _$ApplicationGrantListResponseSerializer implements PrimitiveSerializer<ApplicationGrantListResponse> {
  @override
  final Iterable<Type> types = const [ApplicationGrantListResponse, _$ApplicationGrantListResponse];

  @override
  final String wireName = r'ApplicationGrantListResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationGrantListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'items';
    yield serializers.serialize(
      object.items,
      specifiedType: const FullType(BuiltList, [FullType(ApplicationGrantResponse)]),
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
    ApplicationGrantListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationGrantListResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'items':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(ApplicationGrantResponse)]),
          ) as BuiltList<ApplicationGrantResponse>;
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
  ApplicationGrantListResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationGrantListResponseBuilder();
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
