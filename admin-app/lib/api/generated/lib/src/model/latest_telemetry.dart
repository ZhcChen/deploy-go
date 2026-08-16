//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:deploy_go_api_client/src/model/metric_value.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'latest_telemetry.g.dart';

/// LatestTelemetry
///
/// Properties:
/// * [cpuUsageRatio]
/// * [diskBusyRatio]
/// * [diskReadBytesPerSecond]
/// * [diskWriteBytesPerSecond]
/// * [gpuReason]
/// * [gpuStatus]
/// * [gpus]
/// * [memoryTotalBytes]
/// * [memoryUsedBytes]
/// * [networkReceiveBytesPerSecond]
/// * [networkTransmitBytesPerSecond]
/// * [workRootTotalBytes]
/// * [workRootUsedBytes]
@BuiltValue()
abstract class LatestTelemetry implements Built<LatestTelemetry, LatestTelemetryBuilder> {
  @BuiltValueField(wireName: r'cpu_usage_ratio')
  MetricValue get cpuUsageRatio;

  @BuiltValueField(wireName: r'disk_busy_ratio')
  MetricValue get diskBusyRatio;

  @BuiltValueField(wireName: r'disk_read_bytes_per_second')
  MetricValue get diskReadBytesPerSecond;

  @BuiltValueField(wireName: r'disk_write_bytes_per_second')
  MetricValue get diskWriteBytesPerSecond;

  @BuiltValueField(wireName: r'gpu_reason')
  String? get gpuReason;

  @BuiltValueField(wireName: r'gpu_status')
  String get gpuStatus;

  @BuiltValueField(wireName: r'gpus')
  JsonObject? get gpus;

  @BuiltValueField(wireName: r'memory_total_bytes')
  MetricValue get memoryTotalBytes;

  @BuiltValueField(wireName: r'memory_used_bytes')
  MetricValue get memoryUsedBytes;

  @BuiltValueField(wireName: r'network_receive_bytes_per_second')
  MetricValue get networkReceiveBytesPerSecond;

  @BuiltValueField(wireName: r'network_transmit_bytes_per_second')
  MetricValue get networkTransmitBytesPerSecond;

  @BuiltValueField(wireName: r'work_root_total_bytes')
  MetricValue get workRootTotalBytes;

  @BuiltValueField(wireName: r'work_root_used_bytes')
  MetricValue get workRootUsedBytes;

  LatestTelemetry._();

  factory LatestTelemetry([void updates(LatestTelemetryBuilder b)]) = _$LatestTelemetry;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(LatestTelemetryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<LatestTelemetry> get serializer => _$LatestTelemetrySerializer();
}

class _$LatestTelemetrySerializer implements PrimitiveSerializer<LatestTelemetry> {
  @override
  final Iterable<Type> types = const [LatestTelemetry, _$LatestTelemetry];

  @override
  final String wireName = r'LatestTelemetry';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    LatestTelemetry object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'cpu_usage_ratio';
    yield serializers.serialize(
      object.cpuUsageRatio,
      specifiedType: const FullType(MetricValue),
    );
    yield r'disk_busy_ratio';
    yield serializers.serialize(
      object.diskBusyRatio,
      specifiedType: const FullType(MetricValue),
    );
    yield r'disk_read_bytes_per_second';
    yield serializers.serialize(
      object.diskReadBytesPerSecond,
      specifiedType: const FullType(MetricValue),
    );
    yield r'disk_write_bytes_per_second';
    yield serializers.serialize(
      object.diskWriteBytesPerSecond,
      specifiedType: const FullType(MetricValue),
    );
    if (object.gpuReason != null) {
      yield r'gpu_reason';
      yield serializers.serialize(
        object.gpuReason,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'gpu_status';
    yield serializers.serialize(
      object.gpuStatus,
      specifiedType: const FullType(String),
    );
    yield r'gpus';
    yield object.gpus == null ? null : serializers.serialize(
      object.gpus,
      specifiedType: const FullType.nullable(JsonObject),
    );
    yield r'memory_total_bytes';
    yield serializers.serialize(
      object.memoryTotalBytes,
      specifiedType: const FullType(MetricValue),
    );
    yield r'memory_used_bytes';
    yield serializers.serialize(
      object.memoryUsedBytes,
      specifiedType: const FullType(MetricValue),
    );
    yield r'network_receive_bytes_per_second';
    yield serializers.serialize(
      object.networkReceiveBytesPerSecond,
      specifiedType: const FullType(MetricValue),
    );
    yield r'network_transmit_bytes_per_second';
    yield serializers.serialize(
      object.networkTransmitBytesPerSecond,
      specifiedType: const FullType(MetricValue),
    );
    yield r'work_root_total_bytes';
    yield serializers.serialize(
      object.workRootTotalBytes,
      specifiedType: const FullType(MetricValue),
    );
    yield r'work_root_used_bytes';
    yield serializers.serialize(
      object.workRootUsedBytes,
      specifiedType: const FullType(MetricValue),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    LatestTelemetry object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required LatestTelemetryBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'cpu_usage_ratio':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(MetricValue),
          ) as MetricValue;
          result.cpuUsageRatio.replace(valueDes);
          break;
        case r'disk_busy_ratio':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(MetricValue),
          ) as MetricValue;
          result.diskBusyRatio.replace(valueDes);
          break;
        case r'disk_read_bytes_per_second':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(MetricValue),
          ) as MetricValue;
          result.diskReadBytesPerSecond.replace(valueDes);
          break;
        case r'disk_write_bytes_per_second':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(MetricValue),
          ) as MetricValue;
          result.diskWriteBytesPerSecond.replace(valueDes);
          break;
        case r'gpu_reason':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.gpuReason = valueDes;
          break;
        case r'gpu_status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.gpuStatus = valueDes;
          break;
        case r'gpus':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.gpus = valueDes;
          break;
        case r'memory_total_bytes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(MetricValue),
          ) as MetricValue;
          result.memoryTotalBytes.replace(valueDes);
          break;
        case r'memory_used_bytes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(MetricValue),
          ) as MetricValue;
          result.memoryUsedBytes.replace(valueDes);
          break;
        case r'network_receive_bytes_per_second':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(MetricValue),
          ) as MetricValue;
          result.networkReceiveBytesPerSecond.replace(valueDes);
          break;
        case r'network_transmit_bytes_per_second':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(MetricValue),
          ) as MetricValue;
          result.networkTransmitBytesPerSecond.replace(valueDes);
          break;
        case r'work_root_total_bytes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(MetricValue),
          ) as MetricValue;
          result.workRootTotalBytes.replace(valueDes);
          break;
        case r'work_root_used_bytes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(MetricValue),
          ) as MetricValue;
          result.workRootUsedBytes.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  LatestTelemetry deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = LatestTelemetryBuilder();
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
