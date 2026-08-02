//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'confirm_host_key_request.g.dart';

/// ConfirmHostKeyRequest
///
/// Properties:
/// * [checkId]
/// * [snapshotHash]
/// * [version]
@BuiltValue()
abstract class ConfirmHostKeyRequest implements Built<ConfirmHostKeyRequest, ConfirmHostKeyRequestBuilder> {
  @BuiltValueField(wireName: r'check_id')
  String get checkId;

  @BuiltValueField(wireName: r'snapshot_hash')
  String get snapshotHash;

  @BuiltValueField(wireName: r'version')
  int get version;

  ConfirmHostKeyRequest._();

  factory ConfirmHostKeyRequest([void updates(ConfirmHostKeyRequestBuilder b)]) = _$ConfirmHostKeyRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ConfirmHostKeyRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ConfirmHostKeyRequest> get serializer => _$ConfirmHostKeyRequestSerializer();
}

class _$ConfirmHostKeyRequestSerializer implements PrimitiveSerializer<ConfirmHostKeyRequest> {
  @override
  final Iterable<Type> types = const [ConfirmHostKeyRequest, _$ConfirmHostKeyRequest];

  @override
  final String wireName = r'ConfirmHostKeyRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ConfirmHostKeyRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'check_id';
    yield serializers.serialize(
      object.checkId,
      specifiedType: const FullType(String),
    );
    yield r'snapshot_hash';
    yield serializers.serialize(
      object.snapshotHash,
      specifiedType: const FullType(String),
    );
    yield r'version';
    yield serializers.serialize(
      object.version,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ConfirmHostKeyRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ConfirmHostKeyRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'check_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.checkId = valueDes;
          break;
        case r'snapshot_hash':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.snapshotHash = valueDes;
          break;
        case r'version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.version = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ConfirmHostKeyRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ConfirmHostKeyRequestBuilder();
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
