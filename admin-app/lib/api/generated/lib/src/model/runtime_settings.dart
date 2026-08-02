//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'runtime_settings.g.dart';

/// RuntimeSettings
///
/// Properties:
/// * [logRetentionDays]
/// * [maxConcurrentDeployments]
/// * [maxLogBytes]
/// * [version]
@BuiltValue()
abstract class RuntimeSettings implements Built<RuntimeSettings, RuntimeSettingsBuilder> {
  @BuiltValueField(wireName: r'log_retention_days')
  int get logRetentionDays;

  @BuiltValueField(wireName: r'max_concurrent_deployments')
  int get maxConcurrentDeployments;

  @BuiltValueField(wireName: r'max_log_bytes')
  int get maxLogBytes;

  @BuiltValueField(wireName: r'version')
  int get version;

  RuntimeSettings._();

  factory RuntimeSettings([void updates(RuntimeSettingsBuilder b)]) = _$RuntimeSettings;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RuntimeSettingsBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RuntimeSettings> get serializer => _$RuntimeSettingsSerializer();
}

class _$RuntimeSettingsSerializer implements PrimitiveSerializer<RuntimeSettings> {
  @override
  final Iterable<Type> types = const [RuntimeSettings, _$RuntimeSettings];

  @override
  final String wireName = r'RuntimeSettings';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RuntimeSettings object, {
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
    yield r'version';
    yield serializers.serialize(
      object.version,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RuntimeSettings object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required RuntimeSettingsBuilder result,
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
  RuntimeSettings deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RuntimeSettingsBuilder();
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
