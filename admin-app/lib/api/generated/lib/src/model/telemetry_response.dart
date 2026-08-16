//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:deploy_go_api_client/src/model/latest_telemetry.dart';
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/history_point.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'telemetry_response.g.dart';

/// TelemetryResponse
///
/// Properties:
/// * [capability]
/// * [capabilityReason]
/// * [capturedAt]
/// * [connectivity]
/// * [freshness]
/// * [history]
/// * [latest]
/// * [nodeId]
/// * [receivedAt]
@BuiltValue()
abstract class TelemetryResponse implements Built<TelemetryResponse, TelemetryResponseBuilder> {
  @BuiltValueField(wireName: r'capability')
  String get capability;

  @BuiltValueField(wireName: r'capability_reason')
  String? get capabilityReason;

  @BuiltValueField(wireName: r'captured_at')
  String? get capturedAt;

  @BuiltValueField(wireName: r'connectivity')
  String get connectivity;

  @BuiltValueField(wireName: r'freshness')
  String get freshness;

  @BuiltValueField(wireName: r'history')
  BuiltList<HistoryPoint> get history;

  @BuiltValueField(wireName: r'latest')
  LatestTelemetry? get latest;

  @BuiltValueField(wireName: r'node_id')
  String get nodeId;

  @BuiltValueField(wireName: r'received_at')
  String? get receivedAt;

  TelemetryResponse._();

  factory TelemetryResponse([void updates(TelemetryResponseBuilder b)]) = _$TelemetryResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(TelemetryResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<TelemetryResponse> get serializer => _$TelemetryResponseSerializer();
}

class _$TelemetryResponseSerializer implements PrimitiveSerializer<TelemetryResponse> {
  @override
  final Iterable<Type> types = const [TelemetryResponse, _$TelemetryResponse];

  @override
  final String wireName = r'TelemetryResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    TelemetryResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'capability';
    yield serializers.serialize(
      object.capability,
      specifiedType: const FullType(String),
    );
    if (object.capabilityReason != null) {
      yield r'capability_reason';
      yield serializers.serialize(
        object.capabilityReason,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.capturedAt != null) {
      yield r'captured_at';
      yield serializers.serialize(
        object.capturedAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'connectivity';
    yield serializers.serialize(
      object.connectivity,
      specifiedType: const FullType(String),
    );
    yield r'freshness';
    yield serializers.serialize(
      object.freshness,
      specifiedType: const FullType(String),
    );
    yield r'history';
    yield serializers.serialize(
      object.history,
      specifiedType: const FullType(BuiltList, [FullType(HistoryPoint)]),
    );
    if (object.latest != null) {
      yield r'latest';
      yield serializers.serialize(
        object.latest,
        specifiedType: const FullType.nullable(LatestTelemetry),
      );
    }
    yield r'node_id';
    yield serializers.serialize(
      object.nodeId,
      specifiedType: const FullType(String),
    );
    if (object.receivedAt != null) {
      yield r'received_at';
      yield serializers.serialize(
        object.receivedAt,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    TelemetryResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required TelemetryResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'capability':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.capability = valueDes;
          break;
        case r'capability_reason':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.capabilityReason = valueDes;
          break;
        case r'captured_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.capturedAt = valueDes;
          break;
        case r'connectivity':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.connectivity = valueDes;
          break;
        case r'freshness':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.freshness = valueDes;
          break;
        case r'history':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(HistoryPoint)]),
          ) as BuiltList<HistoryPoint>;
          result.history.replace(valueDes);
          break;
        case r'latest':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(LatestTelemetry),
          ) as LatestTelemetry?;
          if (valueDes == null) continue;
          result.latest.replace(valueDes);
          break;
        case r'node_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.nodeId = valueDes;
          break;
        case r'received_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.receivedAt = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  TelemetryResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = TelemetryResponseBuilder();
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
