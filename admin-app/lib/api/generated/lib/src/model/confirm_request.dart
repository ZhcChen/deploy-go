//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'confirm_request.g.dart';

/// ConfirmRequest
///
/// Properties:
/// * [parameters]
/// * [snapshotHash]
@BuiltValue()
abstract class ConfirmRequest implements Built<ConfirmRequest, ConfirmRequestBuilder> {
  @BuiltValueField(wireName: r'parameters')
  JsonObject? get parameters;

  @BuiltValueField(wireName: r'snapshot_hash')
  String get snapshotHash;

  ConfirmRequest._();

  factory ConfirmRequest([void updates(ConfirmRequestBuilder b)]) = _$ConfirmRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ConfirmRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ConfirmRequest> get serializer => _$ConfirmRequestSerializer();
}

class _$ConfirmRequestSerializer implements PrimitiveSerializer<ConfirmRequest> {
  @override
  final Iterable<Type> types = const [ConfirmRequest, _$ConfirmRequest];

  @override
  final String wireName = r'ConfirmRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ConfirmRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'parameters';
    yield object.parameters == null ? null : serializers.serialize(
      object.parameters,
      specifiedType: const FullType.nullable(JsonObject),
    );
    yield r'snapshot_hash';
    yield serializers.serialize(
      object.snapshotHash,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ConfirmRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ConfirmRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'parameters':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.parameters = valueDes;
          break;
        case r'snapshot_hash':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.snapshotHash = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ConfirmRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ConfirmRequestBuilder();
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
