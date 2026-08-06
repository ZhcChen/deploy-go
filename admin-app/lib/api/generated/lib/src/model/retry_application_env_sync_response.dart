//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'retry_application_env_sync_response.g.dart';

/// RetryApplicationEnvSyncResponse
///
/// Properties:
/// * [retried]
@BuiltValue()
abstract class RetryApplicationEnvSyncResponse implements Built<RetryApplicationEnvSyncResponse, RetryApplicationEnvSyncResponseBuilder> {
  @BuiltValueField(wireName: r'retried')
  int get retried;

  RetryApplicationEnvSyncResponse._();

  factory RetryApplicationEnvSyncResponse([void updates(RetryApplicationEnvSyncResponseBuilder b)]) = _$RetryApplicationEnvSyncResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RetryApplicationEnvSyncResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RetryApplicationEnvSyncResponse> get serializer => _$RetryApplicationEnvSyncResponseSerializer();
}

class _$RetryApplicationEnvSyncResponseSerializer implements PrimitiveSerializer<RetryApplicationEnvSyncResponse> {
  @override
  final Iterable<Type> types = const [RetryApplicationEnvSyncResponse, _$RetryApplicationEnvSyncResponse];

  @override
  final String wireName = r'RetryApplicationEnvSyncResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RetryApplicationEnvSyncResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'retried';
    yield serializers.serialize(
      object.retried,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RetryApplicationEnvSyncResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required RetryApplicationEnvSyncResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'retried':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.retried = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RetryApplicationEnvSyncResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RetryApplicationEnvSyncResponseBuilder();
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
