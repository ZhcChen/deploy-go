//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'config_diff_query.g.dart';

/// ConfigDiffQuery
///
/// Properties:
/// * [version]
@BuiltValue()
abstract class ConfigDiffQuery implements Built<ConfigDiffQuery, ConfigDiffQueryBuilder> {
  @BuiltValueField(wireName: r'version')
  int? get version;

  ConfigDiffQuery._();

  factory ConfigDiffQuery([void updates(ConfigDiffQueryBuilder b)]) = _$ConfigDiffQuery;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ConfigDiffQueryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ConfigDiffQuery> get serializer => _$ConfigDiffQuerySerializer();
}

class _$ConfigDiffQuerySerializer implements PrimitiveSerializer<ConfigDiffQuery> {
  @override
  final Iterable<Type> types = const [ConfigDiffQuery, _$ConfigDiffQuery];

  @override
  final String wireName = r'ConfigDiffQuery';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ConfigDiffQuery object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.version != null) {
      yield r'version';
      yield serializers.serialize(
        object.version,
        specifiedType: const FullType.nullable(int),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    ConfigDiffQuery object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ConfigDiffQueryBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
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
  ConfigDiffQuery deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ConfigDiffQueryBuilder();
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
