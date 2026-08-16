//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'metric_value.g.dart';

/// MetricValue
///
/// Properties:
/// * [status]
/// * [value]
@BuiltValue()
abstract class MetricValue implements Built<MetricValue, MetricValueBuilder> {
  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'value')
  double? get value;

  MetricValue._();

  factory MetricValue([void updates(MetricValueBuilder b)]) = _$MetricValue;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(MetricValueBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<MetricValue> get serializer => _$MetricValueSerializer();
}

class _$MetricValueSerializer implements PrimitiveSerializer<MetricValue> {
  @override
  final Iterable<Type> types = const [MetricValue, _$MetricValue];

  @override
  final String wireName = r'MetricValue';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    MetricValue object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
    if (object.value != null) {
      yield r'value';
      yield serializers.serialize(
        object.value,
        specifiedType: const FullType.nullable(double),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    MetricValue object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required MetricValueBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        case r'value':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(double),
          ) as double?;
          if (valueDes == null) continue;
          result.value = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  MetricValue deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = MetricValueBuilder();
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
