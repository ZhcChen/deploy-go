//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'update_privileged_execution_request.g.dart';

/// UpdatePrivilegedExecutionRequest
///
/// Properties:
/// * [enabled]
@BuiltValue()
abstract class UpdatePrivilegedExecutionRequest implements Built<UpdatePrivilegedExecutionRequest, UpdatePrivilegedExecutionRequestBuilder> {
  @BuiltValueField(wireName: r'enabled')
  bool get enabled;

  UpdatePrivilegedExecutionRequest._();

  factory UpdatePrivilegedExecutionRequest([void updates(UpdatePrivilegedExecutionRequestBuilder b)]) = _$UpdatePrivilegedExecutionRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UpdatePrivilegedExecutionRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UpdatePrivilegedExecutionRequest> get serializer => _$UpdatePrivilegedExecutionRequestSerializer();
}

class _$UpdatePrivilegedExecutionRequestSerializer implements PrimitiveSerializer<UpdatePrivilegedExecutionRequest> {
  @override
  final Iterable<Type> types = const [UpdatePrivilegedExecutionRequest, _$UpdatePrivilegedExecutionRequest];

  @override
  final String wireName = r'UpdatePrivilegedExecutionRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UpdatePrivilegedExecutionRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'enabled';
    yield serializers.serialize(
      object.enabled,
      specifiedType: const FullType(bool),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    UpdatePrivilegedExecutionRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required UpdatePrivilegedExecutionRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'enabled':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.enabled = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  UpdatePrivilegedExecutionRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UpdatePrivilegedExecutionRequestBuilder();
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
