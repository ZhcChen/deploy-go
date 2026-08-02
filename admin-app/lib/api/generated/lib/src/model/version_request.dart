//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'version_request.g.dart';

/// VersionRequest
///
/// Properties:
/// * [version]
@BuiltValue()
abstract class VersionRequest implements Built<VersionRequest, VersionRequestBuilder> {
  @BuiltValueField(wireName: r'version')
  int get version;

  VersionRequest._();

  factory VersionRequest([void updates(VersionRequestBuilder b)]) = _$VersionRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(VersionRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<VersionRequest> get serializer => _$VersionRequestSerializer();
}

class _$VersionRequestSerializer implements PrimitiveSerializer<VersionRequest> {
  @override
  final Iterable<Type> types = const [VersionRequest, _$VersionRequest];

  @override
  final String wireName = r'VersionRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    VersionRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'version';
    yield serializers.serialize(
      object.version,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    VersionRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required VersionRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
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
  VersionRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = VersionRequestBuilder();
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
