//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'history_point.g.dart';

/// HistoryPoint
///
/// Properties:
/// * [cpuUsageRatio]
/// * [diskBusyRatio]
/// * [diskReadBytesPerSecond]
/// * [diskWriteBytesPerSecond]
/// * [memoryUsedBytes]
/// * [networkReceiveBytesPerSecond]
/// * [networkTransmitBytesPerSecond]
/// * [receivedAt]
/// * [workRootUsedBytes]
@BuiltValue()
abstract class HistoryPoint implements Built<HistoryPoint, HistoryPointBuilder> {
  @BuiltValueField(wireName: r'cpu_usage_ratio')
  double? get cpuUsageRatio;

  @BuiltValueField(wireName: r'disk_busy_ratio')
  double? get diskBusyRatio;

  @BuiltValueField(wireName: r'disk_read_bytes_per_second')
  double? get diskReadBytesPerSecond;

  @BuiltValueField(wireName: r'disk_write_bytes_per_second')
  double? get diskWriteBytesPerSecond;

  @BuiltValueField(wireName: r'memory_used_bytes')
  double? get memoryUsedBytes;

  @BuiltValueField(wireName: r'network_receive_bytes_per_second')
  double? get networkReceiveBytesPerSecond;

  @BuiltValueField(wireName: r'network_transmit_bytes_per_second')
  double? get networkTransmitBytesPerSecond;

  @BuiltValueField(wireName: r'received_at')
  String get receivedAt;

  @BuiltValueField(wireName: r'work_root_used_bytes')
  double? get workRootUsedBytes;

  HistoryPoint._();

  factory HistoryPoint([void updates(HistoryPointBuilder b)]) = _$HistoryPoint;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(HistoryPointBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<HistoryPoint> get serializer => _$HistoryPointSerializer();
}

class _$HistoryPointSerializer implements PrimitiveSerializer<HistoryPoint> {
  @override
  final Iterable<Type> types = const [HistoryPoint, _$HistoryPoint];

  @override
  final String wireName = r'HistoryPoint';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    HistoryPoint object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.cpuUsageRatio != null) {
      yield r'cpu_usage_ratio';
      yield serializers.serialize(
        object.cpuUsageRatio,
        specifiedType: const FullType.nullable(double),
      );
    }
    if (object.diskBusyRatio != null) {
      yield r'disk_busy_ratio';
      yield serializers.serialize(
        object.diskBusyRatio,
        specifiedType: const FullType.nullable(double),
      );
    }
    if (object.diskReadBytesPerSecond != null) {
      yield r'disk_read_bytes_per_second';
      yield serializers.serialize(
        object.diskReadBytesPerSecond,
        specifiedType: const FullType.nullable(double),
      );
    }
    if (object.diskWriteBytesPerSecond != null) {
      yield r'disk_write_bytes_per_second';
      yield serializers.serialize(
        object.diskWriteBytesPerSecond,
        specifiedType: const FullType.nullable(double),
      );
    }
    if (object.memoryUsedBytes != null) {
      yield r'memory_used_bytes';
      yield serializers.serialize(
        object.memoryUsedBytes,
        specifiedType: const FullType.nullable(double),
      );
    }
    if (object.networkReceiveBytesPerSecond != null) {
      yield r'network_receive_bytes_per_second';
      yield serializers.serialize(
        object.networkReceiveBytesPerSecond,
        specifiedType: const FullType.nullable(double),
      );
    }
    if (object.networkTransmitBytesPerSecond != null) {
      yield r'network_transmit_bytes_per_second';
      yield serializers.serialize(
        object.networkTransmitBytesPerSecond,
        specifiedType: const FullType.nullable(double),
      );
    }
    yield r'received_at';
    yield serializers.serialize(
      object.receivedAt,
      specifiedType: const FullType(String),
    );
    if (object.workRootUsedBytes != null) {
      yield r'work_root_used_bytes';
      yield serializers.serialize(
        object.workRootUsedBytes,
        specifiedType: const FullType.nullable(double),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    HistoryPoint object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required HistoryPointBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'cpu_usage_ratio':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(double),
          ) as double?;
          if (valueDes == null) continue;
          result.cpuUsageRatio = valueDes;
          break;
        case r'disk_busy_ratio':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(double),
          ) as double?;
          if (valueDes == null) continue;
          result.diskBusyRatio = valueDes;
          break;
        case r'disk_read_bytes_per_second':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(double),
          ) as double?;
          if (valueDes == null) continue;
          result.diskReadBytesPerSecond = valueDes;
          break;
        case r'disk_write_bytes_per_second':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(double),
          ) as double?;
          if (valueDes == null) continue;
          result.diskWriteBytesPerSecond = valueDes;
          break;
        case r'memory_used_bytes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(double),
          ) as double?;
          if (valueDes == null) continue;
          result.memoryUsedBytes = valueDes;
          break;
        case r'network_receive_bytes_per_second':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(double),
          ) as double?;
          if (valueDes == null) continue;
          result.networkReceiveBytesPerSecond = valueDes;
          break;
        case r'network_transmit_bytes_per_second':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(double),
          ) as double?;
          if (valueDes == null) continue;
          result.networkTransmitBytesPerSecond = valueDes;
          break;
        case r'received_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.receivedAt = valueDes;
          break;
        case r'work_root_used_bytes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(double),
          ) as double?;
          if (valueDes == null) continue;
          result.workRootUsedBytes = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  HistoryPoint deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = HistoryPointBuilder();
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
