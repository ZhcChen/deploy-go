//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'runtime_probe_item_response.g.dart';

/// RuntimeProbeItemResponse
///
/// Properties:
/// * [applicationId]
/// * [errorCode]
/// * [errorMessage]
/// * [runtimeStatusId]
/// * [status] - queued | in_progress | failed | skipped
@BuiltValue()
abstract class RuntimeProbeItemResponse implements Built<RuntimeProbeItemResponse, RuntimeProbeItemResponseBuilder> {
  @BuiltValueField(wireName: r'application_id')
  String get applicationId;

  @BuiltValueField(wireName: r'error_code')
  String? get errorCode;

  @BuiltValueField(wireName: r'error_message')
  String? get errorMessage;

  @BuiltValueField(wireName: r'runtime_status_id')
  String? get runtimeStatusId;

  /// queued | in_progress | failed | skipped
  @BuiltValueField(wireName: r'status')
  String get status;

  RuntimeProbeItemResponse._();

  factory RuntimeProbeItemResponse([void updates(RuntimeProbeItemResponseBuilder b)]) = _$RuntimeProbeItemResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RuntimeProbeItemResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RuntimeProbeItemResponse> get serializer => _$RuntimeProbeItemResponseSerializer();
}

class _$RuntimeProbeItemResponseSerializer implements PrimitiveSerializer<RuntimeProbeItemResponse> {
  @override
  final Iterable<Type> types = const [RuntimeProbeItemResponse, _$RuntimeProbeItemResponse];

  @override
  final String wireName = r'RuntimeProbeItemResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RuntimeProbeItemResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'application_id';
    yield serializers.serialize(
      object.applicationId,
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
    if (object.runtimeStatusId != null) {
      yield r'runtime_status_id';
      yield serializers.serialize(
        object.runtimeStatusId,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RuntimeProbeItemResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required RuntimeProbeItemResponseBuilder result,
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
        case r'runtime_status_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.runtimeStatusId = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RuntimeProbeItemResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RuntimeProbeItemResponseBuilder();
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
