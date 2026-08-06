//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_env_file_response.g.dart';

/// ApplicationEnvFileResponse
///
/// Properties:
/// * [applicationId]
/// * [currentDigest]
/// * [currentVersion]
/// * [declaredAt]
/// * [failedCount]
/// * [fileName]
/// * [format]
/// * [id]
/// * [module]
/// * [pendingCount]
/// * [succeededCount]
/// * [syncingCount]
/// * [targetCount]
/// * [updatedAt]
/// * [version]
@BuiltValue()
abstract class ApplicationEnvFileResponse implements Built<ApplicationEnvFileResponse, ApplicationEnvFileResponseBuilder> {
  @BuiltValueField(wireName: r'application_id')
  String get applicationId;

  @BuiltValueField(wireName: r'current_digest')
  String get currentDigest;

  @BuiltValueField(wireName: r'current_version')
  int get currentVersion;

  @BuiltValueField(wireName: r'declared_at')
  String get declaredAt;

  @BuiltValueField(wireName: r'failed_count')
  int get failedCount;

  @BuiltValueField(wireName: r'file_name')
  String get fileName;

  @BuiltValueField(wireName: r'format')
  String get format;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'module')
  String get module;

  @BuiltValueField(wireName: r'pending_count')
  int get pendingCount;

  @BuiltValueField(wireName: r'succeeded_count')
  int get succeededCount;

  @BuiltValueField(wireName: r'syncing_count')
  int get syncingCount;

  @BuiltValueField(wireName: r'target_count')
  int get targetCount;

  @BuiltValueField(wireName: r'updated_at')
  String get updatedAt;

  @BuiltValueField(wireName: r'version')
  int get version;

  ApplicationEnvFileResponse._();

  factory ApplicationEnvFileResponse([void updates(ApplicationEnvFileResponseBuilder b)]) = _$ApplicationEnvFileResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationEnvFileResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationEnvFileResponse> get serializer => _$ApplicationEnvFileResponseSerializer();
}

class _$ApplicationEnvFileResponseSerializer implements PrimitiveSerializer<ApplicationEnvFileResponse> {
  @override
  final Iterable<Type> types = const [ApplicationEnvFileResponse, _$ApplicationEnvFileResponse];

  @override
  final String wireName = r'ApplicationEnvFileResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationEnvFileResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'application_id';
    yield serializers.serialize(
      object.applicationId,
      specifiedType: const FullType(String),
    );
    yield r'current_digest';
    yield serializers.serialize(
      object.currentDigest,
      specifiedType: const FullType(String),
    );
    yield r'current_version';
    yield serializers.serialize(
      object.currentVersion,
      specifiedType: const FullType(int),
    );
    yield r'declared_at';
    yield serializers.serialize(
      object.declaredAt,
      specifiedType: const FullType(String),
    );
    yield r'failed_count';
    yield serializers.serialize(
      object.failedCount,
      specifiedType: const FullType(int),
    );
    yield r'file_name';
    yield serializers.serialize(
      object.fileName,
      specifiedType: const FullType(String),
    );
    yield r'format';
    yield serializers.serialize(
      object.format,
      specifiedType: const FullType(String),
    );
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'module';
    yield serializers.serialize(
      object.module,
      specifiedType: const FullType(String),
    );
    yield r'pending_count';
    yield serializers.serialize(
      object.pendingCount,
      specifiedType: const FullType(int),
    );
    yield r'succeeded_count';
    yield serializers.serialize(
      object.succeededCount,
      specifiedType: const FullType(int),
    );
    yield r'syncing_count';
    yield serializers.serialize(
      object.syncingCount,
      specifiedType: const FullType(int),
    );
    yield r'target_count';
    yield serializers.serialize(
      object.targetCount,
      specifiedType: const FullType(int),
    );
    yield r'updated_at';
    yield serializers.serialize(
      object.updatedAt,
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
    ApplicationEnvFileResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationEnvFileResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'application_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.applicationId = valueDes;
          break;
        case r'current_digest':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.currentDigest = valueDes;
          break;
        case r'current_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.currentVersion = valueDes;
          break;
        case r'declared_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.declaredAt = valueDes;
          break;
        case r'failed_count':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.failedCount = valueDes;
          break;
        case r'file_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.fileName = valueDes;
          break;
        case r'format':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.format = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'module':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.module = valueDes;
          break;
        case r'pending_count':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.pendingCount = valueDes;
          break;
        case r'succeeded_count':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.succeededCount = valueDes;
          break;
        case r'syncing_count':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.syncingCount = valueDes;
          break;
        case r'target_count':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.targetCount = valueDes;
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.updatedAt = valueDes;
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
  ApplicationEnvFileResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationEnvFileResponseBuilder();
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
