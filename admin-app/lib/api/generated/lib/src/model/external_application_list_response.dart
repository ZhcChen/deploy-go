//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/external_application_summary.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'external_application_list_response.g.dart';

/// ExternalApplicationListResponse
///
/// Properties:
/// * [items]
@BuiltValue()
abstract class ExternalApplicationListResponse implements Built<ExternalApplicationListResponse, ExternalApplicationListResponseBuilder> {
  @BuiltValueField(wireName: r'items')
  BuiltList<ExternalApplicationSummary> get items;

  ExternalApplicationListResponse._();

  factory ExternalApplicationListResponse([void updates(ExternalApplicationListResponseBuilder b)]) = _$ExternalApplicationListResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ExternalApplicationListResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ExternalApplicationListResponse> get serializer => _$ExternalApplicationListResponseSerializer();
}

class _$ExternalApplicationListResponseSerializer implements PrimitiveSerializer<ExternalApplicationListResponse> {
  @override
  final Iterable<Type> types = const [ExternalApplicationListResponse, _$ExternalApplicationListResponse];

  @override
  final String wireName = r'ExternalApplicationListResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ExternalApplicationListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'items';
    yield serializers.serialize(
      object.items,
      specifiedType: const FullType(BuiltList, [FullType(ExternalApplicationSummary)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ExternalApplicationListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ExternalApplicationListResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'items':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(ExternalApplicationSummary)]),
          ) as BuiltList<ExternalApplicationSummary>;
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
  ExternalApplicationListResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ExternalApplicationListResponseBuilder();
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
