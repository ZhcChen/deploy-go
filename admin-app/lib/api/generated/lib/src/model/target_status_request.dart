//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'target_status_request.g.dart';

/// TargetStatusRequest
///
/// Properties:
/// * [status]
/// * [version]
@BuiltValue()
abstract class TargetStatusRequest implements Built<TargetStatusRequest, TargetStatusRequestBuilder> {
  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'version')
  int get version;

  TargetStatusRequest._();

  factory TargetStatusRequest([void updates(TargetStatusRequestBuilder b)]) = _$TargetStatusRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(TargetStatusRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<TargetStatusRequest> get serializer => _$TargetStatusRequestSerializer();
}

class _$TargetStatusRequestSerializer implements PrimitiveSerializer<TargetStatusRequest> {
  @override
  final Iterable<Type> types = const [TargetStatusRequest, _$TargetStatusRequest];

  @override
  final String wireName = r'TargetStatusRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    TargetStatusRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'status';
    yield serializers.serialize(
      object.status,
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
    TargetStatusRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required TargetStatusRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
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
  TargetStatusRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = TargetStatusRequestBuilder();
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
