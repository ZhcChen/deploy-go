//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/external_api_key_summary.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'external_api_key_list_response.g.dart';

/// ExternalApiKeyListResponse
///
/// Properties:
/// * [items]
/// * [nextCursor]
@BuiltValue()
abstract class ExternalApiKeyListResponse implements Built<ExternalApiKeyListResponse, ExternalApiKeyListResponseBuilder> {
  @BuiltValueField(wireName: r'items')
  BuiltList<ExternalApiKeySummary> get items;

  @BuiltValueField(wireName: r'next_cursor')
  String? get nextCursor;

  ExternalApiKeyListResponse._();

  factory ExternalApiKeyListResponse([void updates(ExternalApiKeyListResponseBuilder b)]) = _$ExternalApiKeyListResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ExternalApiKeyListResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ExternalApiKeyListResponse> get serializer => _$ExternalApiKeyListResponseSerializer();
}

class _$ExternalApiKeyListResponseSerializer implements PrimitiveSerializer<ExternalApiKeyListResponse> {
  @override
  final Iterable<Type> types = const [ExternalApiKeyListResponse, _$ExternalApiKeyListResponse];

  @override
  final String wireName = r'ExternalApiKeyListResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ExternalApiKeyListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'items';
    yield serializers.serialize(
      object.items,
      specifiedType: const FullType(BuiltList, [FullType(ExternalApiKeySummary)]),
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
    ExternalApiKeyListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ExternalApiKeyListResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'items':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(ExternalApiKeySummary)]),
          ) as BuiltList<ExternalApiKeySummary>;
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
  ExternalApiKeyListResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ExternalApiKeyListResponseBuilder();
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
