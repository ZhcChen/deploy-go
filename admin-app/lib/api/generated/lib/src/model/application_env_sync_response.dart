//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_env_sync_response.g.dart';

/// ApplicationEnvSyncResponse
///
/// Properties:
/// * [actualVersion]
/// * [errorCode]
/// * [errorMessage]
/// * [lastAttemptAt]
/// * [nodeId]
/// * [nodeName]
/// * [status]
/// * [syncedAt]
/// * [targetId]
@BuiltValue()
abstract class ApplicationEnvSyncResponse implements Built<ApplicationEnvSyncResponse, ApplicationEnvSyncResponseBuilder> {
  @BuiltValueField(wireName: r'actual_version')
  int? get actualVersion;

  @BuiltValueField(wireName: r'error_code')
  String? get errorCode;

  @BuiltValueField(wireName: r'error_message')
  String? get errorMessage;

  @BuiltValueField(wireName: r'last_attempt_at')
  String? get lastAttemptAt;

  @BuiltValueField(wireName: r'node_id')
  String get nodeId;

  @BuiltValueField(wireName: r'node_name')
  String get nodeName;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'synced_at')
  String? get syncedAt;

  @BuiltValueField(wireName: r'target_id')
  String get targetId;

  ApplicationEnvSyncResponse._();

  factory ApplicationEnvSyncResponse([void updates(ApplicationEnvSyncResponseBuilder b)]) = _$ApplicationEnvSyncResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationEnvSyncResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationEnvSyncResponse> get serializer => _$ApplicationEnvSyncResponseSerializer();
}

class _$ApplicationEnvSyncResponseSerializer implements PrimitiveSerializer<ApplicationEnvSyncResponse> {
  @override
  final Iterable<Type> types = const [ApplicationEnvSyncResponse, _$ApplicationEnvSyncResponse];

  @override
  final String wireName = r'ApplicationEnvSyncResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationEnvSyncResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.actualVersion != null) {
      yield r'actual_version';
      yield serializers.serialize(
        object.actualVersion,
        specifiedType: const FullType.nullable(int),
      );
    }
    if (object.errorCode != null) {
      yield r'error_code';
      yield serializers.serialize(
        object.errorCode,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.errorMessage != null) {
      yield r'error_message';
      yield serializers.serialize(
        object.errorMessage,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.lastAttemptAt != null) {
      yield r'last_attempt_at';
      yield serializers.serialize(
        object.lastAttemptAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'node_id';
    yield serializers.serialize(
      object.nodeId,
      specifiedType: const FullType(String),
    );
    yield r'node_name';
    yield serializers.serialize(
      object.nodeName,
      specifiedType: const FullType(String),
    );
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
    if (object.syncedAt != null) {
      yield r'synced_at';
      yield serializers.serialize(
        object.syncedAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'target_id';
    yield serializers.serialize(
      object.targetId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ApplicationEnvSyncResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationEnvSyncResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'actual_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.actualVersion = valueDes;
          break;
        case r'error_code':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.errorCode = valueDes;
          break;
        case r'error_message':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.errorMessage = valueDes;
          break;
        case r'last_attempt_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.lastAttemptAt = valueDes;
          break;
        case r'node_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.nodeId = valueDes;
          break;
        case r'node_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.nodeName = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        case r'synced_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.syncedAt = valueDes;
          break;
        case r'target_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.targetId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ApplicationEnvSyncResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationEnvSyncResponseBuilder();
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
