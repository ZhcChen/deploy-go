//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'platform_configuration_center_response.g.dart';

/// PlatformConfigurationCenterResponse
///
/// Properties:
/// * [checkedAt]
/// * [endpoints]
/// * [passwordConfigured]
/// * [provider]
/// * [status]
/// * [updatedAt]
/// * [username]
/// * [version]
@BuiltValue()
abstract class PlatformConfigurationCenterResponse implements Built<PlatformConfigurationCenterResponse, PlatformConfigurationCenterResponseBuilder> {
  @BuiltValueField(wireName: r'checked_at')
  String? get checkedAt;

  @BuiltValueField(wireName: r'endpoints')
  BuiltList<String> get endpoints;

  @BuiltValueField(wireName: r'password_configured')
  bool get passwordConfigured;

  @BuiltValueField(wireName: r'provider')
  String get provider;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'updated_at')
  String get updatedAt;

  @BuiltValueField(wireName: r'username')
  String get username;

  @BuiltValueField(wireName: r'version')
  int get version;

  PlatformConfigurationCenterResponse._();

  factory PlatformConfigurationCenterResponse([void updates(PlatformConfigurationCenterResponseBuilder b)]) = _$PlatformConfigurationCenterResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(PlatformConfigurationCenterResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<PlatformConfigurationCenterResponse> get serializer => _$PlatformConfigurationCenterResponseSerializer();
}

class _$PlatformConfigurationCenterResponseSerializer implements PrimitiveSerializer<PlatformConfigurationCenterResponse> {
  @override
  final Iterable<Type> types = const [PlatformConfigurationCenterResponse, _$PlatformConfigurationCenterResponse];

  @override
  final String wireName = r'PlatformConfigurationCenterResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    PlatformConfigurationCenterResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.checkedAt != null) {
      yield r'checked_at';
      yield serializers.serialize(
        object.checkedAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'endpoints';
    yield serializers.serialize(
      object.endpoints,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    yield r'password_configured';
    yield serializers.serialize(
      object.passwordConfigured,
      specifiedType: const FullType(bool),
    );
    yield r'provider';
    yield serializers.serialize(
      object.provider,
      specifiedType: const FullType(String),
    );
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
    yield r'updated_at';
    yield serializers.serialize(
      object.updatedAt,
      specifiedType: const FullType(String),
    );
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
    PlatformConfigurationCenterResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required PlatformConfigurationCenterResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'checked_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.checkedAt = valueDes;
          break;
        case r'endpoints':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.endpoints.replace(valueDes);
          break;
        case r'password_configured':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.passwordConfigured = valueDes;
          break;
        case r'provider':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.provider = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.updatedAt = valueDes;
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
  PlatformConfigurationCenterResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = PlatformConfigurationCenterResponseBuilder();
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
