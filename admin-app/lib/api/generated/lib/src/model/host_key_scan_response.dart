//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'host_key_scan_response.g.dart';

/// HostKeyScanResponse
///
/// Properties:
/// * [checkId]
/// * [fingerprint]
/// * [snapshotHash]
@BuiltValue()
abstract class HostKeyScanResponse implements Built<HostKeyScanResponse, HostKeyScanResponseBuilder> {
  @BuiltValueField(wireName: r'check_id')
  String get checkId;

  @BuiltValueField(wireName: r'fingerprint')
  String get fingerprint;

  @BuiltValueField(wireName: r'snapshot_hash')
  String get snapshotHash;

  HostKeyScanResponse._();

  factory HostKeyScanResponse([void updates(HostKeyScanResponseBuilder b)]) = _$HostKeyScanResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(HostKeyScanResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<HostKeyScanResponse> get serializer => _$HostKeyScanResponseSerializer();
}

class _$HostKeyScanResponseSerializer implements PrimitiveSerializer<HostKeyScanResponse> {
  @override
  final Iterable<Type> types = const [HostKeyScanResponse, _$HostKeyScanResponse];

  @override
  final String wireName = r'HostKeyScanResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    HostKeyScanResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'check_id';
    yield serializers.serialize(
      object.checkId,
      specifiedType: const FullType(String),
    );
    yield r'fingerprint';
    yield serializers.serialize(
      object.fingerprint,
      specifiedType: const FullType(String),
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
    HostKeyScanResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required HostKeyScanResponseBuilder result,
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
        case r'fingerprint':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.fingerprint = valueDes;
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
  HostKeyScanResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = HostKeyScanResponseBuilder();
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
