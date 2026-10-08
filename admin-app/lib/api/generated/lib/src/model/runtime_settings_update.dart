//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'runtime_settings_update.g.dart';

/// RuntimeSettingsUpdate
///
/// Properties:
/// * [logRetentionDays]
/// * [maxConcurrentDeployments]
/// * [maxLogBytes]
/// * [maxTotalLogBytes]
/// * [version]
@BuiltValue()
abstract class RuntimeSettingsUpdate implements Built<RuntimeSettingsUpdate, RuntimeSettingsUpdateBuilder> {
  @BuiltValueField(wireName: r'log_retention_days')
  int get logRetentionDays;

  @BuiltValueField(wireName: r'max_concurrent_deployments')
  int get maxConcurrentDeployments;

  @BuiltValueField(wireName: r'max_log_bytes')
  int get maxLogBytes;

  @BuiltValueField(wireName: r'max_total_log_bytes')
  int? get maxTotalLogBytes;

  @BuiltValueField(wireName: r'version')
  int get version;

  RuntimeSettingsUpdate._();

  factory RuntimeSettingsUpdate([void updates(RuntimeSettingsUpdateBuilder b)]) = _$RuntimeSettingsUpdate;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RuntimeSettingsUpdateBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RuntimeSettingsUpdate> get serializer => _$RuntimeSettingsUpdateSerializer();
}

class _$RuntimeSettingsUpdateSerializer implements PrimitiveSerializer<RuntimeSettingsUpdate> {
  @override
  final Iterable<Type> types = const [RuntimeSettingsUpdate, _$RuntimeSettingsUpdate];

  @override
  final String wireName = r'RuntimeSettingsUpdate';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RuntimeSettingsUpdate object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'log_retention_days';
    yield serializers.serialize(
      object.logRetentionDays,
      specifiedType: const FullType(int),
    );
    yield r'max_concurrent_deployments';
    yield serializers.serialize(
      object.maxConcurrentDeployments,
      specifiedType: const FullType(int),
    );
    yield r'max_log_bytes';
    yield serializers.serialize(
      object.maxLogBytes,
      specifiedType: const FullType(int),
    );
    if (object.maxTotalLogBytes != null) {
      yield r'max_total_log_bytes';
      yield serializers.serialize(
        object.maxTotalLogBytes,
        specifiedType: const FullType.nullable(int),
      );
    }
    yield r'version';
    yield serializers.serialize(
      object.version,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RuntimeSettingsUpdate object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required RuntimeSettingsUpdateBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'log_retention_days':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.logRetentionDays = valueDes;
          break;
        case r'max_concurrent_deployments':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.maxConcurrentDeployments = valueDes;
          break;
        case r'max_log_bytes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.maxLogBytes = valueDes;
          break;
        case r'max_total_log_bytes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.maxTotalLogBytes = valueDes;
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
  RuntimeSettingsUpdate deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RuntimeSettingsUpdateBuilder();
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
