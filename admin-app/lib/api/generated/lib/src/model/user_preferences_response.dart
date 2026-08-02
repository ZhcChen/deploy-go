//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'user_preferences_response.g.dart';

/// UserPreferencesResponse
///
/// Properties:
/// * [followLogs]
/// * [notifyDeploymentCompleted]
/// * [notifyDeploymentFailed]
/// * [notifyNodeUnhealthy]
/// * [timeFormat]
/// * [version]
@BuiltValue()
abstract class UserPreferencesResponse implements Built<UserPreferencesResponse, UserPreferencesResponseBuilder> {
  @BuiltValueField(wireName: r'follow_logs')
  bool get followLogs;

  @BuiltValueField(wireName: r'notify_deployment_completed')
  bool get notifyDeploymentCompleted;

  @BuiltValueField(wireName: r'notify_deployment_failed')
  bool get notifyDeploymentFailed;

  @BuiltValueField(wireName: r'notify_node_unhealthy')
  bool get notifyNodeUnhealthy;

  @BuiltValueField(wireName: r'time_format')
  String get timeFormat;

  @BuiltValueField(wireName: r'version')
  int get version;

  UserPreferencesResponse._();

  factory UserPreferencesResponse([void updates(UserPreferencesResponseBuilder b)]) = _$UserPreferencesResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UserPreferencesResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UserPreferencesResponse> get serializer => _$UserPreferencesResponseSerializer();
}

class _$UserPreferencesResponseSerializer implements PrimitiveSerializer<UserPreferencesResponse> {
  @override
  final Iterable<Type> types = const [UserPreferencesResponse, _$UserPreferencesResponse];

  @override
  final String wireName = r'UserPreferencesResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UserPreferencesResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'follow_logs';
    yield serializers.serialize(
      object.followLogs,
      specifiedType: const FullType(bool),
    );
    yield r'notify_deployment_completed';
    yield serializers.serialize(
      object.notifyDeploymentCompleted,
      specifiedType: const FullType(bool),
    );
    yield r'notify_deployment_failed';
    yield serializers.serialize(
      object.notifyDeploymentFailed,
      specifiedType: const FullType(bool),
    );
    yield r'notify_node_unhealthy';
    yield serializers.serialize(
      object.notifyNodeUnhealthy,
      specifiedType: const FullType(bool),
    );
    yield r'time_format';
    yield serializers.serialize(
      object.timeFormat,
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
    UserPreferencesResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required UserPreferencesResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'follow_logs':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.followLogs = valueDes;
          break;
        case r'notify_deployment_completed':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.notifyDeploymentCompleted = valueDes;
          break;
        case r'notify_deployment_failed':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.notifyDeploymentFailed = valueDes;
          break;
        case r'notify_node_unhealthy':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.notifyNodeUnhealthy = valueDes;
          break;
        case r'time_format':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.timeFormat = valueDes;
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
  UserPreferencesResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UserPreferencesResponseBuilder();
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
