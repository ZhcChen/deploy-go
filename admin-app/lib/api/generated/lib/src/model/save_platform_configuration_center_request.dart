//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'save_platform_configuration_center_request.g.dart';

/// SavePlatformConfigurationCenterRequest
///
/// Properties:
/// * [endpoints]
/// * [password]
/// * [username]
/// * [version]
@BuiltValue()
abstract class SavePlatformConfigurationCenterRequest implements Built<SavePlatformConfigurationCenterRequest, SavePlatformConfigurationCenterRequestBuilder> {
  @BuiltValueField(wireName: r'endpoints')
  BuiltList<String> get endpoints;

  @BuiltValueField(wireName: r'password')
  String? get password;

  @BuiltValueField(wireName: r'username')
  String get username;

  @BuiltValueField(wireName: r'version')
  int get version;

  SavePlatformConfigurationCenterRequest._();

  factory SavePlatformConfigurationCenterRequest([void updates(SavePlatformConfigurationCenterRequestBuilder b)]) = _$SavePlatformConfigurationCenterRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SavePlatformConfigurationCenterRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SavePlatformConfigurationCenterRequest> get serializer => _$SavePlatformConfigurationCenterRequestSerializer();
}

class _$SavePlatformConfigurationCenterRequestSerializer implements PrimitiveSerializer<SavePlatformConfigurationCenterRequest> {
  @override
  final Iterable<Type> types = const [SavePlatformConfigurationCenterRequest, _$SavePlatformConfigurationCenterRequest];

  @override
  final String wireName = r'SavePlatformConfigurationCenterRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SavePlatformConfigurationCenterRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'endpoints';
    yield serializers.serialize(
      object.endpoints,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    if (object.password != null) {
      yield r'password';
      yield serializers.serialize(
        object.password,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'username';
    yield serializers.serialize(
      object.username,
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
    SavePlatformConfigurationCenterRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required SavePlatformConfigurationCenterRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'endpoints':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.endpoints.replace(valueDes);
          break;
        case r'password':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.password = valueDes;
          break;
        case r'username':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.username = valueDes;
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
  SavePlatformConfigurationCenterRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SavePlatformConfigurationCenterRequestBuilder();
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
