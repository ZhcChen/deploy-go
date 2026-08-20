//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'delete_platform_configuration_center_request.g.dart';

/// DeletePlatformConfigurationCenterRequest
///
/// Properties:
/// * [version]
@BuiltValue()
abstract class DeletePlatformConfigurationCenterRequest implements Built<DeletePlatformConfigurationCenterRequest, DeletePlatformConfigurationCenterRequestBuilder> {
  @BuiltValueField(wireName: r'version')
  int get version;

  DeletePlatformConfigurationCenterRequest._();

  factory DeletePlatformConfigurationCenterRequest([void updates(DeletePlatformConfigurationCenterRequestBuilder b)]) = _$DeletePlatformConfigurationCenterRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(DeletePlatformConfigurationCenterRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<DeletePlatformConfigurationCenterRequest> get serializer => _$DeletePlatformConfigurationCenterRequestSerializer();
}

class _$DeletePlatformConfigurationCenterRequestSerializer implements PrimitiveSerializer<DeletePlatformConfigurationCenterRequest> {
  @override
  final Iterable<Type> types = const [DeletePlatformConfigurationCenterRequest, _$DeletePlatformConfigurationCenterRequest];

  @override
  final String wireName = r'DeletePlatformConfigurationCenterRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    DeletePlatformConfigurationCenterRequest object, {
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
    DeletePlatformConfigurationCenterRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required DeletePlatformConfigurationCenterRequestBuilder result,
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
  DeletePlatformConfigurationCenterRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = DeletePlatformConfigurationCenterRequestBuilder();
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
