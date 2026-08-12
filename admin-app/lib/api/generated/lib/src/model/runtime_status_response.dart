//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'runtime_status_response.g.dart';

/// RuntimeStatusResponse
///
/// Properties:
/// * [applicationId]
/// * [createdAt]
/// * [errorCode]
/// * [errorMessage]
/// * [observedAt]
/// * [payload]
/// * [requestedAt]
/// * [requestedBy]
/// * [runtimeStatusId]
/// * [status]
/// * [targetCode]
/// * [targetId]
/// * [updatedAt]
@BuiltValue()
abstract class RuntimeStatusResponse implements Built<RuntimeStatusResponse, RuntimeStatusResponseBuilder> {
  @BuiltValueField(wireName: r'application_id')
  String get applicationId;

  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  @BuiltValueField(wireName: r'error_code')
  String? get errorCode;

  @BuiltValueField(wireName: r'error_message')
  String? get errorMessage;

  @BuiltValueField(wireName: r'observed_at')
  String? get observedAt;

  @BuiltValueField(wireName: r'payload')
  JsonObject? get payload;

  @BuiltValueField(wireName: r'requested_at')
  String get requestedAt;

  @BuiltValueField(wireName: r'requested_by')
  String? get requestedBy;

  @BuiltValueField(wireName: r'runtime_status_id')
  String get runtimeStatusId;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'target_code')
  String get targetCode;

  @BuiltValueField(wireName: r'target_id')
  String get targetId;

  @BuiltValueField(wireName: r'updated_at')
  String get updatedAt;

  RuntimeStatusResponse._();

  factory RuntimeStatusResponse([void updates(RuntimeStatusResponseBuilder b)]) = _$RuntimeStatusResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RuntimeStatusResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RuntimeStatusResponse> get serializer => _$RuntimeStatusResponseSerializer();
}

class _$RuntimeStatusResponseSerializer implements PrimitiveSerializer<RuntimeStatusResponse> {
  @override
  final Iterable<Type> types = const [RuntimeStatusResponse, _$RuntimeStatusResponse];

  @override
  final String wireName = r'RuntimeStatusResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RuntimeStatusResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'application_id';
    yield serializers.serialize(
      object.applicationId,
      specifiedType: const FullType(String),
    );
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(String),
    );
    if (object.errorCode != null) {
      yield r'error_code';
      yield serializers.serialize(
        object.errorCode,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.errorMessage != null) {
      yield r'error_message';
      yield serializers.serialize(
        object.errorMessage,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.observedAt != null) {
      yield r'observed_at';
      yield serializers.serialize(
        object.observedAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.payload != null) {
      yield r'payload';
      yield serializers.serialize(
        object.payload,
        specifiedType: const FullType.nullable(JsonObject),
      );
    }
    yield r'requested_at';
    yield serializers.serialize(
      object.requestedAt,
      specifiedType: const FullType(String),
    );
    if (object.requestedBy != null) {
      yield r'requested_by';
      yield serializers.serialize(
        object.requestedBy,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'runtime_status_id';
    yield serializers.serialize(
      object.runtimeStatusId,
      specifiedType: const FullType(String),
    );
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
    yield r'target_code';
    yield serializers.serialize(
      object.targetCode,
      specifiedType: const FullType(String),
    );
    yield r'target_id';
    yield serializers.serialize(
      object.targetId,
      specifiedType: const FullType(String),
    );
    yield r'updated_at';
    yield serializers.serialize(
      object.updatedAt,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RuntimeStatusResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required RuntimeStatusResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'application_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.applicationId = valueDes;
          break;
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.createdAt = valueDes;
          break;
        case r'error_code':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.errorCode = valueDes;
          break;
        case r'error_message':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.errorMessage = valueDes;
          break;
        case r'observed_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.observedAt = valueDes;
          break;
        case r'payload':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.payload = valueDes;
          break;
        case r'requested_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.requestedAt = valueDes;
          break;
        case r'requested_by':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.requestedBy = valueDes;
          break;
        case r'runtime_status_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.runtimeStatusId = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        case r'target_code':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.targetCode = valueDes;
          break;
        case r'target_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.targetId = valueDes;
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.updatedAt = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RuntimeStatusResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RuntimeStatusResponseBuilder();
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
