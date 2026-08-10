//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'privileged_execution_response.g.dart';

/// PrivilegedExecutionResponse
///
/// Properties:
/// * [enabled]
/// * [nodeId]
@BuiltValue()
abstract class PrivilegedExecutionResponse implements Built<PrivilegedExecutionResponse, PrivilegedExecutionResponseBuilder> {
  @BuiltValueField(wireName: r'enabled')
  bool get enabled;

  @BuiltValueField(wireName: r'node_id')
  String get nodeId;

  PrivilegedExecutionResponse._();

  factory PrivilegedExecutionResponse([void updates(PrivilegedExecutionResponseBuilder b)]) = _$PrivilegedExecutionResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(PrivilegedExecutionResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<PrivilegedExecutionResponse> get serializer => _$PrivilegedExecutionResponseSerializer();
}

class _$PrivilegedExecutionResponseSerializer implements PrimitiveSerializer<PrivilegedExecutionResponse> {
  @override
  final Iterable<Type> types = const [PrivilegedExecutionResponse, _$PrivilegedExecutionResponse];

  @override
  final String wireName = r'PrivilegedExecutionResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    PrivilegedExecutionResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'enabled';
    yield serializers.serialize(
      object.enabled,
      specifiedType: const FullType(bool),
    );
    yield r'node_id';
    yield serializers.serialize(
      object.nodeId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    PrivilegedExecutionResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required PrivilegedExecutionResponseBuilder result,
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
        case r'node_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.nodeId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  PrivilegedExecutionResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = PrivilegedExecutionResponseBuilder();
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
