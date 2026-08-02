//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/audit_log_response.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'audit_log_list_response.g.dart';

/// AuditLogListResponse
///
/// Properties:
/// * [items]
/// * [nextCursor]
@BuiltValue()
abstract class AuditLogListResponse implements Built<AuditLogListResponse, AuditLogListResponseBuilder> {
  @BuiltValueField(wireName: r'items')
  BuiltList<AuditLogResponse> get items;

  @BuiltValueField(wireName: r'next_cursor')
  String? get nextCursor;

  AuditLogListResponse._();

  factory AuditLogListResponse([void updates(AuditLogListResponseBuilder b)]) = _$AuditLogListResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AuditLogListResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AuditLogListResponse> get serializer => _$AuditLogListResponseSerializer();
}

class _$AuditLogListResponseSerializer implements PrimitiveSerializer<AuditLogListResponse> {
  @override
  final Iterable<Type> types = const [AuditLogListResponse, _$AuditLogListResponse];

  @override
  final String wireName = r'AuditLogListResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AuditLogListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'items';
    yield serializers.serialize(
      object.items,
      specifiedType: const FullType(BuiltList, [FullType(AuditLogResponse)]),
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
    AuditLogListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required AuditLogListResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'items':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(AuditLogResponse)]),
          ) as BuiltList<AuditLogResponse>;
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
  AuditLogListResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AuditLogListResponseBuilder();
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
